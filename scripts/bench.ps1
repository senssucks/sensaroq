# Runs the game a few times with different switches, a set time each, and compares the runs by the numbers in
# their logs (scripts/log_summary.py). Load a save at a fixed spot before you start: each run continues the last
# save, so leave the character standing still there (or the same short walk each time) for fair numbers.
#
#   .\scripts\bench.ps1 -Exe 'E:\Wabbleshot\game_scarlett_master.exe'
#   .\scripts\bench.ps1 -Exe ... -Minutes 6 -Variants '', 'VICTORY_WIDE_SPIN=50', 'VICTORY_TEXTURE_STAGE_MS=50'
#
# Each variant is a list of switches, NAME=value separated by ';', put over the launcher's (play-foxhound.ps1 -More).
# VICTORY_THIN=1 records the title's list calls on eight threads of ours instead of the title's own (findings.md:
# 75 -> 68 ms a frame on an i9), but washed the scene out on an RTX 5060 Ti in another fork (avatarmanz#112): look at
# the picture during that run.
# VICTORY_GAPS_KNOWN=0 turns off 6d40795 (refreshes around shader-written ranges known to copy nothing), to see what it saved.
# '' is the launcher as it is. The first minute in the world is left out of the numbers (shaders, streaming).
# Results go to bench-<time>.txt in the logs folder (%LOCALAPPDATA%\victory\logs) and on screen.
param(
    [Parameter(Mandatory)][string]$Exe,
    [double]$Minutes = 5,
    [string[]]$Variants = @('', 'VICTORY_THIN=1', 'VICTORY_GAPS_KNOWN=0', 'VICTORY_PIPELINE_WAIT_MS=500', 'VICTORY_PIPELINE_WORKERS=4', 'VICTORY_WIDE_SPIN=50', 'VICTORY_TEXTURE_STAGE_MS=50'),
    [string[]]$PlayArgs = @()
)
$play = Join-Path $PSScriptRoot 'play-foxhound.ps1'
$summary = Join-Path $PSScriptRoot 'log_summary.py'
$logs = Join-Path $env:LOCALAPPDATA 'victory\logs'
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { $python = Get-Command py -ErrorAction SilentlyContinue }
$made = @()
foreach ($variant in $Variants) {
    $pairs = @($variant -split ';' | Where-Object { $_ -match '=' } | ForEach-Object {
        $k, $v = $_ -split '=', 2
        "$($k.Trim())='$($v.Trim() -replace "'", "''")'"
    })
    $more = '@{' + ($pairs -join '; ') + '}'
    $name = if ($variant) { $variant } else { '(launcher as it is)' }
    Write-Host "== $name, $Minutes min"
    $before = Get-Date
    $command = "& '$($play -replace "'", "''")' -Exe '$($Exe -replace "'", "''")' -More $more $($PlayArgs -join ' ')"
    $shell = (Get-Process -Id $PID).Path
    $run = Start-Process $shell -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', $command -PassThru
    Start-Sleep -Seconds ([int]($Minutes * 60))
    Get-Process victory -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -ge $before } | Stop-Process -Force
    Start-Sleep -Seconds 3
    if (-not $run.HasExited) { Stop-Process -Id $run.Id -Force -ErrorAction SilentlyContinue }
    $log = Get-ChildItem $logs -Filter 'foxhound-*.log' | Where-Object { $_.LastWriteTime -ge $before } |
        Sort-Object LastWriteTime | Select-Object -Last 1
    if ($log) { $made += $log.FullName; Write-Host "   log: $($log.FullName)" } else { Write-Host '   no log found' }
    Start-Sleep -Seconds 10   # let the PC settle (files closed, memory given back)
}
if (-not $made) { return }
$out = Join-Path $logs ('bench-{0:yyyyMMdd-HHmmss}.txt' -f (Get-Date))
if ($python) {
    & $python.Source $summary --no-switches @made | Tee-Object -FilePath $out
    Write-Host "Saved: $out"
} else {
    Write-Host 'No Python to sum the logs up with; send these logs instead:'
    $made | ForEach-Object { Write-Host "   $_" }
}
