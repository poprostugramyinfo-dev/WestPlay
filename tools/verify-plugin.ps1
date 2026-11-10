param(
  [Parameter(Mandatory = $true)]
  [string]$Path
)

$ErrorActionPreference = 'Stop'
$resolved = (Resolve-Path -LiteralPath $Path).Path
$stream = [System.IO.File]::OpenRead($resolved)
try {
  $reader = [System.IO.BinaryReader]::new($stream)
  if ($reader.ReadUInt16() -ne 0x5A4D) { throw 'Missing DOS MZ signature.' }

  $stream.Position = 0x3C
  $peOffset = $reader.ReadInt32()
  $stream.Position = $peOffset
  if ($reader.ReadUInt32() -ne 0x00004550) { throw 'Missing PE signature.' }

  $machine = $reader.ReadUInt16()
  $sectionCount = $reader.ReadUInt16()
  $stream.Position += 12
  $optionalSize = $reader.ReadUInt16()
  $characteristics = $reader.ReadUInt16()
  $optionalStart = $stream.Position
  $magic = $reader.ReadUInt16()
  if ($machine -ne 0x8664 -or $magic -ne 0x20B) {
    throw ('Expected a 64-bit x86-64 PE, got machine 0x{0:X4}, optional-header 0x{1:X4}.' -f $machine, $magic)
  }
  if (($characteristics -band 0x2000) -eq 0) { throw 'The PE image is not marked as a DLL.' }

  $stream.Position = $optionalStart + 112
  $exportRva = $reader.ReadUInt32()
  $exportSize = $reader.ReadUInt32()
  if ($exportRva -eq 0 -or $exportSize -eq 0) { throw 'The DLL has no export directory.' }

  $sections = @()
  $stream.Position = $optionalStart + $optionalSize
  for ($i = 0; $i -lt $sectionCount; $i++) {
    $stream.Position += 8
    $virtualSize = $reader.ReadUInt32()
    $virtualAddress = $reader.ReadUInt32()
    $rawSize = $reader.ReadUInt32()
    $rawOffset = $reader.ReadUInt32()
    $sections += [pscustomobject]@{
      VirtualSize = $virtualSize
      VirtualAddress = $virtualAddress
      RawSize = $rawSize
      RawOffset = $rawOffset
    }
    $stream.Position += 16
  }

  function Get-FileOffset([uint32]$Rva) {
    foreach ($section in $sections) {
      $span = [Math]::Max([double]$section.VirtualSize, [double]$section.RawSize)
      if ($Rva -ge $section.VirtualAddress -and $Rva -lt ($section.VirtualAddress + $span)) {
        return [long]($section.RawOffset + ($Rva - $section.VirtualAddress))
      }
    }
    throw ('RVA 0x{0:X8} is outside the DLL sections.' -f $Rva)
  }

  $stream.Position = Get-FileOffset $exportRva
  $stream.Position += 24
  $nameCount = $reader.ReadUInt32()
  $stream.Position += 4
  $namesRva = $reader.ReadUInt32()
  $namesOffset = Get-FileOffset $namesRva
  $exports = [System.Collections.Generic.List[string]]::new()
  $stream.Position = $namesOffset
  for ($i = 0; $i -lt $nameCount; $i++) {
    $nameRva = $reader.ReadUInt32()
    $returnTo = $stream.Position
    $stream.Position = Get-FileOffset $nameRva
    $bytes = [System.Collections.Generic.List[byte]]::new()
    while (($byte = $reader.ReadByte()) -ne 0) { $bytes.Add($byte) }
    $exports.Add([System.Text.Encoding]::ASCII.GetString($bytes.ToArray()))
    $stream.Position = $returnTo
  }

  foreach ($required in @('scs_telemetry_init', 'scs_telemetry_shutdown')) {
    if (-not $exports.Contains($required)) { throw "Missing ETS2 SCS export: $required" }
  }
}
finally {
  $stream.Dispose()
}

# Ask Windows' native loader to map the x64 DLL and resolve its imports. This does not start ETS2 or
# call the SCS entry points; only a live game session can prove the plugin works inside the game.
$module = [System.Runtime.InteropServices.NativeLibrary]::Load($resolved)
[System.Runtime.InteropServices.NativeLibrary]::Free($module)

$hash = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Host "Verified x64 ETS2 plugin PE and required SCS exports: $resolved"
Write-Host "SHA-256: $hash"
