[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Path
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "Windows EXE not found: $Path"
}

$resolved = (Resolve-Path -LiteralPath $Path).ProviderPath
[byte[]]$bytes = [System.IO.File]::ReadAllBytes($resolved)

if ($bytes.Length -lt 128) {
    throw "Not a valid PE file (too short): $resolved"
}

# MS-DOS header: IMAGE_DOS_SIGNATURE = 'MZ'
if ([BitConverter]::ToUInt16($bytes, 0) -ne 0x5A4D) {
    throw "Invalid DOS MZ signature: $resolved"
}

# DOS header offset 0x3C points at PE signature.
[int]$peOffset = [BitConverter]::ToInt32($bytes, 0x3C)
if ($peOffset -lt 64 -or $peOffset -gt ($bytes.Length - 24)) {
    throw "Invalid PE header offset: $peOffset"
}

if ([BitConverter]::ToUInt32($bytes, $peOffset) -ne 0x00004550) {
    throw "Invalid PE signature: $resolved"
}

# COFF file header is 20 bytes after the four-byte PE signature.
[int]$coff = $peOffset + 4
[int]$optionalHeaderSize = [BitConverter]::ToUInt16($bytes, $coff + 16)
[int]$optional = $coff + 20
if ($optionalHeaderSize -lt 70 -or $optionalHeaderSize -gt ($bytes.Length - $optional)) {
    throw "Invalid PE optional header length: $optionalHeaderSize"
}

[int]$magic = [BitConverter]::ToUInt16($bytes, $optional)
if ($magic -ne 0x10B -and $magic -ne 0x20B) {
    throw "Unsupported PE optional header magic: 0x$($magic.ToString('X'))"
}

# Microsoft PE specification: Subsystem is at offset 68 in PE32 and PE32+.
[int]$subsystem = [BitConverter]::ToUInt16($bytes, $optional + 68)
[int]$machine = [BitConverter]::ToUInt16($bytes, $coff)

if ($machine -ne 0x8664) {
    throw "Expected x86_64 PE machine 0x8664, got 0x$($machine.ToString('X'))"
}
if ($subsystem -ne 2) {
    $kind = if ($subsystem -eq 3) { 'Windows CUI (console)' } else { "subsystem $subsystem" }
    throw "MauLink must be Windows GUI (2), actual: $kind ($resolved)"
}

Write-Host "PASS: Windows x64 GUI subsystem (2): $resolved"
