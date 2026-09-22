# Run with PowerShell 7 from the repository. No publishing or Git-index mutations.
param([switch]$SnapshotOnly)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $root
$version = (Get-Content -Raw package.json | ConvertFrom-Json).version
$stamp = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss')
$out = Join-Path $root "artifacts/prepared-$version-$stamp"
$stage = Join-Path ([IO.Path]::GetTempPath()) ('zero-source-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage,$out | Out-Null
function Get-SourceFiles {
    $paths = @('src','src-tauri/src','src-tauri/icons','src-tauri/capabilities','scripts','schema','images','catalog','docs','.github')
    $files = @(Get-ChildItem -LiteralPath $root -File | Where-Object { $_.Name -match '\.(md|json|html|ts)$|^(LICENSE|\.gitignore)$' -and $_.Name -notmatch 'vite.config.d.ts' })
    foreach ($path in $paths) { $files += Get-ChildItem -LiteralPath (Join-Path $root $path) -File -Recurse }
    foreach ($name in @('Cargo.toml','Cargo.lock','build.rs','tauri.conf.json')) { $files += Get-Item -LiteralPath (Join-Path $root "src-tauri/$name") }
    $files | Sort-Object FullName -Unique | ForEach-Object {
        if ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Source symlinks are not permitted' }
        $relative = [IO.Path]::GetRelativePath($root, $_.FullName).Replace('\','/')
        if ($relative -match '\.(exe|dll|pak|utoc|ucas|sqlite3|zip|key|pem)$|(^|/)\.env') { throw "Unexpected source payload: $relative" }
        [pscustomobject]@{ path=$relative; sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    }
}
try {
    $before = @(Get-SourceFiles)
    foreach ($entry in $before) {
        $destination = Join-Path $stage $entry.path
        New-Item -ItemType Directory -Force -Path (Split-Path $destination -Parent) | Out-Null
        Copy-Item -LiteralPath (Join-Path $root $entry.path) -Destination $destination
    }
    $before | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $stage 'SOURCE-MANIFEST.json') -Encoding utf8NoBOM
    $sourceName = "Zero.Mod.Manager_${version}-source.zip"
    # .NET includes dot-directories, unlike Compress-Archive's hidden-file behavior.
    [IO.Compression.ZipFile]::CreateFromDirectory($stage, (Join-Path $out $sourceName))
    if ($SnapshotOnly) { Write-Host "SOURCE_SNAPSHOT=$out"; return }
    $env:ZERO_MOD_MANAGER_PROJECT_URL = 'https://github.com/stellamarislabs/zero-mod-manager'
    $env:ZERO_MOD_MANAGER_RELEASE_API = 'https://api.github.com/repos/stellamarislabs/zero-mod-manager/releases/latest'
    npm.cmd run build:windows
    if ($LASTEXITCODE -ne 0) { throw 'Production build failed' }
    $after = @(Get-SourceFiles)
    if (($before | ConvertTo-Json -Compress) -cne ($after | ConvertTo-Json -Compress)) { throw 'Source changed during build; do not publish this preparation' }
    $exe = Join-Path $root 'src-tauri/target/release/zero-mod-manager.exe'
    & "$PSScriptRoot/smoke-production.ps1" -Executable $exe
    $portable = Join-Path $stage 'portable'
    New-Item -ItemType Directory -Path $portable | Out-Null
    Copy-Item -LiteralPath $exe -Destination (Join-Path $portable 'Zero Mod Manager.exe')
    foreach ($name in @('LICENSE','README.md','THIRD_PARTY_NOTICES.md','PORTABLE-DATA.md','KNOWN_LIMITATIONS.md')) { Copy-Item -LiteralPath (Join-Path $root $name) -Destination $portable }
    Copy-Item -LiteralPath (Join-Path $root 'docs/RELEASE-SECURITY.md') -Destination $portable
    $portableName = "Zero.Mod.Manager_${version}_x64-portable.zip"
    [IO.Compression.ZipFile]::CreateFromDirectory($portable, (Join-Path $out $portableName))
    $installerName = "Zero.Mod.Manager_${version}_x64-setup.exe"
    Copy-Item -LiteralPath (Join-Path $root "src-tauri/target/release/bundle/nsis/Zero Mod Manager_${version}_x64-setup.exe") -Destination (Join-Path $out $installerName)
    foreach ($doc in @('RELEASE-SECURITY.md','NEXUS-RELEASE-DRAFT.md','QA-MATRIX.md','MIGRATION-ROLLBACK.md')) { Copy-Item -LiteralPath (Join-Path $root "docs/$doc") -Destination $out }
    Copy-Item -LiteralPath (Join-Path $root 'KNOWN_LIMITATIONS.md') -Destination $out
    $provenance = [ordered]@{
        version=$version; preparedUtc=[DateTime]::UtcNow.ToString('o'); channel='rc'; gameAcceptance='pending';
        project=$env:ZERO_MOD_MANAGER_PROJECT_URL; releaseApi=$env:ZERO_MOD_MANAGER_RELEASE_API;
        baseCommit=(git rev-parse HEAD); sourceState='working-tree snapshot, not a claim that baseCommit contains all changes';
        sourceArchive=$sourceName; sourceSha256=(Get-FileHash -LiteralPath (Join-Path $out $sourceName)).Hash.ToLowerInvariant();
        executableSha256=(Get-FileHash -LiteralPath $exe).Hash.ToLowerInvariant();
        executableSignature=(Get-AuthenticodeSignature -LiteralPath $exe).Status.ToString();
        installerSignature=(Get-AuthenticodeSignature -LiteralPath (Join-Path $out $installerName)).Status.ToString();
        detachedSignature='not supplied'; sourceFileCount=$before.Count
    }
    $provenance | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $out 'RELEASE-PROVENANCE.json') -Encoding utf8NoBOM
    $hashes = Get-ChildItem -LiteralPath $out -File | Sort-Object Name | ForEach-Object { '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName).Hash.ToLowerInvariant(), $_.Name }
    $hashes | Set-Content -LiteralPath (Join-Path $out 'SHA256SUMS') -Encoding utf8NoBOM
    & "$PSScriptRoot/verify-release.ps1" -Directory $out
    Write-Host "PREPARED_RELEASE=$out"
} finally {
    $resolved = (Resolve-Path -LiteralPath $stage).Path
    if (-not $resolved.StartsWith([IO.Path]::GetTempPath(), [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($resolved) -notlike 'zero-source-*') { throw 'Unsafe temporary cleanup path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
