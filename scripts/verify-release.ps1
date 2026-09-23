param([Parameter(Mandatory=$true)][string]$Directory)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Directory).Path
$seen = @{}
foreach ($line in Get-Content -LiteralPath (Join-Path $root 'SHA256SUMS')) {
    if ($line -notmatch '^([a-fA-F0-9]{64})  ([A-Za-z0-9_.-]+)$') { throw 'Invalid checksum entry' }
    $expected, $name = $Matches[1], $Matches[2]
    if ($seen.ContainsKey($name)) { throw "Duplicate checksum entry: $name" }
    $seen[$name] = $true
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $root $name)).Hash
    if ($actual -ne $expected) { throw "Checksum mismatch: $name" }
    Write-Host "Verified $name"
}
if ($seen.Count -lt 3) { throw 'Incomplete release: expected binary packages and source at minimum' }
foreach ($file in Get-ChildItem -LiteralPath $root -File) {
    if ($file.Name -notin @('SHA256SUMS','SHA256SUMS.minisig') -and -not $seen.ContainsKey($file.Name)) {
        throw "Unlisted release file: $($file.Name)"
    }
}
Write-Host 'Integrity verified. This does not authenticate the publisher or pass game QA.'
