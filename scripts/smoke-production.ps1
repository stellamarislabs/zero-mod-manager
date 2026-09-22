param(
  [Parameter(Mandatory = $true)]
  [string]$Executable,
  [int]$TimeoutSeconds = 30
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
}
finally {
  if ($process -and -not $process.HasExited) {
    $process.Kill($true)
    $process.WaitForExit(5000)
  }
  Remove-Item -LiteralPath $smokeRoot -Recurse -Force -ErrorAction SilentlyContinue
}
