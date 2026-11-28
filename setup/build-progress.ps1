# Runs cargo build for victory and shows a filling progress bar with an estimated time left.
# cargo's own output goes to the log file, as before.
param(
  [Parameter(Mandatory)] [string]$Repo,
  [Parameter(Mandatory)] [string]$Log
)
$ErrorActionPreference = 'Continue'
Set-Location $Repo
[Console]::OutputEncoding = [Text.Encoding]::UTF8

# Downloads first (cargo shows nothing useful for these when its output is in a file)
Write-Host 'Getting the source packages...'
& cargo fetch --locked 2>&1 | Out-Null

# How many packages will be compiled: the dependency tree of victory, each package once
$total = 0
try {
  $names = & cargo tree --locked -p victory -e normal,build --prefix none 2>$null |
    ForEach-Object { ($_ -replace '\s*\(.*\)$', '').Trim() } |
    Where-Object { $_ } | Sort-Object -Unique
  $total = @($names).Count
} catch {}
if ($total -lt 10) {
  $total = [int]((Select-String -Path (Join-Path $Repo 'Cargo.lock') -Pattern '^\[\[package\]\]').Count * 0.7)
}

# The time of the last full build, when there was one, makes the estimate better
$timeFile = Join-Path $Repo 'target\build-seconds.txt'
$lastSeconds = 0
if (Test-Path $timeFile) { [int]::TryParse((Get-Content $timeFile -Raw).Trim(), [ref]$lastSeconds) | Out-Null }

if (Test-Path $Log) { Remove-Item $Log -Force }
$p = Start-Process -FilePath 'cargo' -ArgumentList 'build','--locked','--release','-p','victory' `
  -RedirectStandardError $Log -NoNewWindow -PassThru
$null = $p.Handle
$start = Get-Date
$width = 40
$full = [string][char]0x2588
$empty = [string][char]0x2591
$shown = 0.0
$lastStart = $null   # when the final program (victory) started compiling

function Read-Log {
  try {
    $fs = [IO.File]::Open($Log, 'Open', 'Read', 'ReadWrite')
    $sr = New-Object IO.StreamReader($fs)
    $t = $sr.ReadToEnd(); $sr.Close(); return $t
  } catch { return '' }
}

while (-not $p.HasExited) {
  Start-Sleep -Milliseconds 500
  $text = Read-Log
  $lines = $text -split "`r?`n"
  $compiling = @($lines | Where-Object { $_ -match '^\s+Compiling ' })
  $done = $compiling.Count
  $current = ''
  if ($done -gt 0) { $current = ($compiling[-1] -replace '^\s+Compiling\s+', '' -replace '\s+v\d.*$', '') }
  $elapsed = ((Get-Date) - $start).TotalSeconds

  # Packages make up 85%; the final program (big, optimized and linked last) the other 15%
  if ($current -eq 'victory') {
    if (-not $lastStart) { $lastStart = Get-Date }
    $before = ($lastStart - $start).TotalSeconds
    $expect = [Math]::Max(60, $before * 0.35)
    if ($lastSeconds -gt 0) { $expect = [Math]::Max(60, $lastSeconds - $before) }
    $inLast = ((Get-Date) - $lastStart).TotalSeconds
    $frac = 0.85 + 0.14 * [Math]::Min(1.0, $inLast / $expect)
    $current = 'victory (the main program - the longest step)'
  } else {
    $frac = 0.85 * [Math]::Min(1.0, $done / [Math]::Max(1, $total))
  }
  if ($lastSeconds -gt 0) { $frac = [Math]::Max($frac, [Math]::Min(0.99, $elapsed / $lastSeconds) * 0.5 + $frac * 0.5) }
  $frac = [Math]::Min(0.99, $frac)
  if ($frac -lt $shown) { $frac = $shown }   # never moves back
  $shown = $frac

  $left = ''
  if ($frac -gt 0.02) {
    $rest = $elapsed / $frac * (1 - $frac)
    $left = '{0:mm\:ss}' -f [TimeSpan]::FromSeconds($rest)
    if ($rest -ge 3600) { $left = '{0:h\:mm\:ss}' -f [TimeSpan]::FromSeconds($rest) }
  } else { $left = '...' }
  $fill = [int][Math]::Round($frac * $width)
  $bar = ($full * $fill) + ($empty * ($width - $fill))
  $pct = [int]($frac * 100)
  $el = '{0:mm\:ss}' -f [TimeSpan]::FromSeconds($elapsed)
  if ($current.Length -gt 34) { $current = $current.Substring(0, 34) }
  $line = " [$bar] $pct%  passed $el  left ~$left  $current"
  Write-Host ("`r" + $line.PadRight([Math]::Max(10, [Console]::WindowWidth - 1))) -NoNewline
  $Host.UI.RawUI.WindowTitle = "Building $pct% - about $left left"
}
$p.WaitForExit()
$elapsed = ((Get-Date) - $start).TotalSeconds
if ($p.ExitCode -eq 0) {
  $bar = $full * $width
  Write-Host ("`r" + " [$bar] 100%  done in " + ('{0:mm\:ss}' -f [TimeSpan]::FromSeconds($elapsed))).PadRight([Math]::Max(10, [Console]::WindowWidth - 1))
  # Only a build that compiled most packages is a full build worth timing
  $done = @((Read-Log) -split "`r?`n" | Where-Object { $_ -match '^\s+Compiling ' }).Count
  if ($done -gt $total * 0.5) { [int]$elapsed | Set-Content $timeFile }
  exit 0
}
Write-Host ''
exit $p.ExitCode
