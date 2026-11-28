//! Steps of a submission that read title memory flights in the air may
//! still write, done on the card in the flights' order instead of on the
//! CPU after a wait for them (`VICTORY_CARD_STEPS`, see [`stage`]).

use std::sync::Arc;

use super::*;
use crate::shadow::Shadow;

/// The stage (`VICTORY_CARD_STEPS`; off by default): `1` constant buffers
/// of descriptor tables (`Step::Fill` with a slot of its own), `2` the
/// title's buffer copies (`Step::Copy`) too, with the steps of the same
/// submission that depend on them. Only with the relay's second stage
/// (`relay_exact`).
///
/// Why: eleven dispatches a frame (the skinning passes, in two early
/// submissions) write through a view of the title's whole 174 MB heap, at
/// places they read out of job records in buffers: no bound of what they
/// write is known before they ran (frame capture replay2: 17af15c4015cb4b6,
/// 55679a0547649d0b, 89d08e561897d4e5, fdd56d97a806bcca and three more,
/// all through one view at offset 5920 of the title's bindless table). The
/// title's ring of constants lies in the same heap. One constant buffer of
/// 480 bytes and one copy of 20 KiB a frame, at a new place of the ring
/// each time, waited for these passes to land: 25-30 ms of a 105 ms frame,
/// the submitting thread standing (run record-1; with `VICTORY_FILL_TITLE=1`
/// in run relaycopy-1 the copy alone took over the wait).
///
/// What is done: a list of ours goes to the card in front of the
/// submission's lists, in the same call. It copies the bytes read from the
/// host copy that the flights write into a buffer of ours, and from there
/// to the destination's host copy (a copy), or the fill's descriptor slot
/// is pointed at that buffer (a fill; the slot is the fill's alone, the
/// root signatures say such descriptors are volatile, and the list that
/// names it has not gone to the card).
///
/// Why that is the value the CPU would have read after the wait. After the
/// wait title memory holds, for every byte read, what the title wrote, or
/// what the last of the flights that wrote it wrote (the landings put it
/// there, in order). The host copy on the card holds the same when our list
/// runs: every flight that may write one of these bytes writes this same
/// host copy ([`holder`]: a write through another host copy, a read-back
/// or a value written behind a flight there, and the old wait is taken),
/// they are all ahead of our list on the queue, and the title's own bytes
/// are brought up to date by a refresh step of our list, which is prepared
/// like any other list's (also under flights' declared writes:
/// `stale_under_flights`, which settles if it cannot). Nothing of this
/// submission writes these bytes earlier than our list: the steps are done
/// in front of all its lists in either case, and among themselves in the
/// order they stand in wherever one depends on another: [`plan`] takes
/// every copy and fill of the submission that writes what a taken one
/// reads or writes, or reads what a taken one writes. Every draw that
/// reads a buffer view of the heap relies on the same refresh and the same
/// order; what is new is only that a constant buffer and a copy do too.
/// Where a page is not mapped the host copy may hold other bytes than the
/// CPU's copy would have left: refused.
///
/// For a copy's destination our list declares a shader write, exactly the
/// bytes copied, after a refresh of them: from there on it is what any
/// shader's write is. The relay compares, and puts what changed into title
/// memory before the title is told anything of this flight (its
/// bottom-of-pipe writes and fences come after, in order); CPU steps of
/// later submissions that read it find it declared and wait, or are done
/// here. Refused: a step of the submission that cannot be taken (a texture,
/// vertices expanded, a fill of memory of another kind) reads the
/// destination behind the copy, or a refresh brings another host copy of it
/// up to date ([`plan`], [`elsewhere`]). While such a copy is in the air a
/// submission that refreshes another host copy of its destination, or
/// reads it in a step that is not checked otherwise, waits for it
/// ([`copied_read`]); with the copy done on the CPU it was there at once.
///
/// What stage 2 does not see: code of ours that reads title memory away
/// from a submission's steps (the pass over textures picked by index, while
/// a list is recorded). With the CPU's copy such a read of the destination
/// found the bytes at once; now it finds them when the flight has landed.
/// The landing tells the guards (`crate::watch::landing`), so a texture
/// made from older bytes is made again. This is why stage 2 is its own.
pub(super) fn stage() -> u32 {
    static STAGE: OnceLock<u32> = OnceLock::new();
    *STAGE.get_or_init(|| match std::env::var("VICTORY_CARD_STEPS").ok().as_deref() {
        Some("1") => 1,
        Some("2") => 2,
        _ => 0,
    })
}

/// Whether a copy at its place in a list ([`place`]) is only in the way of
/// what is done on the card in front of the lists if that stands behind it in
/// them (`VICTORY_PLACED_ORDER=1`; off: wherever it stands), and a
/// submission where it does is cut behind the copy's list ([`cut_for`]). Off:
/// the case did not come up once in runs submit-1 to submit-3, so it has only
/// its tests behind it.
///
/// The fills of constant buffers read the title's ring (7,500 a frame in the
/// opening), and one copy of a few hundred bytes a frame writes into it at its
/// place in a list. The whole submission was refused for it, and stood 40 to
/// 60 ms until the flights had landed (run n19).
fn placed_order() -> bool {
    #[cfg(feature = "diagnostics")]
    if let Some(on) = crate::ab::pick("VICTORY_PLACED_ORDER") {
        return on;
    }
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| crate::switch::on("VICTORY_PLACED_ORDER"))
}

/// Whether a fill behind a copy at its place in a list ([`place`]) that reads
/// only bytes that copy put is given them from the copy's source instead
/// (`VICTORY_PLACED_FORWARD`, `=1`; off by default), see [`forward`]; and
/// with it the steps ahead of such a copy are done in front of the lists
/// whatever they read or write of its bytes, as `VICTORY_PLACED_ORDER=1` does
/// (they are ahead of it in the order too).
///
/// The constant buffers of a frame are filled out of the title's ring, and
/// some 26 copies a frame write into the ring at their place in a list, out
/// of what shaders wrote. A fill of 480 bytes behind one of them read what it
/// had put: the whole submission was refused for it and stood 14-17 ms until
/// the flights had landed (run rdna4-tuned-stage2, 0.5 waits a frame, 13.7 ms).
fn forward_on() -> bool {
    crate::asked_for!("VICTORY_PLACED_FORWARD")
}

/// Why [`plan`] refuses for [`place`].
const BEHIND_PLACED: &str = "a copy at its place in a list writes what it reads, or reads or writes what it writes";

/// Why [`plan`] refuses a copy at its place whose source the CPU takes.
const PLACED_ON_CPU: &str = "a copy at its place takes its source on the CPU, where it is not yet";

/// Why [`plan`] refuses what the CPU leaves to read behind a copy at its
/// place that title memory does not take at submission.
const PLACED_STALE: &str = "a step left on the CPU reads what a copy at its place puts only on the card";

/// The bytes of our buffer between reading and writing: what one submission
/// may do this way (each place read once, however many fills take it).
const BETWEEN: u64 = 4 << 20;

/// The most steps of a submission done this way.
const MOST: usize = 12288;

/// How a fill's bytes get where its list reads them, if the card brings
/// them.
#[derive(Clone)]
pub(super) enum FillTo {
    /// A descriptor slot that shows the fill's memory, and nothing else does
    /// (title heap, slot): it is made to show our buffer instead.
    Slot(usize, u32),
    /// A buffer the card can write, filled from its beginning.
    Buffer(ID3D12Resource),
}

/// Title memory something in the air writes: the host copy it goes through
/// (`None`: none, the bytes reach title memory when the flight lands), from,
/// to, and whether it is the destination of a copy done here.
pub(super) struct Writer {
    shadow: Option<Arc<Shadow>>,
    from: usize,
    to: usize,
    copy: bool,
}

/// What `items` of the line write.
pub(super) fn writers(items: &[Late]) -> Vec<Writer> {
    let mut out = Vec::new();
    for item in items {
        match item {
            Late::Run(flight) => {
                for (shadow, ranges) in &flight.back {
                    out.extend(ranges.iter().map(|&(from, to)| Writer {
                        shadow: Some(shadow.clone()),
                        from,
                        to,
                        copy: false,
                    }));
                }
                for &i in &flight.order {
                    for &(_, at, [row, rows, pitch]) in &flight.lists[i].host().deferred().backs {
                        out.push(Writer {
                            shadow: None,
                            from: at,
                            to: at + rows.saturating_sub(1) * pitch + row,
                            copy: false,
                        });
                    }
                }
                out.extend(flight.own.iter().flat_map(Taken::copied));
            }
            Late::Write(at, _, bytes) => out.push(Writer {
                shadow: None,
                from: *at,
                to: *at + *bytes,
                copy: false,
            }),
            Late::Signal(..) => {}
        }
    }
    out
}

/// The destinations of copies done here that have not landed.
pub(super) fn copies(line: &Line) -> Vec<Writer> {
    if stage() < 2 {
        return Vec::new();
    }
    let flying = line.line.iter().filter_map(|l| match l {
        Late::Run(flight) => flight.own.as_ref(),
        _ => None,
    });
    let landing = line.landing.iter().filter(|w| w.copy).map(|w| Writer {
        shadow: w.shadow.clone(),
        ..*w
    });
    flying.flat_map(Taken::copied).chain(landing).collect()
}

/// Whether `runs` refresh another host copy of a destination in `copies`,
/// or read it in a step that flights are not waited for otherwise
/// (`Step::Host`): that reads title memory, where the copy is not yet.
pub(super) fn copied_read(copies: &[Writer], runs: &[&HostList]) -> bool {
    if copies.is_empty() {
        return false;
    }
    runs.iter().any(|list| {
        list.deferred().copies.iter().any(|step| match step {
            Step::Refresh(shadow, at, len) => copies
                .iter()
                .any(|w| w.from < *at + *len && *at < w.to && w.shadow.as_ref().is_none_or(|s| !Arc::ptr_eq(s, shadow))),
            Step::Host(_, (from, to)) => copies.iter().any(|w| w.from < *to && *from < w.to),
            Step::Plane((at, len), _) => copies.iter().any(|w| w.from < at.saturating_add(*len) && *at < w.to),
            _ => false,
        })
    })
}

/// What a step of a submission does to title memory, as the plan sees it.
/// `asked`: flights in the air stop it from being done on the CPU now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Does {
    Copy { dst: usize, src: usize, len: usize, asked: bool },
    /// `to`: its memory can be given the bytes on the card. `title`: where
    /// the CPU reads it if it is left there; `src` is where the card does,
    /// another place if [`forward`] found the bytes there.
    Fill { src: usize, len: usize, asked: bool, to: bool, title: usize },
    /// Another read on the CPU (a texture, vertices expanded).
    Read { from: usize, to: usize, asked: bool },
    /// A refresh of a host copy (named by its address).
    Refresh { shadow: usize, from: usize, to: usize },
    /// A copy that its list does on the card ([`place`]). `free`: no flight
    /// writes its source or destination, so title memory takes the copy at
    /// submission too; `cpu`: its source is taken on the CPU at submission;
    /// `stuck`: that source is one flights write ([`place::Waits::Yes`]).
    Placed { dst: usize, src: usize, len: usize, free: bool, cpu: bool, stuck: bool },
    /// Anything, anywhere.
    Run,
}

fn meet(a: (usize, usize), b: (usize, usize)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

/// The bytes `d` reads and writes as a copy or a fill (empty ranges for
/// what it does not).
fn reads_writes(d: &Does) -> Option<((usize, usize), (usize, usize))> {
    match *d {
        Does::Copy { dst, src, len, .. } => Some(((src, src + len), (dst, dst + len))),
        Does::Fill { src, len, .. } => Some(((src, src + len), (0, 0))),
        _ => None,
    }
}

fn touched(ranges: &[(usize, usize)], r: (usize, usize)) -> bool {
    touches(ranges, r.0, r.1)
}

/// The steps of `does` (by place) to be done on the card, in order, each
/// with its level; or why not. All that are asked for, and every copy and
/// fill that is in the way of one that is done there: it writes what that
/// one reads or writes, or reads what it writes, whichever comes first. So
/// what is left on the CPU writes none of their bytes, and reads of them
/// only what a taken copy has not been told to write yet: bytes a step
/// ahead of it reads, in title memory as it is.
///
/// `behind`: for [`BEHIND_PLACED`], the copy at its place and the first step
/// behind it that is in its way.
///
/// Steps of a level are done together, every read before every write; a
/// step that writes what an earlier one reads or writes, or reads what it
/// writes, is of a higher level. Our list does them level by level.
fn plan(does: &[Does], stage: u32, by_order: bool, behind: &mut Option<(usize, usize)>) -> Result<Vec<(usize, u32)>, &'static str> {
    let takes = |d: &Does| match *d {
        Does::Copy { .. } if stage < 2 => Err("a buffer copy (VICTORY_CARD_STEPS=2 does it on the card)"),
        Does::Fill { to: false, .. } => Err("a fill of memory the card cannot be given the bytes of"),
        _ => Ok(()),
    };
    let mut taken = vec![false; does.len()];
    let (mut read, mut written) = (Vec::new(), Vec::new());
    let mut more = Vec::new();
    for (i, d) in does.iter().enumerate() {
        match *d {
            Does::Run => return Err("a step of ours runs"),
            Does::Read { asked: true, .. } => return Err("a texture reads it"),
            // (Its source would be taken into upload memory before the
            // flights that write it have landed.)
            Does::Placed { stuck: true, .. } => return Err(PLACED_ON_CPU),
            Does::Copy { asked: true, .. } | Does::Fill { asked: true, .. } => more.push(i),
            _ => {}
        }
    }
    while !more.is_empty() {
        // (Joined once for the round: one by one the 2,800 fills of a frame
        // sorted the ranges each time, 16 ms of the submitting thread.)
        let (mut reads, mut writes) = (Vec::new(), Vec::new());
        for i in std::mem::take(&mut more) {
            takes(&does[i])?;
            taken[i] = true;
            if let Some((r, w)) = reads_writes(&does[i]) {
                if w.0 < w.1 && meet(r, w) {
                    return Err("a copy onto itself");
                }
                reads.push(r);
                writes.push(w);
            }
        }
        join_ranges(&mut read, reads);
        if writes.iter().any(|w| w.0 < w.1) {
            join_ranges(&mut written, writes);
        }
        if taken.iter().filter(|t| **t).count() > MOST {
            return Err("too many steps depend on it");
        }
        for (i, d) in does.iter().enumerate() {
            // A fill that cannot be taken is looked at below.
            if taken[i] || matches!(d, Does::Fill { to: false, .. }) {
                continue;
            }
            if let Some((r, w)) = reads_writes(d)
                && (touched(&written, r) || touched(&written, w) || touched(&read, w))
            {
                more.push(i);
            }
        }
    }
    // A copy at its place in a list comes after what is done in front of all
    // of them: as it should after the steps ahead of it, but not after those
    // behind it that read or write its bytes.
    for (p, d) in does.iter().enumerate() {
        if let Does::Placed { dst, src, len, .. } = *d
            && (touched(&read, (dst, dst + len)) || touched(&written, (dst, dst + len)) || touched(&written, (src, src + len)))
        {
            let (to, from) = ((dst, dst + len), (src, src + len));
            let in_the_way = |i: usize| {
                reads_writes(&does[i]).is_some_and(|(r, w)| meet(r, to) || w.0 < w.1 && (meet(w, to) || meet(w, from)))
            };
            let found = (p + 1..does.len()).find(|&i| taken[i] && in_the_way(i));
            *behind = found.map(|i| (p, i));
            if !by_order || found.is_some() {
                return Err(BEHIND_PLACED);
            }
        }
    }
    let order: Vec<usize> = (0..does.len()).filter(|&i| taken[i]).collect();
    // What stays on the CPU and reads what an earlier taken copy writes
    // would read it too early. So would what reads the destination of a copy
    // at its place that title memory does not take at submission (`stale`):
    // a flight writes its source or destination, or a step on the card its
    // source (title memory is given it when the flight lands).
    let mut behind: Vec<(usize, usize)> = Vec::new();
    let mut stale: Vec<(usize, usize)> = Vec::new();
    for (i, d) in does.iter().enumerate() {
        if taken[i] {
            if let Some((_, w)) = reads_writes(d)
                && w.0 < w.1
            {
                join_ranges(&mut behind, [w]);
            }
            continue;
        }
        let late = |r: (usize, usize)| touched(&behind, r) || touched(&stale, r);
        // What it reads on the CPU.
        let reads = match *d {
            Does::Read { from, to, .. } => Some((from, to)),
            Does::Fill { title, len, .. } => Some((title, title + len)),
            Does::Copy { src, len, .. } => Some((src, src + len)),
            _ => None,
        };
        if reads.is_some_and(|r| touched(&stale, r)) {
            return Err(PLACED_STALE);
        }
        match *d {
            Does::Read { from, to, .. } if touched(&behind, (from, to)) => {
                return Err("a texture or expanded vertices read what it writes");
            }
            Does::Fill { title, len, to: false, .. } if touched(&behind, (title, title + len)) => {
                return Err("a fill of memory the card cannot be given the bytes of reads what it writes");
            }
            // (Only one moved to a copy's source: any other is taken along.)
            Does::Fill { title, len, .. } if touched(&behind, (title, title + len)) => {
                return Err("a fill left on the CPU reads what it writes");
            }
            Does::Placed { dst, src, len, free, cpu, .. } => {
                let from = (src, src + len);
                if cpu && late(from) {
                    return Err(PLACED_ON_CPU);
                }
                if !free || late(from) {
                    join_ranges(&mut stale, [(dst, dst + len)]);
                }
            }
            _ => {}
        }
    }
    let mut out: Vec<(usize, u32)> = Vec::with_capacity(order.len());
    // The copies among them, which alone put others behind them or have to
    // stand behind others.
    let mut copies: Vec<usize> = Vec::new();
    for &i in &order {
        let Some((r, w)) = reads_writes(&does[i]) else {
            continue;
        };
        let writes = w.0 < w.1;
        let mut level = 0;
        // Behind what writes what this reads or writes, and, if this
        // writes, behind what reads it.
        let ahead: Box<dyn Iterator<Item = usize>> =
            if writes { Box::new(0..out.len()) } else { Box::new(copies.iter().copied()) };
        for k in ahead {
            let (j, at): (usize, u32) = out[k];
            let Some((jr, jw)) = reads_writes(&does[j]) else {
                continue;
            };
            if at >= level && (meet(jw, r) || meet(jw, w) || meet(jr, w)) {
                level = at + 1;
            }
        }
        if writes {
            copies.push(out.len());
        }
        out.push((i, level));
    }
    Ok(out)
}

/// Whether a refresh of `does` brings a host copy other than `shadow` up to
/// date in `[from, to)`.
fn elsewhere(does: &[Does], shadow: usize, from: usize, to: usize) -> bool {
    does.iter().any(|d| match *d {
        Does::Refresh { shadow: s, from: a, to: b } => s != shadow && meet((a, b), (from, to)),
        _ => false,
    })
}

/// The host copy that holds `[from, to)` on the card once what `writers`
/// write is there: the one they all write it through, or without any the
/// one shaders would be given.
fn holder(writers: &[Writer], from: usize, to: usize) -> Result<Arc<Shadow>, &'static str> {
    holder_among(writers.iter().filter(|w| meet((w.from, w.to), (from, to))), from, to)
}

/// Whether what flights write is looked through by address
/// (`VICTORY_HOLDER_INDEX=1`, off by default) instead of all of it for every
/// step: some 2,800 fills a frame, each against every range the flights in
/// the air write back, on the submitting thread.
fn holder_index_on() -> bool {
    crate::asked_for!("VICTORY_HOLDER_INDEX")
}

/// The writers of [`holder`], by where they begin, and those added since
/// (copies done here, a few).
struct Holders {
    sorted: Vec<Writer>,
    /// The furthest end of `sorted[..=i]`.
    reach: Vec<usize>,
    added: Vec<Writer>,
}

impl Holders {
    fn new(mut writers: Vec<Writer>) -> Self {
        writers.sort_by_key(|w| w.from);
        let mut reach = Vec::with_capacity(writers.len());
        let mut most = 0;
        for w in &writers {
            most = most.max(w.to);
            reach.push(most);
        }
        Self { sorted: writers, reach, added: Vec::new() }
    }

    fn push(&mut self, w: Writer) {
        self.added.push(w);
    }

    /// [`holder`] over all of them: the same answer (which of the errors
    /// may differ where both are).
    fn holder(&self, from: usize, to: usize) -> Result<Arc<Shadow>, &'static str> {
        let end = self.sorted.partition_point(|w| w.from < to);
        let mut i = end;
        let mut meeting: Vec<&Writer> = Vec::new();
        while i > 0 && self.reach[i - 1] > from {
            i -= 1;
            if meet((self.sorted[i].from, self.sorted[i].to), (from, to)) {
                meeting.push(&self.sorted[i]);
            }
        }
        meeting.extend(self.added.iter().filter(|w| meet((w.from, w.to), (from, to))));
        holder_among(meeting.into_iter(), from, to)
    }
}

fn holder_among<'a>(meeting: impl Iterator<Item = &'a Writer>, from: usize, to: usize) -> Result<Arc<Shadow>, &'static str> {
    let mut found: Option<&Arc<Shadow>> = None;
    for w in meeting {
        match (&w.shadow, found) {
            (None, _) => return Err("a read-back or a value behind a flight is written there"),
            (Some(s), Some(f)) if !Arc::ptr_eq(s, f) => return Err("flights write it through two host copies"),
            (Some(s), _) => found = Some(s),
        }
    }
    let shadow = match found {
        Some(s) => s.clone(),
        None => crate::shadow::find(from as u64, (to - from) as u64).ok_or("no host copy of it")?,
    };
    let (start, len) = shadow.span();
    if shadow.in_place() || (from as u64) < start || to as u64 > start + len {
        return Err("no host copy holds all of it");
    }
    Ok(shadow)
}

/// Fills of `does` that the card can do behind a copy at its place in a list
/// ([`place`]) that put every byte they read: they read the copy's source
/// instead, at the same offset (and on through a copy at its place that put
/// that, and so on). Gives their places.
///
/// Why that is what they read on the CPU after the wait. There a copy at its
/// place is done in title memory at submission, in the order of the steps:
/// a fill behind it reads its source's bytes as they were when its step came.
/// The card reads that source in front of the lists, which is the same if
/// nothing between the copy and the fill writes it: no copy, at its place or
/// not, and no step that runs (refused by [`plan`] anyway). Steps ahead of
/// the copy that write its source are ordered before the fill by [`plan`],
/// as before any reader; a copy at its place ahead of it that writes the
/// source puts it only in its list, and [`plan`] refuses the fill for it
/// ([`BEHIND_PLACED`]), unless this went on through that copy too.
///
/// The fill a draw ahead of the copy reads is left alone: in front of the
/// lists the destination holds what it held before the copy. A fill that
/// reads part of what the copy put and part of something else is left as it
/// is (and refused, as before). Such a fill must be done on the card if title
/// memory does not take a copy it goes through at submission (`free`).
fn forward(written: &[(usize, usize)], does: &mut [Does]) -> Vec<usize> {
    let mut moved = Vec::new();
    // What steps so far write (their place, the bytes).
    let mut writes: Vec<(usize, (usize, usize))> = Vec::new();
    let mut placed = false;
    for i in 0..does.len() {
        match does[i] {
            Does::Fill { src, len, asked, to: true, title } if placed => {
                if let Some((from, stale)) = forwarded(does, &writes, (src, src + len)) {
                    let asked = asked || stale || read_blocked(written, from, len);
                    does[i] = Does::Fill { src: from, len, asked, to: true, title };
                    moved.push(i);
                }
            }
            Does::Copy { dst, len, .. } => writes.push((i, (dst, dst + len))),
            Does::Placed { dst, len, .. } => {
                writes.push((i, (dst, dst + len)));
                placed = true;
            }
            Does::Run => break,
            _ => {}
        }
    }
    moved
}

/// Where the bytes `read` are, read in front of the lists, for a step behind
/// all of `writes` ([`forward`]); and whether a copy they go through is one
/// title memory does not take at submission. `None`: where they are.
fn forwarded(does: &[Does], writes: &[(usize, (usize, usize))], mut read: (usize, usize)) -> Option<(usize, bool)> {
    let mut found = None;
    let mut stale = false;
    let mut end = writes.len();
    while let Some(k) = writes[..end].iter().rposition(|&(_, w)| meet(w, read)) {
        let Does::Placed { dst, src, len, free, .. } = does[writes[k].0] else {
            break;
        };
        if read.0 < dst || read.1 > dst + len || meet((src, src + len), (dst, dst + len)) {
            break;
        }
        let from = (src + (read.0 - dst), src + (read.1 - dst));
        // What is written of it between the copy and the step.
        if writes[k + 1..].iter().any(|&(_, w)| meet(w, from)) {
            break;
        }
        (read, end, found) = (from, k, Some(from.0));
        stale |= !free;
    }
    found.map(|from| (from, stale))
}

/// What the steps of `runs` do, and which step each is (its list by address,
/// its place in it), and the fills [`forward`] moved. `written`: what flights
/// may write.
fn does_of(written: &[(usize, usize)], runs: &[&HostList]) -> (Vec<Does>, Vec<(usize, usize)>, Vec<usize>) {
    let mut does = Vec::new();
    // Which step each is. One list's lock at a time: a submission may
    // name a list twice.
    let mut steps = Vec::new();
    for list in runs {
        for (i, step) in list.deferred().copies.iter().enumerate() {
            does.push(match step {
                &Step::Copy(dst, src, len) => Does::Copy {
                    dst,
                    src,
                    len,
                    asked: copy_blocked(written, dst, src, len).is_some(),
                },
                Step::Fill(_, src, len, to) => Does::Fill {
                    src: *src,
                    len: *len,
                    asked: read_blocked(written, *src, *len),
                    to: to.is_some(),
                    title: *src,
                },
                Step::Texture(staging) => {
                    let (at, len) = staging.memory();
                    Does::Read {
                        from: at,
                        to: at + len,
                        asked: touches(written, at, at + len),
                    }
                }
                &Step::Plane((at, len), _) => Does::Read {
                    from: at,
                    to: at.saturating_add(len),
                    asked: touches(written, at, at.saturating_add(len)),
                },
                &Step::Host(_, (from, to)) if from < to => Does::Read { from, to, asked: false },
                Step::Refresh(shadow, at, len) => Does::Refresh {
                    shadow: Arc::as_ptr(shadow) as usize,
                    from: *at,
                    to: *at + *len,
                },
                Step::Run(_) => Does::Run,
                Step::Placed(p) => Does::Placed {
                    dst: p.dst,
                    src: p.src,
                    len: p.len,
                    free: !touches(written, p.src, p.src + p.len) && !touches(written, p.dst, p.dst + p.len),
                    cpu: p.source_on_cpu(),
                    stuck: matches!(p.waits(written), place::Waits::Yes),
                },
                Step::GpuWrite(..) | Step::Host(..) => continue,
            });
            steps.push((std::ptr::from_ref::<HostList>(list) as usize, i));
        }
    }
    let moved = if forward_on() { forward(written, &mut does) } else { Vec::new() };
    (does, steps, moved)
}

/// With flights in the air: before which of `runs` the submission is cut so
/// that what reads or writes the bytes of a copy at its place in an earlier
/// list ([`BEHIND_PLACED`]) is done on the card in front of a submission of
/// its own, behind that copy.
pub(super) fn cut_for(runs: &[&HostList]) -> Option<usize> {
    if stage() == 0 || !relay_exact() || !placed_order() || runs.len() < 2 {
        return None;
    }
    // (Most submissions have none.)
    let first = runs.iter().position(|l| l.deferred().copies.iter().any(|s| matches!(s, Step::Placed(_))))?;
    if first + 1 == runs.len() {
        return None;
    }
    let at = std::time::Instant::now();
    let written = line().blocking().clone();
    blocked_by(&written, runs, false)?;
    let (does, steps, _) = does_of(&written, runs);
    let mut behind = None;
    let behind_placed = plan(&does, stage(), true, &mut behind) == Err(BEHIND_PLACED);
    plan_ns(6, at);
    if !behind_placed {
        return None;
    }
    let (placed, reader) = behind?;
    let place = |i: usize| runs.iter().position(|l| std::ptr::from_ref::<HostList>(l) as usize == steps[i].0);
    let cut = place(placed)? + 1;
    (place(reader)? >= cut).then_some(cut)
}

/// Whether every page of `[from, to)` can be read (through the kernel:
/// asking Windows walks the page tables, milliseconds in the title's heap).
/// `known`: pages that could be, of the same submission: its fills' places
/// lie eight to a page, and each was read whole to find out.
fn mapped(from: usize, to: usize, known: &mut HashMap<usize, ()>) -> bool {
    const PAGE: usize = 4096;
    (from / PAGE..to.div_ceil(PAGE)).all(|page| {
        known.contains_key(&page) || {
            let there = read_into((page * PAGE).max(from) as *const u8, &mut [0]);
            if there {
                known.insert(page, ());
            }
            there
        }
    })
}

/// A table by addresses and numbers of ours.
type HashMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<AddressHash>>;

/// Where the bytes of the fill that is `step` (its list by address, its place
/// in it) go.
fn fill_to(runs: &[&HostList], step: (usize, usize)) -> Option<FillTo> {
    let list = runs.iter().find(|l| std::ptr::from_ref::<HostList>(l) as usize == step.0)?;
    match list.deferred().copies.get(step.1)? {
        Step::Fill(_, _, _, to) => to.clone(),
        _ => None,
    }
}

/// A step done on the card.
struct Op {
    from: (Arc<Shadow>, usize),
    len: usize,
    /// Its place in the buffer between.
    at: u64,
    level: u32,
    to: To,
}

enum To {
    /// A host copy, and the title address.
    Copy(Arc<Shadow>, usize),
    /// A descriptor slot (title heap, slot).
    Slot(usize, u32),
    /// A buffer's beginning.
    Buffer(ID3D12Resource),
}

/// Whether the reads of a level are gathered by one dispatch for each buffer
/// they read from instead of a copy each (`VICTORY_CARD_GATHER`, `=1`;
/// off by default: copies, as before).
///
/// Why: some 8,600 fills a frame, of a few hundred bytes each and seldom of
/// the same bytes, were as many `CopyBufferRegion`s, on the queue the card
/// was 96% busy with (run rdna4-tuned-stage2). Each is a command the card's
/// front end goes through on its own; the gather is one dispatch, a group of
/// threads for each read.
fn gather_on() -> bool {
    crate::asked_for!("VICTORY_CARD_GATHER")
}

/// Fewer reads than this out of one buffer are copied.
const GATHER_LEAST: usize = 8;

/// The gather: job `i` (group `i`) copies `words` words from `src` at
/// `from` to `dst` at `to`, its threads a word apart.
const GATHER: &str = r#"
#define RS "RootConstants(num32BitConstants=1, b0), SRV(t0), SRV(t1), UAV(u0)"
cbuffer Count : register(b0) { uint count; };
ByteAddressBuffer src : register(t0);
// Per job: from, to, words, nothing.
ByteAddressBuffer jobs : register(t1);
RWByteAddressBuffer dst : register(u0);
[RootSignature(RS)]
[numthreads(64, 1, 1)]
void main(uint3 group : SV_GroupID, uint gi : SV_GroupIndex) {
    if (group.x >= count) return;
    uint4 job = jobs.Load4(group.x * 16);
    for (uint w = gi; w < job.z; w += 64) {
        dst.Store(job.y + w * 4, src.Load(job.x + w * 4));
    }
}
"#;

/// The gather's root signature and pipeline.
struct Gather {
    root: ID3D12RootSignature,
    pso: ID3D12PipelineState,
}

// SAFETY: D3D12 objects are free-threaded.
unsafe impl Send for Gather {}
unsafe impl Sync for Gather {}

/// The gather, made once; `None` if it could not be (reads are copied).
fn gather() -> Option<&'static Gather> {
    static MADE: OnceLock<Option<Gather>> = OnceLock::new();
    MADE.get_or_init(|| {
        let made = host().ok_or_else(windows::core::Error::empty).and_then(|h| make_gather(&h.device));
        made.inspect_err(|e| tracing::warn!(error = %e, "no gather for steps on the card: they are copied")).ok()
    })
    .as_ref()
}

fn make_gather(device: &ID3D12Device) -> windows::core::Result<Gather> {
    use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
    use windows::Win32::Graphics::Direct3D::ID3DBlob;
    let (mut code, mut errors): (Option<ID3DBlob>, Option<ID3DBlob>) = (None, None);
    // SAFETY: the source and names live through the call; out pointers ours.
    let compiled = unsafe {
        D3DCompile(
            GATHER.as_ptr().cast(),
            GATHER.len(),
            windows::core::s!("gather"),
            None,
            None,
            windows::core::s!("main"),
            windows::core::s!("cs_5_1"),
            0,
            0,
            &mut code,
            Some(&mut errors),
        )
    };
    if let Err(e) = compiled {
        if let Some(errors) = errors {
            // SAFETY: the blob's own bytes.
            let text = unsafe { std::slice::from_raw_parts(errors.GetBufferPointer().cast::<u8>(), errors.GetBufferSize()) };
            tracing::warn!("gather: {}", String::from_utf8_lossy(text));
        }
        return Err(e);
    }
    let code = code.ok_or_else(windows::core::Error::empty)?;
    // SAFETY: the blob's own bytes; the root signature is inside the shader.
    unsafe {
        let bytes = std::slice::from_raw_parts(code.GetBufferPointer().cast::<u8>(), code.GetBufferSize());
        let root: ID3D12RootSignature = device.CreateRootSignature(0, bytes)?;
        let pso = device.CreateComputePipelineState(&D3D12_COMPUTE_PIPELINE_STATE_DESC {
            pRootSignature: std::mem::transmute_copy(&root),
            CS: D3D12_SHADER_BYTECODE {
                pShaderBytecode: bytes.as_ptr().cast(),
                BytecodeLength: bytes.len(),
            },
            ..Default::default()
        })?;
        name(&pso, || "steps on the card: gather".into());
        Ok(Gather { root, pso })
    }
}

/// Of `reads` (buffer, offset, offset in the buffer between, bytes): the
/// buffers whose reads are gathered, each with the places of its reads in
/// `reads`. A buffer's reads are gathered all or none: words on words, its
/// offsets 32 bits, and [`GATHER_LEAST`] of them at least.
fn gathered<'a>(reads: &[(&'a ID3D12Resource, u64, u64, u64)]) -> Vec<(&'a ID3D12Resource, Vec<usize>)> {
    let mut out: Vec<(&ID3D12Resource, Vec<usize>, bool)> = Vec::new();
    for (i, &(buffer, offset, at, len)) in reads.iter().enumerate() {
        let fits = (offset | at | len).is_multiple_of(4) && offset + len <= u64::from(u32::MAX) && at + len <= u64::from(u32::MAX);
        match out.iter_mut().find(|g| g.0.as_raw() == buffer.as_raw()) {
            Some(g) => {
                g.1.push(i);
                g.2 &= fits;
            }
            None => out.push((buffer, vec![i], fits)),
        }
    }
    out.into_iter().filter(|g| g.2 && g.1.len() >= GATHER_LEAST).map(|g| (g.0, g.1)).collect()
}

/// Since the last frame-rate line: copies recorded for steps on the card,
/// reads gathered, and the dispatches that gathered them.
static RECORDED: [std::sync::atomic::AtomicU64; 3] = [const { std::sync::atomic::AtomicU64::new(0) }; 3];

/// Records a level of what [`stage`] describes into `list` (open): `reads`
/// are copied or gathered ([`gather_on`]) into `between` (source, offset in
/// it, offset in `between`, bytes), then `writes` copied out of it
/// (destination, offset in it, offset in `between`, bytes). `between` is
/// left readable as constants (and must be so, unless this is the `first`
/// level), the others in the common state, as the submission's lists expect
/// them.
fn record(
    first: bool,
    host_list: &HostList,
    between: &ID3D12Resource,
    reads: &[(&ID3D12Resource, u64, u64, u64)],
    writes: &[(&ID3D12Resource, u64, u64, u64)],
) {
    let list = &host_list.list;
    let common = D3D12_RESOURCE_STATE_COMMON;
    let (source, dest) = (D3D12_RESOURCE_STATE_COPY_SOURCE, D3D12_RESOURCE_STATE_COPY_DEST);
    let unordered = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
    let shader = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
    // The reads gathered, with their jobs' table (in the list's upload
    // memory, until it is reset).
    let pipeline = if gather_on() { gather() } else { None };
    let mut groups: Vec<(&ID3D12Resource, Vec<usize>, u64)> = Vec::new();
    if pipeline.is_some() {
        for (buffer, of) in gathered(reads) {
            let Some((jobs, data)) = host_list.scratch(16 * of.len() as u64) else {
                break;
            };
            for (k, &i) in of.iter().enumerate() {
                let (_, offset, at, len) = reads[i];
                let job = [offset as u32, at as u32, (len / 4) as u32, 0];
                // SAFETY: the list's own upload memory of 16 bytes a job.
                unsafe { std::ptr::copy_nonoverlapping(job.as_ptr(), data.cast::<u32>().add(4 * k), 4) };
            }
            groups.push((buffer, of, jobs));
        }
    }
    let is_gathered = |b: &ID3D12Resource| groups.iter().any(|g| g.0.as_raw() == b.as_raw());
    let mut sources: Vec<&ID3D12Resource> = Vec::new();
    let mut dests: Vec<&ID3D12Resource> = Vec::new();
    for (all, list) in [(reads, &mut sources), (writes, &mut dests)] {
        for &(buffer, ..) in all {
            if !list.iter().any(|b| b.as_raw() == buffer.as_raw()) {
                list.push(buffer);
            }
        }
    }
    let both = |b: &ID3D12Resource| dests.iter().any(|d| d.as_raw() == b.as_raw());
    let constants = source | D3D12_RESOURCE_STATE_VERTEX_AND_CONSTANT_BUFFER;
    let mut barriers = vec![transition(between, dest, constants)];
    // (A buffer gathered from was promoted to be read by shaders, the
    // others to be copied from.)
    barriers.extend(sources.iter().map(|b| {
        let read = if is_gathered(b) { shader } else { source };
        transition(b, read, if both(b) { dest } else { common })
    }));
    let done: Vec<D3D12_RESOURCE_BARRIER> = dests.iter().map(|b| transition(b, dest, common)).collect();
    let copies = reads.len() - groups.iter().map(|g| g.1.len()).sum::<usize>();
    RECORDED[0].fetch_add((copies + writes.len()) as u64, Ordering::Relaxed);
    RECORDED[1].fetch_add((reads.len() - copies) as u64, Ordering::Relaxed);
    RECORDED[2].fetch_add(groups.len() as u64, Ordering::Relaxed);
    // SAFETY: an open list; live buffers, and the callers' ranges lie in
    // them. The buffers start in the common state (the list is the first of
    // its call) and are promoted by their first use; `between` too, but
    // where it is gathered into it is moved there and out again.
    unsafe {
        if first {
            // What earlier lists' shaders wrote, first.
            list.ResourceBarrier(&[D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_UAV,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    UAV: std::mem::ManuallyDrop::new(D3D12_RESOURCE_UAV_BARRIER {
                        pResource: std::mem::ManuallyDrop::new(None),
                    }),
                },
            }]);
        }
        match (first, groups.is_empty()) {
            (true, true) => {}
            (true, false) => list.ResourceBarrier(&[transition(between, common, unordered)]),
            (false, true) => list.ResourceBarrier(&[transition(between, constants, dest)]),
            (false, false) => list.ResourceBarrier(&[transition(between, constants, unordered)]),
        }
        if let (Some(g), false) = (pipeline, groups.is_empty()) {
            list.SetComputeRootSignature(&g.root);
            list.SetPipelineState(&g.pso);
            list.SetComputeRootUnorderedAccessView(3, between.GetGPUVirtualAddress());
            for (buffer, of, jobs) in &groups {
                list.SetComputeRoot32BitConstant(0, of.len() as u32, 0);
                list.SetComputeRootShaderResourceView(1, buffer.GetGPUVirtualAddress());
                list.SetComputeRootShaderResourceView(2, *jobs);
                list.Dispatch(of.len() as u32, 1, 1);
            }
            // (The gather's writes are done before what follows.)
            list.ResourceBarrier(&[transition(between, unordered, dest)]);
        }
        for &(buffer, offset, at, len) in reads.iter().filter(|r| !is_gathered(r.0)) {
            list.CopyBufferRegion(between, at, buffer, offset, len);
        }
        list.ResourceBarrier(&barriers);
        for &(buffer, offset, at, len) in writes {
            list.CopyBufferRegion(buffer, offset, between, at, len);
        }
        if !done.is_empty() {
            list.ResourceBarrier(&done);
        }
    }
}

/// The list in front of a submission's, and its buffer (`size` bytes, at
/// most [`BETWEEN`]).
struct Own {
    list: HostList,
    between: ID3D12Resource,
    size: u64,
}

/// The smallest buffer between: a submission's steps read some 110 KB on
/// average (4.25 MB a frame in 38 submissions, run rdna4-tuned-stage2).
const BETWEEN_LEAST: u64 = 128 << 10;

/// The bytes of the buffer between made for `need` bytes: a power of two,
/// so that a pooled one serves the submissions of about its size.
fn between_size(need: u64) -> u64 {
    need.max(BETWEEN_LEAST).next_power_of_two().min(BETWEEN)
}

/// The most lists kept in [`POOL`]. Each was a list and 4 MiB of the card's
/// memory, and as many were kept as were ever in the air at once.
const POOLED: usize = 32;

/// Since the start: lists with their buffer made, and the bytes of those
/// made and let go (for the frame line: what the pool holds).
static MADE: [std::sync::atomic::AtomicU64; 3] = [const { std::sync::atomic::AtomicU64::new(0) }; 3];

/// Back into the pool once landed; past [`POOLED`] the smallest goes, as it
/// serves the fewest submissions.
fn pool(own: Own) {
    let gone = {
        let mut pool = POOL.lock().unwrap_or_else(|e| e.into_inner());
        pool.push(own);
        (pool.len() > POOLED)
            .then(|| pool.iter().enumerate().min_by_key(|(_, o)| o.size).map(|(i, _)| i))
            .flatten()
            .map(|i| pool.swap_remove(i))
    };
    if let Some(gone) = gone {
        MADE[2].fetch_add(gone.size, Ordering::Relaxed);
    }
}

// SAFETY: D3D12 objects are free-threaded, and the list is touched by one
// thread at a time (the submitting one until it is sent, then the one that
// lands it, then nobody until it is taken out of `POOL`).
unsafe impl Send for Own {}

/// Those that have landed: closed, their steps dropped.
static POOL: Mutex<Vec<Own>> = Mutex::new(Vec::new());

/// Since the last frame-rate line: fills and copies done on the card, their
/// bytes, submissions that waited instead, refused by a check, and fills done
/// on the card out of a copy's source ([`forward`]).
/// Nanoseconds of the planning, by part, since the last frame-rate line:
/// what the steps do, the plan, what holds their bytes, the list, the slots,
/// the copy-or-not check of [`Host::card_steps`], and [`cut_for`].
static PLAN_NS: [std::sync::atomic::AtomicU64; 7] = [const { std::sync::atomic::AtomicU64::new(0) }; 7];

fn plan_ns(part: usize, from: std::time::Instant) -> std::time::Instant {
    let now = std::time::Instant::now();
    PLAN_NS[part].fetch_add((now - from).as_nanos() as u64, Ordering::Relaxed);
    now
}

static DONE: [std::sync::atomic::AtomicU64; 5] = [const { std::sync::atomic::AtomicU64::new(0) }; 5];

/// What [`Host::card_steps`] took off a submission: the list that does it,
/// for the submission to run first, and the steps the CPU leaves out (their
/// list by address, their place in it).
pub(super) struct Taken {
    own: Own,
    steps: std::collections::HashSet<(usize, usize), std::hash::BuildHasherDefault<AddressHash>>,
    copied: Vec<(Arc<Shadow>, usize, usize)>,
}

impl Taken {
    pub(super) fn list(&self) -> &HostList {
        &self.own.list
    }

    /// Whether step `i` of `list` is done by this.
    pub(super) fn takes(&self, list: &HostList, i: usize) -> bool {
        self.steps.contains(&(std::ptr::from_ref(list) as usize, i))
    }

    fn copied(&self) -> impl Iterator<Item = Writer> {
        self.copied.iter().map(|(shadow, from, to)| Writer {
            shadow: Some(shadow.clone()),
            from: *from,
            to: *to,
            copy: true,
        })
    }

    /// The submission has run and landed: the list and its buffer are free.
    pub(super) fn landed(self) {
        // The host copies its steps held go; its commands stay until the
        // next use resets it.
        drop(std::mem::take(&mut *self.own.list.deferred()));
        pool(self.own);
    }
}

impl Host {
    /// With flights in the air: if what keeps `runs` from being prepared
    /// now is reads of title memory that can all be done on the card
    /// ([`stage`]), the list that does them. `None` if nothing is in the
    /// way or the old wait is due. Under the submission lock.
    pub(super) fn card_steps(&self, runs: &[&HostList]) -> Option<Taken> {
        if stage() == 0 || !relay_exact() {
            return None;
        }
        let (written, copies) = {
            let line = line();
            if line.blocking().is_empty() {
                return None;
            }
            (line.blocking().clone(), copies(&line))
        };
        let at = std::time::Instant::now();
        let blocked = blocked_by(&written, runs, false);
        let copied = blocked.is_some() && copied_read(&copies, runs);
        plan_ns(5, at);
        blocked?;
        let taken = if copied {
            Err("a copy done on the card is read through another host copy")
        } else {
            self.take_steps(&written, runs)
        };
        match taken {
            Ok(taken) => taken,
            Err(why) => {
                DONE[3].fetch_add(1, Ordering::Relaxed);
                #[cfg(feature = "diagnostics")]
                refused(why);
                let _ = why;
                None
            }
        }
    }

    fn take_steps(&self, written: &[(usize, usize)], runs: &[&HostList]) -> Result<Option<Taken>, &'static str> {
        let mut spent = crate::timeline::span("steps on the card: chosen");
        let at = std::time::Instant::now();
        let (does, steps, moved) = does_of(written, runs);
        let at = plan_ns(0, at);
        let chosen = plan(&does, stage(), placed_order() || forward_on(), &mut None);
        let at = plan_ns(1, at);
        let chosen = chosen?;
        if chosen.is_empty() {
            return Ok(None);
        }
        spent.next("steps on the card: what holds their bytes");
        let mut writers = {
            let line = line();
            let mut all = writers(&line.line);
            all.extend(line.landing.iter().map(|w| Writer {
                shadow: w.shadow.clone(),
                ..*w
            }));
            all
        };
        let mut index = holder_index_on().then(|| Holders::new(std::mem::take(&mut writers)));
        let holder = |writers: &[Writer], index: &Option<Holders>, from: usize, to: usize| match index {
            Some(index) => index.holder(from, to),
            None => holder(writers, from, to),
        };
        let mut ops: Vec<Op> = Vec::with_capacity(chosen.len());
        // The places read (host copy, address, bytes, level): where in the
        // buffer between their bytes are.
        let mut places: HashMap<(usize, usize, usize, u32), u64> = HashMap::default();
        // Some 2,800 fills a frame, of a few hundred places: what holds each
        // is looked for once, as long as no copy is added to the writers.
        let mut held: HashMap<(usize, usize), Arc<Shadow>> = HashMap::default();
        let mut room = HashMap::default();
        let mut end = 0u64;
        for &(i, level) in &chosen {
            let Some(((src, _), (dst, _))) = reads_writes(&does[i]) else {
                continue;
            };
            let (Does::Copy { len, .. } | Does::Fill { len, .. }) = does[i] else {
                continue;
            };
            if len == 0 {
                return Err("nothing to read");
            }
            let from = match held.get(&(src, len)) {
                Some(from) => from.clone(),
                None => {
                    let from = holder(&writers, &index, src, src + len)?;
                    held.insert((src, len), from.clone());
                    from
                }
            };
            let place = (Arc::as_ptr(&from) as usize, src, len, level);
            let at = match places.get(&place) {
                Some(&at) => at,
                None => {
                    if end + len as u64 > BETWEEN {
                        return Err("too much at once");
                    }
                    if !mapped(src, src + len, &mut room) {
                        return Err("not all of what it reads is mapped");
                    }
                    let at = end;
                    end = (end + len as u64).next_multiple_of(256);
                    places.insert(place, at);
                    at
                }
            };
            let to = match does[i] {
                Does::Copy { .. } => {
                    let to = holder(&writers, &index, dst, dst + len)?;
                    if elsewhere(&does, Arc::as_ptr(&to) as usize, dst, dst + len) {
                        return Err("another host copy of what it writes is read");
                    }
                    if !mapped(dst, dst + len, &mut room) {
                        return Err("not all of what it writes is mapped");
                    }
                    // What the steps behind it read of it is in that host
                    // copy, or nowhere.
                    let added = Writer {
                        shadow: Some(to.clone()),
                        from: dst,
                        to: dst + len,
                        copy: true,
                    };
                    match &mut index {
                        Some(index) => index.push(added),
                        None => writers.push(added),
                    }
                    held.clear();
                    To::Copy(to, dst)
                }
                _ => match fill_to(runs, steps[i]) {
                    Some(FillTo::Slot(heap, index)) => To::Slot(heap, index),
                    Some(FillTo::Buffer(buffer)) => To::Buffer(buffer),
                    None => continue,
                },
            };
            ops.push(Op {
                from: (from, src),
                len,
                at,
                level,
                to,
            });
        }
        spent.next("steps on the card: the list");
        let at = plan_ns(2, at);
        let Some(own) = self.own_list(end) else {
            return Err("no host list or buffer for it");
        };
        // First every refresh, then the writes: the bytes compared with are
        // the title's. (One step for each host copy, over all that is read of
        // it: a flush brings a copy up to date from the first range asked for
        // to the last, whatever the steps.)
        let mut spans: ByCopy<(Arc<Shadow>, usize, usize)> = ByCopy::default();
        for op in &ops {
            let to = match &op.to {
                To::Copy(to, dst) => Some((to, *dst)),
                _ => None,
            };
            for (shadow, at) in std::iter::once((&op.from.0, op.from.1)).chain(to) {
                let span = spans.entry(Arc::as_ptr(shadow) as usize).or_insert_with(|| (shadow.clone(), at, at + op.len));
                (span.1, span.2) = (span.1.min(at), span.2.max(at + op.len));
            }
        }
        for (shadow, from, to) in spans.into_values() {
            shadow.refresh(&own.list, from as u64, (to - from) as u64);
        }
        let mut copied = Vec::new();
        for op in &ops {
            if let To::Copy(to, dst) = &op.to {
                to.gpu_writes(&own.list, *dst as u64, op.len as u64);
                copied.push((to.clone(), *dst, *dst + op.len));
            }
        }
        #[cfg(feature = "diagnostics")]
        own.list.note_op(|| format!("{} copies of title memory done on the card, {end} bytes (VICTORY_CARD_STEPS)", ops.len()));
        let levels = ops.iter().map(|op| op.level).max().unwrap_or(0);
        for level in 0..=levels {
            let mut reads = Vec::new();
            let mut writes = Vec::new();
            for op in ops.iter().filter(|op| op.level == level) {
                let len = op.len as u64;
                let read = (&op.from.0.buffer, op.from.0.offset(op.from.1 as u64), op.at, len);
                // (Each place once.)
                if places.remove(&(Arc::as_ptr(&op.from.0) as usize, op.from.1, op.len, level)).is_some() {
                    reads.push(read);
                }
                match &op.to {
                    To::Copy(to, dst) => writes.push((&to.buffer, to.offset(*dst as u64), op.at, len)),
                    To::Buffer(buffer) => writes.push((buffer, 0, op.at, len)),
                    To::Slot(..) => {}
                }
            }
            record(level == 0, &own.list, &own.between, &reads, &writes);
        }
        own.list.close();
        // SAFETY: a live buffer.
        let gpu = unsafe { own.between.GetGPUVirtualAddress() };
        spent.next("steps on the card: the slots");
        let at = plan_ns(3, at);
        // (11,000 slots a frame in a street, two descriptors each: on
        // several threads, a slot always on the same one, in the steps'
        // order.)
        let parts = if ops.len() < 256 { 1 } else { 8 };
        crate::land::each_under(parts, &|part| {
            for op in &ops {
                if let To::Slot(heap, index) = &op.to
                    && *index as usize / 4 % parts == part
                {
                    // (A heap that is gone: nothing can read the slot.)
                    crate::descriptor::constant_slot(*heap, *index, &own.between, gpu + op.at, op.len as u32);
                }
            }
        });
        plan_ns(4, at);
        let copies = ops.iter().filter(|op| matches!(op.to, To::Copy(..))).count();
        DONE[0].fetch_add((ops.len() - copies) as u64, Ordering::Relaxed);
        DONE[1].fetch_add(copies as u64, Ordering::Relaxed);
        DONE[2].fetch_add(ops.iter().map(|op| op.len as u64).sum(), Ordering::Relaxed);
        if !moved.is_empty() {
            let n = chosen.iter().filter(|&&(i, _)| moved.binary_search(&i).is_ok()).count();
            DONE[4].fetch_add(n as u64, Ordering::Relaxed);
        }
        #[cfg(feature = "diagnostics")]
        noted(&ops);
        spent.end();
        Ok(Some(Taken {
            own,
            steps: chosen.iter().map(|&(i, _)| steps[i]).collect(),
            copied,
        }))
    }

    /// A list of ours open for recording, and its buffer of `need` bytes at
    /// least: the smallest pooled one that holds them, else a new one.
    fn own_list(&self, need: u64) -> Option<Own> {
        let pooled = {
            let mut pool = POOL.lock().unwrap_or_else(|e| e.into_inner());
            let best = pool.iter().enumerate().filter(|(_, o)| o.size >= need).min_by_key(|(_, o)| o.size).map(|(i, _)| i);
            best.map(|i| pool.swap_remove(i))
        };
        if let Some(own) = pooled {
            own.list.reset();
            return Some(own);
        }
        let size = between_size(need);
        let between = self.uav_buffer(size)?;
        name(&between, || "copies of title memory on the card".into());
        let list = self.new_list().ok()?;
        MADE[0].fetch_add(1, Ordering::Relaxed);
        MADE[1].fetch_add(size, Ordering::Relaxed);
        Some(Own { list, between, size })
    }
}

/// Diagnostics: the copies and fills done on the card since the last
/// frame-rate line, by bytes, host copy read and host copy written (start
/// and length; zeros for a fill): how many, and the last title addresses.
#[cfg(feature = "diagnostics")]
type Noted = std::collections::HashMap<(usize, (u64, u64), (u64, u64)), (u64, usize, usize)>;
#[cfg(feature = "diagnostics")]
static NOTED: Mutex<Option<Noted>> = Mutex::new(None);

#[cfg(feature = "diagnostics")]
fn noted(ops: &[Op]) {
    let mut noted = NOTED.lock().unwrap_or_else(|e| e.into_inner());
    let noted = noted.get_or_insert_with(Default::default);
    for op in ops {
        let (to, dst) = match &op.to {
            To::Copy(to, dst) => (to.span(), *dst),
            To::Slot(..) | To::Buffer(_) => ((0, 0), 0),
        };
        let e = noted.entry((op.len, op.from.0.span(), to)).or_default();
        *e = (e.0 + 1, op.from.1, dst);
    }
}

/// For the frame-rate line.
pub(super) fn log(frames: f64) {
    if stage() == 0 {
        return;
    }
    let d: Vec<f64> = DONE.iter().map(|d| d.swap(0, Ordering::Relaxed) as f64 / frames).collect();
    tracing::info!(
        "steps done on the card instead of waiting for flights, per frame (VICTORY_CARD_STEPS={}): {:.2} fills, {:.2} buffer \
         copies, {:.1} KB; {:.2} submissions refused and waited; {:.2} of the fills out of the source of a copy at its \
         place (VICTORY_PLACED_FORWARD={})",
        stage(),
        d[0],
        d[1],
        d[2] / 1024.0,
        d[3],
        d[4],
        u8::from(forward_on()),
    );
    let t: Vec<f64> = PLAN_NS.iter().map(|t| t.swap(0, Ordering::Relaxed) as f64 / 1e6 / frames).collect();
    tracing::info!(
        "planning steps done on the card, per frame, ms: {:.2} what the steps do, {:.2} the plan, {:.2} what holds their \
         bytes (VICTORY_HOLDER_INDEX={}), {:.2} the list, {:.2} the slots, {:.2} the checks first, {:.2} where to cut a \
         submission",
        t[0],
        t[1],
        t[2],
        u8::from(holder_index_on()),
        t[3],
        t[4],
        t[5],
        t[6],
    );
    let r: Vec<f64> = RECORDED.iter().map(|r| r.swap(0, Ordering::Relaxed) as f64 / frames).collect();
    tracing::info!(
        "steps on the card, per frame: {:.0} copy commands recorded; {:.0} reads gathered in {:.1} dispatches \
         (VICTORY_CARD_GATHER={}); since the start {} lists of ours made, their buffers {:.1} MB alive ({} pooled)",
        r[0],
        r[1],
        r[2],
        u8::from(gather_on() && gather().is_some()),
        MADE[0].load(Ordering::Relaxed),
        MADE[1].load(Ordering::Relaxed).saturating_sub(MADE[2].load(Ordering::Relaxed)) as f64 / 1048576.0,
        POOL.lock().unwrap_or_else(|e| e.into_inner()).len(),
    );
    #[cfg(feature = "diagnostics")]
    {
        let noted = NOTED.lock().unwrap_or_else(|e| e.into_inner()).take().unwrap_or_default();
        let mut noted: Vec<_> = noted.into_iter().collect();
        noted.sort_unstable_by_key(|(_, e)| std::cmp::Reverse(e.0));
        for ((len, from, to), (n, src, dst)) in noted.into_iter().take(5) {
            let n = n as f64 / frames;
            if to.1 == 0 {
                tracing::info!("done on the card, per frame: {n:.2} fills of {len:#x} bytes from the host copy of {:#x}+{:#x} (last {src:#x})", from.0, from.1);
            } else {
                tracing::info!(
                    "done on the card, per frame: {n:.2} copies of {len:#x} bytes from the host copy of {:#x}+{:#x} (last {src:#x}) \
                     to that of {:#x}+{:#x} (last {dst:#x})",
                    from.0,
                    from.1,
                    to.0,
                    to.1
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy(dst: usize, src: usize, len: usize, asked: bool) -> Does {
        Does::Copy { dst, src, len, asked }
    }

    fn fill(src: usize, len: usize, asked: bool) -> Does {
        Does::Fill { src, len, asked, to: true, title: src }
    }

    fn stuck(src: usize, len: usize) -> Does {
        Does::Fill { src, len, asked: false, to: false, title: src }
    }

    /// A copy at its place out of what shaders wrote.
    fn placed(dst: usize, src: usize, len: usize, free: bool) -> Does {
        Does::Placed { dst, src, len, free, cpu: false, stuck: false }
    }

    /// What is done on the card, as (place, level), in stage 2.
    fn planned(does: &[Does]) -> Vec<(usize, u32)> {
        plan(does, 2, true, &mut None).unwrap()
    }

    /// Only what is asked for is taken, all of it, and a step that stands
    /// in none of their ways stays where it is.
    #[test]
    fn steps_apart_from_one_another_are_taken() {
        let does = [
            copy(1000, 5000, 100, false),
            fill(2000, 64, true),
            Does::Refresh { shadow: 1, from: 0, to: 9000 },
            copy(3000, 6000, 100, true),
            fill(1000, 64, false),
            Does::Read { from: 3100, to: 4000, asked: false },
            stuck(3100, 5),
        ];
        assert_eq!(planned(&does), [(1, 0), (3, 0)]);
        // Two fills of the same bytes.
        assert_eq!(plan(&[fill(0, 64, true), fill(32, 64, true)], 1, true, &mut None), Ok(vec![(0, 0), (1, 0)]));
        assert_eq!(plan(&[fill(0, 64, false)], 1, true, &mut None), Ok(vec![]));
        // Ends that touch are apart.
        let does = [
            copy(1000, 5000, 100, true),
            copy(900, 9000, 100, false),
            copy(1100, 9000, 5, false),
            copy(4900, 9000, 100, false),
        ];
        assert_eq!(planned(&does), [(0, 0)]);
    }

    /// What depends on a taken step, or it on it, is taken too, on the level
    /// the order of the steps asks for.
    #[test]
    fn steps_that_depend_on_one_another_are_taken_in_order() {
        let taken = copy(1000, 5000, 100, true);
        // Reads what it writes, behind and ahead of it.
        assert_eq!(planned(&[taken, fill(1099, 64, false)]), [(0, 0), (1, 1)]);
        assert_eq!(planned(&[fill(1099, 64, false), taken]), [(0, 0), (1, 1)]);
        assert_eq!(planned(&[taken, copy(7000, 1050, 10, false)]), [(0, 0), (1, 1)]);
        // Writes what it reads, and what it writes.
        assert_eq!(planned(&[copy(5099, 9000, 1, false), taken]), [(0, 0), (1, 1)]);
        assert_eq!(planned(&[taken, copy(5099, 9000, 1, false)]), [(0, 0), (1, 1)]);
        assert_eq!(planned(&[taken, copy(999, 9000, 2, false)]), [(0, 0), (1, 1)]);
        // The same list twice.
        assert_eq!(planned(&[taken, taken]), [(0, 0), (1, 1)]);
        // A chain, with steps apart from it in between, and two fills of one
        // copy on one level.
        let does = [
            copy(8000, 7000, 50, false),
            taken,
            copy(2000, 1010, 20, false),
            fill(90_000, 10, false),
            fill(2000, 20, false),
            fill(2010, 20, true),
            copy(1000, 2005, 5, false),
            fill(1000, 100, false),
        ];
        assert_eq!(planned(&does), [(1, 0), (2, 1), (4, 2), (5, 2), (6, 2), (7, 3)]);
        // A fill is in nobody's way.
        assert_eq!(planned(&[taken, fill(5000, 100, false)]), [(0, 0)]);
        // A copy ahead that a taken one only shares its source with.
        assert_eq!(planned(&[copy(7000, 5000, 10, false), taken]), [(1, 0)]);
    }

    /// A copy at its place in a list is in the way of what reads or writes its
    /// bytes behind it, not of what did ahead of it.
    #[test]
    fn copies_at_their_place_stand_in_the_way_of_what_is_behind_them() {
        let placed = placed(2000, 3000, 64, true);
        let behind = |does: &[Does]| {
            let mut found = None;
            let refused = plan(does, 2, true, &mut found);
            assert!(refused.is_ok() || found.is_some() && refused == Err(BEHIND_PLACED));
            assert_eq!(plan(does, 2, false, &mut None).is_err(), does.len() > 1 && !matches!(does, [_, Does::Fill { src: 5000, .. }]));
            found
        };
        assert_eq!(behind(&[fill(2032, 64, true), placed]), None);
        assert_eq!(behind(&[placed, fill(2032, 64, true)]), Some((0, 1)));
        assert_eq!(behind(&[placed, fill(5000, 64, true)]), None);
        assert_eq!(behind(&[fill(2000, 8, true), placed, fill(2000, 8, true)]), Some((1, 2)));
        // One that writes what it copies from, and one that writes over it.
        assert_eq!(behind(&[copy(3000, 5000, 8, true), placed]), None);
        assert_eq!(behind(&[placed, copy(3000, 5000, 8, true)]), Some((0, 1)));
        assert_eq!(behind(&[copy(2060, 5000, 8, true), placed]), None);
        assert_eq!(behind(&[placed, copy(2060, 5000, 8, true)]), Some((0, 1)));
    }

    /// The title's ring: a copy at its place writes 64 bytes of it out of what
    /// shaders wrote, under flights' writes. The draw ahead of it reads the
    /// ring's old bytes in front of the lists; the draw behind it reads what
    /// the copy put, out of the copy's source, instead of the submission
    /// waiting for the flights (run rdna4-tuned-stage2: 14-17 ms a frame).
    #[test]
    fn a_fill_behind_a_copy_at_its_place_reads_what_the_copy_put() {
        let mut does = vec![fill(2000, 64, true), placed(2000, 3000, 64, false), fill(2016, 32, true)];
        // As it was: refused, either way of ordering.
        assert_eq!(plan(&does, 2, false, &mut None), Err(BEHIND_PLACED));
        assert_eq!(plan(&does, 2, true, &mut None), Err(BEHIND_PLACED));
        assert_eq!(forward(&[], &mut does), [2]);
        assert_eq!(does[0], fill(2000, 64, true), "the draw ahead is left alone");
        assert_eq!(does[2], Does::Fill { src: 3016, len: 32, asked: true, to: true, title: 2016 });
        let chosen = plan(&does, 2, true, &mut None).unwrap();
        assert_eq!(chosen, [(0, 0), (2, 0)]);
        // In front of the lists: the ring as it was, the source as shaders
        // left it. What each draw is given, against the order of the steps.
        let mut card: Vec<u8> = (0..4096).map(|i| (i % 251) as u8).collect();
        card[3000..3064].fill(0xab);
        let old = card[2000..2064].to_vec();
        let read = |i: usize| {
            let ((from, to), _) = reads_writes(&does[i]).unwrap();
            card[from..to].to_vec()
        };
        assert_eq!(read(0), old, "the draw ahead of the copy sees the old bytes");
        assert_eq!(read(2), [0xab; 32], "the draw behind it sees the new ones");
        // A draw behind that is not the copy's alone is not moved, and
        // refused as before.
        let mut does = vec![placed(2000, 3000, 64, false), fill(2048, 32, true)];
        assert!(forward(&[], &mut does).is_empty());
        assert_eq!(plan(&does, 2, true, &mut None), Err(BEHIND_PLACED));
    }

    /// What lies between the copy and the fill decides whether the fill can
    /// read the copy's source.
    #[test]
    fn fills_are_moved_only_where_the_source_still_holds_the_bytes() {
        let moved = |does: &[Does]| {
            let mut does = does.to_vec();
            let at = forward(&[], &mut does);
            (at, does)
        };
        // A copy that writes the source between them, and one that writes
        // the fill's bytes over the copy's: not moved.
        let (at, _) = moved(&[placed(2000, 3000, 64, true), copy(3010, 9000, 4, false), fill(2008, 8, true)]);
        assert!(at.is_empty());
        let (at, _) = moved(&[placed(2000, 3000, 64, true), copy(2008, 9000, 4, false), fill(2008, 8, true)]);
        assert!(at.is_empty());
        // ...but one ahead of the copy is no matter: the plan orders it.
        let (at, does) = moved(&[copy(3010, 9000, 4, true), placed(2000, 3000, 64, true), fill(2008, 8, false)]);
        assert_eq!(at, [2]);
        assert_eq!(planned(&does), [(0, 0), (2, 1)]);
        // On through two copies at their place; a copy onto itself, not.
        let (at, does) = moved(&[placed(3000, 5000, 64, true), placed(2000, 3000, 64, true), fill(2008, 8, true)]);
        assert_eq!(at, [2]);
        assert_eq!(does[2], Does::Fill { src: 5008, len: 8, asked: true, to: true, title: 2008 });
        let (at, _) = moved(&[placed(2000, 2032, 64, true), fill(2008, 8, true)]);
        assert!(at.is_empty());
        // Behind one that title memory does not take, the fill is the card's
        // even if nothing asked for it.
        let (_, does) = moved(&[placed(2000, 3000, 64, false), fill(2008, 8, false)]);
        assert_eq!(does[1], Does::Fill { src: 3008, len: 8, asked: true, to: true, title: 2008 });
        // A step that runs ends it.
        let (at, _) = moved(&[placed(2000, 3000, 64, true), Does::Run, fill(2008, 8, true)]);
        assert!(at.is_empty());
    }

    /// What the CPU would read stale behind a copy at its place is refused.
    #[test]
    fn steps_left_on_the_cpu_behind_a_copy_only_the_card_does_are_refused() {
        let ask = fill(9000, 4, true);
        // Title memory does not take the copy: a texture, a fill that cannot
        // be taken, or a copy left on the CPU reads its destination behind it.
        for read in [Does::Read { from: 2010, to: 2012, asked: false }, stuck(2010, 2), copy(7000, 2010, 2, false)] {
            assert_eq!(plan(&[ask, placed(2000, 3000, 64, false), read], 2, true, &mut None), Err(PLACED_STALE), "{read:?}");
            assert!(plan(&[ask, placed(2000, 3000, 64, true), read], 2, true, &mut None).is_ok(), "{read:?}");
            assert!(plan(&[read, ask, placed(2000, 3000, 64, false)], 2, true, &mut None).is_ok(), "{read:?} ahead");
        }
        // A copy whose source the CPU takes: of bytes flights write, or that
        // a step taken by the card writes ahead of it.
        let on_cpu = |stuck| Does::Placed { dst: 2000, src: 3000, len: 64, free: true, cpu: true, stuck };
        assert_eq!(plan(&[ask, on_cpu(true)], 2, true, &mut None), Err(PLACED_ON_CPU));
        assert_eq!(plan(&[copy(3010, 9000, 4, true), on_cpu(false)], 2, true, &mut None), Err(PLACED_ON_CPU));
        assert!(plan(&[ask, on_cpu(false)], 2, true, &mut None).is_ok());
    }

    /// Copies at their place among the rest: whatever is done on the card
    /// (fills moved to a copy's source among them) or left on the CPU, every
    /// fill, copy and read sees what it would one step after the other.
    /// Title memory takes the CPU's copies and the free copies at their
    /// place; in front of the lists the card holds the CPU's copies but not
    /// those at their place (they come in the lists, behind).
    #[test]
    fn copies_at_their_place_and_moved_fills_do_what_the_order_does() {
        let mut seed = 777u32;
        let mut random = move |n: u32| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (seed >> 16) % n
        };
        let (mut done, mut moved_done, mut rescued) = (0, 0, 0);
        for _ in 0..20_000 {
            let does: Vec<Does> = (0..2 + random(9))
                .map(|_| {
                    let (at, other, len) = (random(60) as usize, random(60) as usize, 1 + random(12) as usize);
                    let asked = random(3) == 0;
                    match random(10) {
                        0..2 => copy(at, other, len, asked),
                        2..5 => fill(at, len, asked),
                        5 => stuck(at, len),
                        6 => Does::Read { from: at, to: at + len, asked: false },
                        _ => {
                            let cpu = random(4) == 0;
                            Does::Placed { dst: at, src: other, len: len * 2, free: random(2) == 0, cpu, stuck: false }
                        }
                    }
                })
                .collect();
            let before = plan(&does, 2, true, &mut None);
            let mut forwarded = does.clone();
            let moved = forward(&[], &mut forwarded);
            let Ok(chosen) = plan(&forwarded, 2, true, &mut None) else {
                continue;
            };
            done += 1;
            if before == Err(BEHIND_PLACED) {
                rescued += 1;
            }
            let taken = |i: usize| chosen.iter().any(|c| c.0 == i);
            moved_done += moved.iter().filter(|&&i| taken(i)).count();
            let start: Vec<u32> = (0..100).collect();
            // One step after the other.
            let mut memory = start.clone();
            let mut wanted = Vec::new();
            for d in &does {
                let (from, to) = match *d {
                    Does::Read { from, to, .. } => (from, to),
                    Does::Placed { src, len, .. } => (src, src + len),
                    _ => reads_writes(d).unwrap().0,
                };
                wanted.push(memory[from..to].to_vec());
                match *d {
                    Does::Copy { dst, src, len, .. } | Does::Placed { dst, src, len, .. } => {
                        memory.copy_within(src..src + len, dst);
                    }
                    _ => {}
                }
            }
            // The CPU's steps in title memory, and what the host copies hold
            // in front of the lists.
            let (mut title, mut card) = (start.clone(), start.clone());
            for (i, d) in does.iter().enumerate().filter(|&(i, _)| !taken(i)) {
                let here = |from: usize, to: usize| &title[from..to] == &wanted[i][..];
                match *d {
                    Does::Copy { dst, src, len, .. } => {
                        assert!(here(src, src + len), "CPU copy {i} of {does:?}: {chosen:?}");
                        title.copy_within(src..src + len, dst);
                        card[dst..dst + len].copy_from_slice(&title[dst..dst + len]);
                    }
                    Does::Fill { title: at, len, .. } => assert!(here(at, at + len), "CPU fill {i} of {does:?}: {chosen:?}"),
                    Does::Read { from, to, .. } => assert!(here(from, to), "CPU read {i} of {does:?}: {chosen:?}"),
                    Does::Placed { dst, src, len, free, cpu, .. } => {
                        if cpu {
                            assert!(here(src, src + len), "source on the CPU {i} of {does:?}: {chosen:?}");
                        }
                        if free {
                            title.copy_within(src..src + len, dst);
                        }
                    }
                    _ => {}
                }
            }
            for level in 0..=chosen.iter().map(|c| c.1).max().unwrap_or(0) {
                let ahead = card.clone();
                for &(i, _) in chosen.iter().filter(|c| c.1 == level) {
                    let ((from, to), (dst, _)) = reads_writes(&forwarded[i]).unwrap();
                    assert_eq!(ahead[from..to], wanted[i][..], "card step {i} of {does:?} as {forwarded:?}: {chosen:?}");
                    if matches!(forwarded[i], Does::Copy { .. }) {
                        card[dst..dst + to - from].copy_from_slice(&ahead[from..to]);
                    }
                }
            }
        }
        assert!(done > 2000 && rescued > 100 && moved_done > 200, "{done} {rescued} {moved_done}");
    }

    /// Whatever the steps, done level by level (in a level every read
    /// before every write) they read and leave what they read and leave one
    /// after the other, the CPU's steps ahead of all of them.
    #[test]
    fn levels_do_what_the_order_does() {
        let mut seed = 12345u32;
        let mut random = move |n: u32| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (seed >> 16) % n
        };
        let (mut done, mut refused) = (0, 0);
        for _ in 0..4000 {
            let does: Vec<Does> = (0..2 + random(9))
                .map(|_| {
                    let (at, other, len) = (random(60) as usize, random(60) as usize, 1 + random(8) as usize);
                    let asked = random(4) == 0;
                    match random(8) {
                        0..4 => copy(at, other, len, asked),
                        4..6 => fill(at, len, asked),
                        6 => stuck(at, len),
                        _ => Does::Read { from: at, to: at + len, asked: false },
                    }
                })
                .collect();
            let Ok(chosen) = plan(&does, 2, true, &mut None) else {
                refused += 1;
                continue;
            };
            done += 1;
            let start: Vec<u32> = (0..80).collect();
            // One after the other: what each reads, and what is left.
            let mut memory = start.clone();
            let mut wanted = Vec::new();
            for d in &does {
                let (from, to) = match *d {
                    Does::Read { from, to, .. } => (from, to),
                    _ => reads_writes(d).unwrap().0,
                };
                wanted.push(memory[from..to].to_vec());
                if let Does::Copy { dst, src, len, .. } = *d {
                    memory.copy_within(src..src + len, dst);
                }
            }
            // The CPU's, then the card's.
            let mut cpu = start.clone();
            let taken = |i: usize| chosen.iter().any(|c| c.0 == i);
            for (i, d) in does.iter().enumerate().filter(|&(i, _)| !taken(i)) {
                let (from, to) = match *d {
                    Does::Read { from, to, .. } => (from, to),
                    _ => reads_writes(d).unwrap().0,
                };
                assert_eq!(cpu[from..to], wanted[i][..], "step {i} of {does:?}");
                if let Does::Copy { dst, src, len, .. } = *d {
                    cpu.copy_within(src..src + len, dst);
                }
            }
            let mut card = cpu.clone();
            for level in 0..=chosen.iter().map(|c| c.1).max().unwrap_or(0) {
                let of: Vec<usize> = chosen.iter().filter(|c| c.1 == level).map(|c| c.0).collect();
                let before = card.clone();
                for i in of {
                    let ((from, to), (dst, _)) = reads_writes(&does[i]).unwrap();
                    assert_eq!(before[from..to], wanted[i][..], "step {i} of {does:?}: {chosen:?}");
                    if matches!(does[i], Does::Copy { .. }) {
                        card[dst..dst + to - from].copy_from_slice(&before[from..to]);
                    }
                }
            }
            assert_eq!(card, memory, "{does:?}: {chosen:?}");
        }
        assert!(done > 500 && refused > 100, "{done} {refused}");
    }

    /// What cannot be done on the card refuses the lot, where it stands in
    /// the way.
    #[test]
    fn steps_in_the_way_are_refused() {
        let taken = copy(1000, 5000, 100, true);
        assert!(plan(&[taken, Does::Run], 2, true, &mut None).is_err());
        // A texture that itself waits for flights; a fill that cannot be
        // taken but waits.
        assert!(plan(&[taken, Does::Read { from: 1, to: 2, asked: true }], 2, true, &mut None).is_err());
        assert!(plan(&[Does::Fill { src: 1, len: 1, asked: true, to: false, title: 1 }], 2, true, &mut None).is_err());
        // Reads on the CPU of what it writes: behind it, not ahead of it.
        for read in [Does::Read { from: 900, to: 1001, asked: false }, stuck(1050, 3)] {
            assert!(plan(&[taken, read], 2, true, &mut None).is_err(), "{read:?} behind");
            assert_eq!(planned(&[read, taken]), [(1, 0)], "{read:?} ahead");
        }
        // Also of what is taken along.
        assert!(plan(&[taken, copy(3000, 1000, 10, false), stuck(3005, 1)], 2, true, &mut None).is_err());
        // Into itself, and copies without stage 2.
        assert!(plan(&[copy(1000, 1050, 100, true)], 2, true, &mut None).is_err());
        assert!(plan(&[taken], 1, true, &mut None).is_err());
        assert!(plan(&[fill(1000, 4, true), copy(1000, 5000, 100, false)], 1, true, &mut None).is_err());
    }

    #[test]
    fn a_refresh_of_another_host_copy_is_seen() {
        let does = [Does::Refresh { shadow: 7, from: 100, to: 200 }];
        assert!(!elsewhere(&does, 7, 100, 200));
        assert!(elsewhere(&does, 8, 199, 300));
        assert!(!elsewhere(&does, 8, 200, 300));
    }

    /// A value written behind a flight, or a read-back, is in no host copy
    /// before it lands.
    #[test]
    fn what_no_host_copy_holds_is_refused() {
        let writers = [Writer { shadow: None, from: 100, to: 108, copy: false }];
        assert!(holder(&writers, 104, 120).is_err());
    }

    /// Reads are all before writes, between two buffers and within one, and
    /// the buffers are usable in the lists that follow in the same call.
    #[test]
    fn the_card_copies_what_is_recorded() {
        let Some(host) = host() else {
            return; // no host GPU
        };
        let (a, a_data) = host.shared_buffer(1 << 16).unwrap();
        let (b, b_data) = host.shared_buffer(1 << 16).unwrap();
        // SAFETY: mappings of 64 KiB.
        let (a_bytes, b_bytes) =
            unsafe { (std::slice::from_raw_parts_mut(a_data, 1 << 16), std::slice::from_raw_parts_mut(b_data, 1 << 16)) };
        for (i, (x, y)) in a_bytes.iter_mut().zip(b_bytes.iter_mut()).enumerate() {
            (*x, *y) = (i as u8 ^ (i >> 8) as u8, 0xee);
        }
        let was = a_bytes.to_vec();
        let own = host.own_list(BETWEEN).unwrap();
        // Within `a` (there and back), from `a` to `b`, and a read alone.
        record(
            true,
            &own.list,
            &own.between,
            &[(&a, 33, 0, 100), (&a, 2000, 256, 100), (&a, 5001, 512, 7), (&a, 9, 1024, 4)],
            &[(&a, 2000, 0, 100), (&a, 33, 256, 100), (&b, 77, 512, 7)],
        );
        // A level behind it, of what it wrote.
        record(false, &own.list, &own.between, &[(&b, 77, 2048, 7)], &[(&b, 1077, 2048, 7)]);
        own.list.close();
        // A list behind it that uses the buffers in other ways.
        let next = host.new_list().unwrap();
        let (out, out_data) = host.shared_buffer(1 << 16).unwrap();
        // SAFETY: an open list; live buffers.
        unsafe {
            next.list.CopyBufferRegion(&out, 0, &own.between, 1024, 4);
            next.list.CopyBufferRegion(&b, 4096, &a, 33, 100);
        }
        next.close();
        host.execute(&[&own.list, &next]);
        // With the debug layer (`VICTORY_D3D12_DEBUG=1`, diagnostics).
        assert_eq!(host.log_messages(), None);
        assert_eq!(a_bytes[2000..2100], was[33..133]);
        assert_eq!(a_bytes[33..133], was[2000..2100]);
        assert_eq!(b_bytes[77..84], was[5001..5008]);
        assert_eq!(b_bytes[1077..1084], was[5001..5008]);
        assert_eq!(b_bytes[4096..4196], was[2000..2100]);
        // SAFETY: a mapping of 64 KiB.
        assert_eq!(unsafe { std::slice::from_raw_parts(out_data, 4) }, &was[9..13]);
        assert!(b_bytes[..77].iter().all(|&x| x == 0xee));
        a_bytes[2000..2100].copy_from_slice(&was[2000..2100]);
        a_bytes[33..133].copy_from_slice(&was[33..133]);
        assert_eq!(a_bytes, &was[..]);
        // As the landing does: its next use records anew.
        POOL.lock().unwrap().push(own);
        let own = host.own_list(BETWEEN).unwrap();
        own.list.close();
    }

    /// The index by address finds the same writers as the scan of all of
    /// them (`VICTORY_HOLDER_INDEX`): with writers that hold no host copy,
    /// any that meets the range answers with that, else nothing holds it.
    #[test]
    fn the_holder_index_finds_what_the_scan_finds() {
        let mut seed = 0x9e37_79b9_u64;
        let mut next = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        for _ in 0..50 {
            let writers: Vec<Writer> = (0..40)
                .map(|_| {
                    let from = next(10_000);
                    Writer { shadow: None, from, to: from + 1 + next(300), copy: false }
                })
                .collect();
            let copy = |w: &Writer| Writer { shadow: None, ..*w };
            let mut index = Holders::new(writers.iter().map(copy).collect());
            let mut all: Vec<Writer> = writers.iter().map(copy).collect();
            let extra = Writer { shadow: None, from: 20_000, to: 20_010, copy: true };
            index.push(copy(&extra));
            all.push(extra);
            for _ in 0..200 {
                let from = next(20_100);
                let to = from + 1 + next(200);
                assert_eq!(index.holder(from, to).err(), holder(&all, from, to).err(), "[{from}, {to})");
            }
        }
    }

    /// Buffers between are sized to what a submission needs, a pooled one is
    /// taken only if it holds that, and the pool keeps its largest.
    #[test]
    fn buffers_between_fit_and_the_pool_is_bounded() {
        assert_eq!(between_size(0), BETWEEN_LEAST);
        assert_eq!(between_size(110 << 10), 128 << 10);
        assert_eq!(between_size((128 << 10) + 1), 256 << 10);
        assert_eq!(between_size(BETWEEN), BETWEEN);
        let Some(host) = host() else {
            return; // no host GPU
        };
        // (The other tests share the pool: by sizes of their own.)
        let small = host.own_list(1000).unwrap();
        assert_eq!(small.size, BETWEEN_LEAST);
        small.list.close();
        pool(small);
        let big = host.own_list(3 << 20).unwrap();
        assert_eq!(big.size, BETWEEN);
        big.list.close();
        let held = POOL.lock().unwrap().len();
        pool(big);
        assert!(POOL.lock().unwrap().len() <= POOLED.max(held + 1));
        // A need the small one does not hold takes no small one.
        let again = host.own_list((2 << 20) + 1).unwrap();
        assert!(again.size > (2 << 20));
        again.list.close();
        pool(again);
        for _ in 0..POOLED + 4 {
            let own = host.own_list(1).unwrap();
            own.list.close();
            let more = host.own_list(1).unwrap();
            more.list.close();
            pool(own);
            pool(more);
        }
        assert!(POOL.lock().unwrap().len() <= POOLED);
    }

    /// Many reads out of one buffer are gathered by a dispatch, a few out of
    /// another copied, and a level behind gathers what the first wrote: the
    /// bytes are where copies would have put them.
    #[test]
    fn the_card_gathers_what_is_recorded() {
        let Some(host) = host() else {
            return; // no host GPU
        };
        assert!(gather().is_some(), "the gather was not made");
        const LEN: usize = 1 << 16;
        let (a, a_data) = host.shared_buffer(LEN as u64).unwrap();
        let (b, b_data) = host.shared_buffer(LEN as u64).unwrap();
        let (c, c_data) = host.shared_buffer(LEN as u64).unwrap();
        // SAFETY: mappings of 64 KiB.
        let (a_bytes, b_bytes, c_bytes) = unsafe {
            (
                std::slice::from_raw_parts_mut(a_data, LEN),
                std::slice::from_raw_parts_mut(b_data, LEN),
                std::slice::from_raw_parts_mut(c_data, LEN),
            )
        };
        for (i, x) in a_bytes.iter_mut().enumerate() {
            *x = (i * 7 + i / 256) as u8;
        }
        b_bytes.fill(0xee);
        c_bytes.fill(0x11);
        let was = a_bytes.to_vec();
        // 40 reads of `a` (lengths as the frame's fills: 0x1e0 to 0x10), into
        // places 256 bytes apart, then out to `b`; two of `c`, copied.
        let lens = [0x1e0u64, 0x290, 0xc0, 0x70, 0x20, 0x50, 0x30, 0x10];
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        let mut at = 0u64;
        for i in 0..40u64 {
            let len = lens[i as usize % lens.len()];
            reads.push((&a, 100 * 16 + i * 1040, at, len));
            writes.push((&b, i * 1100, at, len));
            at = (at + len).next_multiple_of(256);
        }
        reads.push((&c, 4, at, 8));
        reads.push((&c, 3, at + 256, 5));
        writes.push((&b, 60_000, at, 8));
        writes.push((&b, 61_000, at + 256, 5));
        let level = at + 512;
        assert_eq!(gathered(&reads).len(), 1, "only `a` is gathered");
        let before = RECORDED[2].load(Ordering::Relaxed);
        let own = host.own_list(BETWEEN).unwrap();
        record(true, &own.list, &own.between, &reads, &writes);
        // A level behind it: eight reads of what it wrote, out to `c`.
        let behind: Vec<_> = (0..8u64).map(|i| (&b, i * 1100, level + i * 512, 0x40)).collect();
        let out: Vec<_> = (0..8u64).map(|i| (&c, 30_000 + i * 100, level + i * 512, 0x40)).collect();
        record(false, &own.list, &own.between, &behind, &out);
        own.list.close();
        host.execute(&[&own.list]);
        assert_eq!(host.log_messages(), None);
        assert!(RECORDED[2].load(Ordering::Relaxed) >= before + 2, "both levels gathered");
        for i in 0..40usize {
            let len = lens[i % lens.len()] as usize;
            let from = 100 * 16 + i * 1040;
            assert_eq!(b_bytes[i * 1100..i * 1100 + len], was[from..from + len], "read {i}");
        }
        assert_eq!(b_bytes[60_000..60_008], [0x11; 8]);
        assert_eq!(b_bytes[61_000..61_005], [0x11; 5]);
        for i in 0..8usize {
            let len = (lens[i % lens.len()] as usize).min(0x40);
            let from = 100 * 16 + i * 1040;
            assert_eq!(c_bytes[30_000 + i * 100..30_000 + i * 100 + len], was[from..from + len], "level 2, read {i}");
        }
        POOL.lock().unwrap().push(own);
        let own = host.own_list(BETWEEN).unwrap();
        own.list.close();
    }

    /// The whole way, in a process of its own with the relay on:
    /// `VICTORY_RELAY=2 VICTORY_CARD_STEPS=2 cargo test -p victory-gpu --lib
    /// card_steps -- --ignored --test-threads=1`. A flight's list writes
    /// part of what the next submission's copy reads, the title wrote the
    /// rest; the copy is done on the card behind the flight, and a second
    /// one reads what the first wrote. Title memory ends as on the CPU.
    #[test]
    #[ignore = "wants VICTORY_RELAY=2 VICTORY_CARD_STEPS=2 and a process of its own"]
    fn a_copy_behind_a_flight_reads_what_the_flight_wrote() {
        use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc};
        const LEN: usize = 4 << 20;
        // Without it (`TEST_UNWATCHED=1`) the title's bytes cannot be put in
        // under a flight's writes: the flights that declared them are waited
        // for.
        let write_watch = if std::env::var_os("TEST_UNWATCHED").is_some() { 0 } else { 0x20_0000 };
        let Some(host) = host() else {
            return;
        };
        assert!(relay_exact() && stage() == 2, "set VICTORY_RELAY=2 VICTORY_CARD_STEPS=2");
        // Two title buffers: in the first a copy within it, from it into the
        // second, and back.
        let (title, start) = {
            // SAFETY: memory of our own, never freed.
            let base = unsafe { VirtualAlloc(std::ptr::null(), LEN, MEM_RESERVE | MEM_COMMIT | write_watch, PAGE_READWRITE) }.cast::<u8>();
            assert!(!base.is_null());
            // SAFETY: `LEN` bytes just committed.
            (unsafe { std::slice::from_raw_parts_mut(base, LEN) }, base as usize)
        };
        const HALF: usize = LEN / 2;
        crate::shadow::register(0x1111, start as u64, HALF as u64);
        crate::shadow::register(0x2222, (start + HALF) as u64, HALF as u64);
        let first = crate::shadow::find(start as u64, 64).unwrap();
        let (slow_from, slow_to) = (host.uav_buffer(64 << 20).unwrap(), host.uav_buffer(64 << 20).unwrap());
        let before = DONE[1].load(Ordering::Relaxed);
        let mut lists = Vec::new();
        for round in 0..8usize {
            // The title writes; then a "shader" of a flight writes 300 bytes
            // of what the copy reads, behind some work.
            let seed = |i: usize| (i * 7 + round * 13 + i / 251) as u8;
            for (i, b) in title[4000..6000].iter_mut().enumerate() {
                *b = seed(i);
            }
            let (upload, data) = host.upload_buffer(4096).unwrap();
            for i in 0..300 {
                // SAFETY: inside the mapping.
                unsafe { data.add(i).write(0x80 | (round * 31 + i) as u8) };
            }
            let a = host.new_list().unwrap();
            first.refresh(&a, start as u64, HALF as u64);
            first.gpu_writes(&a, start as u64, HALF as u64);
            // SAFETY: an open list; live buffers.
            unsafe {
                for _ in 0..24 {
                    a.list.CopyBufferRegion(&slow_to, 0, &slow_from, 0, 64 << 20);
                }
                a.list.CopyBufferRegion(&first.buffer, 4500, &upload, 0, 300);
                a.list
                    .ResourceBarrier(&[transition(&first.buffer, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_COMMON)]);
            }
            a.close();
            host.execute_own(&a);
            // With the flight in the air, beside what it writes.
            for (i, b) in title[5300..5400].iter_mut().enumerate() {
                *b = seed(1300 + i) ^ 0x5c;
            }
            // Thousands of runs of bytes of its own.
            for b in title[1_700_000..1_708_000].iter_mut().step_by(2) {
                *b = 1 + round as u8;
            }
            // Into the second buffer, and within the first.
            let b = host.new_list().unwrap();
            b.defer_copy((start + HALF + 100) as *mut u8, (start + 4001) as *const u8, 1500);
            b.defer_copy((start + 900_001) as *mut u8, (start + 4400) as *const u8, 777);
            b.defer_copy((start + HALF + 10_000) as *mut u8, (start + 1_700_000) as *const u8, 8000);
            // What depends on them: a copy of the first copy, upload memory
            // filled from that, and the first one's bytes copied over.
            b.defer_copy((start + 1_500_000) as *mut u8, (start + HALF + 600) as *const u8, 700);
            let (staging, staged) = b.upload(host, 4096).unwrap();
            b.defer_fill_buffer(&staging, staged, (start + 1_500_100) as *const u8, 500);
            b.defer_copy((start + HALF + 300) as *mut u8, (start + 8000) as *const u8, 64);
            b.close();
            host.execute_own(&b);
            // What the first wrote, read by the next submission.
            let c = host.new_list().unwrap();
            c.defer_copy((start + 1_200_000) as *mut u8, (start + HALF + 100) as *const u8, 1500);
            c.close();
            let number = host.execute_own(&c);
            settle_now(WHY_SYNC);
            assert!(FINISHED.load(Ordering::Acquire) >= number);
            let wanted: Vec<u8> = (0..2000)
                .map(|i| match i {
                    500..800 => 0x80 | (round * 31 + i - 500) as u8,
                    1300..1400 => seed(i) ^ 0x5c,
                    _ => seed(i),
                })
                .collect();
            assert!(title[4000..6000] == wanted[..], "what the flight wrote, round {round}");
            // (The copy over the first one's bytes came last.)
            let mut over = wanted[1..1501].to_vec();
            over[200..264].fill(0);
            assert!(title[HALF + 100..HALF + 1600] == over[..], "the copy to the other buffer, round {round}");
            assert!(title[1_500_000..1_500_700] == wanted[501..1201], "the copy behind a copy, round {round}");
            // SAFETY: the mapping, which the flight wrote and has landed.
            let staged = unsafe { std::slice::from_raw_parts(staged, 500) };
            assert!(staged == &wanted[601..1101], "the fill behind it, round {round}");
            assert!(title[900_001..900_778] == wanted[400..1177], "the copy within the buffer, round {round}");
            assert!(title[1_200_000..1_201_500] == over[..], "the copy of the copy, round {round}");
            assert!(title[HALF..HALF + 100].iter().all(|&x| x == 0));
            assert!(title[HALF + 1600..HALF + 10_000].iter().all(|&x| x == 0));
            assert!(title[HALF + 18_000..].iter().all(|&x| x == 0));
            let runs = &title[HALF + 10_000..HALF + 18_000];
            assert!(runs.chunks(2).all(|b| b == [1 + round as u8, 0]), "the runs, round {round}");
            a.keep(&upload);
            lists.extend([a, b, c]);
        }
        assert_eq!(host.log_messages(), None);
        let done = DONE[1].load(Ordering::Relaxed) - before;
        eprintln!("{done} of 48 copies were done on the card, {} submissions waited", DONE[3].load(Ordering::Relaxed));
        eprintln!("settles for title memory under shader writes: {}", LATE_STATS[WHY_STALE].load(Ordering::Relaxed));
        assert!(done >= 20, "only {done} copies were done on the card");
    }
}
