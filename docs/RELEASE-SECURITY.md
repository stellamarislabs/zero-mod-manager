# Download verification and Windows warnings

## Current release candidate

The local 0.7.0-rc.7 Windows EXE and installer are **not Authenticode-signed**.
A detached signature is not supplied for these local RC artifacts either.
Do not describe them as signed, Microsoft-approved, malware-free, or Stable.

Windows may display an unknown-publisher or SmartScreen reputation warning.
A warning is not proof of malware, and its absence is not proof of safety.
Do not disable Defender, SmartScreen, Smart App Control, or organizational policy.
If Windows blocks execution, stop and report the exact message. Never bypass a
malware detection on the assumption that it is merely a reputation warning.

Microsoft explains the distinction in its [SmartScreen reputation documentation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation).

## Check a download (PowerShell)

Download the package and SHA256SUMS from the same maintainer release page.
In the download directory, run:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath '.\Zero.Mod.Manager_0.7.0-rc.7_x64-portable.zip'
Get-FileHash -Algorithm SHA256 -LiteralPath '.\Zero.Mod.Manager_0.7.0-rc.7_x64-setup.exe'
Get-AuthenticodeSignature -LiteralPath '.\Zero.Mod.Manager_0.7.0-rc.7_x64-setup.exe' | Select-Object Status, StatusMessage
```

Run the hash command only for the package you downloaded. Compare the complete
64-character hash with its filename's entry in SHA256SUMS (case does not matter).
If it differs, do not run the file. Delete that download and investigate the source.
For this unsigned RC, Authenticode reports NotSigned; this is not signature validation.

Checksums detect changes relative to the published manifest. They do not prove
publisher identity if an attacker can replace both the download and manifest.
No third-party verification tool is required to perform this checksum check.

## Future signed releases

An independently distributed maintainer public key and SHA256SUMS.minisig are
required before advertising detached-signature verification. A minisign signature
is separate from Windows Authenticode and does not remove SmartScreen warnings.
Maintainers must keep the private key outside the repository, set the CI secret
MINISIGN_SECRET_KEY, publish the public key through a verified channel, and verify
the generated signature before publishing. Never manufacture an identity certificate
or substitute a checksum for a signature. Stable remains blocked until release
signing policy and platform acceptance tests are satisfied.
