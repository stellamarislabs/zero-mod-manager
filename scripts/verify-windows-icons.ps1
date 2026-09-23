# Verify PE resources, not Explorer's cached extraction of an icon.
param(
  [Parameter(Mandatory=$true)][string]$Executable,
  [string]$Installer
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
if (-not ('ZeroBrandResourceProbe' -as [type])) {
  Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
public static class ZeroBrandResourceProbe {
  private delegate bool EnumName(IntPtr module, IntPtr type, IntPtr name, IntPtr parameter);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr LoadLibraryEx(string path, IntPtr file, uint flags);
  [DllImport("kernel32.dll")] static extern bool FreeLibrary(IntPtr module);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool EnumResourceNames(IntPtr module, IntPtr type, EnumName callback, IntPtr parameter);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
  [DllImport("kernel32.dll", SetLastError=true)] static extern uint SizeofResource(IntPtr module, IntPtr resource);
  [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
  [DllImport("kernel32.dll")] static extern IntPtr LockResource(IntPtr resource);
  static byte[] Read(IntPtr module, IntPtr name, int type) {
    var resource = FindResource(module, name, (IntPtr)type);
    if (resource == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
    var result = new byte[SizeofResource(module, resource)];
    Marshal.Copy(LockResource(LoadResource(module, resource)), result, 0, result.Length);
    return result;
  }
  public static byte[][] Frames(string path) {
    var module = LoadLibraryEx(path, IntPtr.Zero, 0x22); // data + image resources; never execute code
    if (module == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
    try {
      var frames = new List<byte[]>();
      EnumName callback = (h, t, name, p) => {
        var group = Read(h, name, 14);
        if (group.Length < 6) throw new InvalidOperationException("Invalid icon group");
        for (int i=0; i<BitConverter.ToUInt16(group,4); ++i) {
          int id = BitConverter.ToUInt16(group,6+i*14+12);
          frames.Add(Read(h,(IntPtr)id,3));
        }
        return true;
      };
      if (!EnumResourceNames(module,(IntPtr)14,callback,IntPtr.Zero)) throw new Win32Exception(Marshal.GetLastWin32Error());
      GC.KeepAlive(callback);
      return frames.ToArray();
    } finally { FreeLibrary(module); }
  }
}
'@
}
$manifest = Get-Content -Raw -LiteralPath (Join-Path $root 'src-tauri/icons/brand-manifest.json') | ConvertFrom-Json
$expected = @($manifest.frames.PSObject.Properties | ForEach-Object { $_.Value } | Sort-Object)
foreach ($path in @($Executable, $Installer) | Where-Object { $_ }) {
  $resolved = (Resolve-Path -LiteralPath $path).Path
  $actual = @([ZeroBrandResourceProbe]::Frames($resolved) | ForEach-Object { [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([byte[]]$_)).ToLowerInvariant() } | Sort-Object)
  if (($expected -join ',') -cne ($actual -join ',')) { throw "Embedded Windows icon does not match the shared brand: $resolved" }
  Write-Host "Embedded Windows icon verified (all 4 frames): $([IO.Path]::GetFileName($resolved))"
}
