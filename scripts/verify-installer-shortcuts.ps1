# Compile/run the real NSIS hook in an isolated temp fixture; no user links touched.
param([string]$MakeNsis = "$env:LOCALAPPDATA/tauri/NSIS/makensis.exe")
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('zero-icon-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixture | Out-Null
try {
  $shell = New-Object -ComObject WScript.Shell
  foreach ($name in @('own','other')) {
    $target = if ($name -eq 'own') { 'manager.exe' } else { 'another-app.exe' }
    [IO.File]::WriteAllText((Join-Path $fixture $target), 'non-executable fixture')
    $link = $shell.CreateShortcut((Join-Path $fixture "$name.lnk"))
    $link.TargetPath = Join-Path $fixture $target
    $link.Arguments = '--fixture "keep arguments"'
    $link.WorkingDirectory = $fixture
    $link.Description = 'Keep this custom description'
    $link.WindowStyle = 7
    $link.IconLocation = "$fixture/old.ico,0"
    $link.Save()
  }
  Copy-Item -LiteralPath (Join-Path $root 'src-tauri/icons/icon.ico') -Destination (Join-Path $fixture 'brand.ico')
  $beforeOther = (Get-FileHash -LiteralPath (Join-Path $fixture 'other.lnk')).Hash
  & $MakeNsis '/V2' "/DSOURCE_ROOT=$root" "/DFIXTURE_ROOT=$fixture" (Join-Path $PSScriptRoot 'fixtures/icon-shortcuts.nsi')
  if ($LASTEXITCODE -ne 0) { throw 'NSIS hook fixture failed to compile' }
  $run = Start-Process -FilePath (Join-Path $fixture 'verify-shortcuts.exe') -WindowStyle Hidden -PassThru
  if (-not $run.WaitForExit(15000)) { $run.Kill(); throw 'NSIS icon fixture timed out' }
  if ($run.ExitCode -ne 0) { throw "NSIS icon fixture failed: $($run.ExitCode)" }
  $updated = $shell.CreateShortcut((Join-Path $fixture 'own.lnk'))
  if ($updated.IconLocation -cne "$fixture\brand.ico,0") { throw "Wrong icon location: $($updated.IconLocation)" }
  if ($updated.Arguments -cne '--fixture "keep arguments"' -or $updated.WorkingDirectory -cne $fixture -or $updated.Description -cne 'Keep this custom description' -or $updated.WindowStyle -ne 7) { throw 'Shortcut preferences changed' }
  if ((Get-FileHash -LiteralPath (Join-Path $fixture 'other.lnk')).Hash -cne $beforeOther) { throw 'Unrelated shortcut changed' }
  if (Test-Path -LiteralPath (Join-Path $fixture 'missing.lnk')) { throw 'Missing shortcut was created' }
  $explorer = New-Object -ComObject Shell.Application
  $identity = $explorer.Namespace($fixture).ParseName('own.lnk').ExtendedProperty('System.AppUserModel.ID')
  if ($identity -cne 'app.zeromodmanager.desktop') { throw "Wrong shortcut identity: $identity" }
  Write-Host 'Installer icon migration passed: exact target only, identity/icon updated, arguments and preferences preserved, missing/unrelated links untouched.'
} finally {
  $resolved = (Resolve-Path -LiteralPath $fixture).Path
  if (-not $resolved.StartsWith([IO.Path]::GetTempPath(), [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($resolved) -notlike 'zero-icon-test-*') { throw 'Unsafe icon fixture cleanup path' }
  Remove-Item -LiteralPath $resolved -Recurse -Force
}
