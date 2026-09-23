param(
  [Parameter(Mandatory = $true)]
  [string]$Executable,
  [int]$TimeoutSeconds = 30,
  [switch]$FactoryReset
)

$ErrorActionPreference = "Stop"
$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
$smokeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("zero-mod-manager-smoke-" + [guid]::NewGuid().ToString("N"))
$smokeExecutable = Join-Path $smokeRoot "Zero Mod Manager.exe"
New-Item -ItemType Directory -Force -Path $smokeRoot | Out-Null
Copy-Item -LiteralPath $resolvedExecutable -Destination $smokeExecutable
# The marker isolates the smoke run from real user data and verifies the
# self-contained portable mode at the same time.
New-Item -ItemType File -Force -Path (Join-Path $smokeRoot "portable-data.flag") | Out-Null

if ($FactoryReset) {
  $dataRoot = Join-Path $smokeRoot 'data'
  New-Item -ItemType Directory -Path (Join-Path $dataRoot 'mods') | Out-Null
  # Deliberately invalid SQLite: startup succeeds only if reset precedes DB open.
  [IO.File]::WriteAllText((Join-Path $dataRoot 'zcom-mod-manager.sqlite3'), 'old app state')
  [IO.File]::WriteAllText((Join-Path $dataRoot 'mods/old-copy.pak'), 'managed copy')
  [IO.File]::WriteAllText((Join-Path $smokeRoot 'game-mod-sentinel.pak'), 'game file stays')
  $resetId = [guid]::NewGuid().ToString()
  $digestInput = [Text.Encoding]::UTF8.GetBytes('portable' + [char]0 + $dataRoot.Replace('\','/').ToLowerInvariant())
  $digest = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($digestInput)).ToLowerInvariant()
  $journal = @{ version=1; id=$resetId; roots_digest=$digest; present=@($true); external_library_retained=$null }
  [IO.File]::WriteAllText((Join-Path $smokeRoot '.data.factory-reset.json'), ($journal | ConvertTo-Json -Compress))
}

$process = $null
try {
  $process = Start-Process -FilePath $smokeExecutable -PassThru -WindowStyle Hidden

  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $ready = $false
  while ([DateTime]::UtcNow -lt $deadline) {
    if ($process.HasExited) {
      throw "Production executable exited before the embedded frontend became ready (exit code $($process.ExitCode))."
    }
    $logs = Get-ChildItem -LiteralPath (Join-Path $smokeRoot "data") -Recurse -Filter application.jsonl -File -ErrorAction SilentlyContinue
    foreach ($log in $logs) {
      $body = Get-Content -Raw -LiteralPath $log.FullName -ErrorAction SilentlyContinue
      if ($body -match '"event":"frontend_ready"') {
        $ready = $true
        break
      }
    }
    if ($ready) { break }
    Start-Sleep -Milliseconds 500
    $process.Refresh()
  }

  if (-not $ready) {
    throw "The backend started, but the embedded frontend did not report ready within $TimeoutSeconds seconds. A development URL such as localhost:1420 may have been packaged."
  }
  Write-Host "Production smoke passed: the embedded frontend reported ready without a development server."
  if ($FactoryReset) {
    $recovery = Join-Path $smokeRoot "data.reset-recovery-$resetId"
    if ([IO.File]::ReadAllText((Join-Path $recovery 'zcom-mod-manager.sqlite3')) -cne 'old app state') { throw 'Reset backup lost the old database fixture.' }
    if ([IO.File]::ReadAllText((Join-Path $recovery 'mods/old-copy.pak')) -cne 'managed copy') { throw 'Reset backup lost the managed copy fixture.' }
    if (Test-Path -LiteralPath (Join-Path $dataRoot 'mods/old-copy.pak')) { throw 'Old library state survived factory reset.' }
    if (Test-Path -LiteralPath (Join-Path $smokeRoot '.data.factory-reset.json')) { throw 'Reset journal was not completed.' }
    if ([IO.File]::ReadAllText((Join-Path $smokeRoot 'game-mod-sentinel.pak')) -cne 'game file stays') { throw 'Reset changed an out-of-scope game fixture.' }
    Write-Host 'Factory-reset production smoke passed: old state backed up, fresh app ready, game sentinel unchanged.'
  }
}
finally {
  if ($process -and -not $process.HasExited) {
    $process.Kill($true)
    $process.WaitForExit(5000)
  }
  $resolvedSmoke = (Resolve-Path -LiteralPath $smokeRoot).Path
  if (-not $resolvedSmoke.StartsWith([IO.Path]::GetTempPath(), [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($resolvedSmoke) -notlike 'zero-mod-manager-smoke-*') { throw 'Unsafe smoke cleanup path' }
  Remove-Item -LiteralPath $resolvedSmoke -Recurse -Force -ErrorAction SilentlyContinue
}
