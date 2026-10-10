# Builds release and zips: visualiser.exe + scenes/ + ffmpeg.exe (+ its licence).
# Put a GPL static ffmpeg build in .\ffmpeg\ (ffmpeg.exe, LICENSE, README with source link) first.
$ErrorActionPreference = "Stop"
cargo build --release
$out = "dist\visualiser"
Remove-Item -Recurse -Force dist -ErrorAction SilentlyContinue
New-Item -ItemType Directory $out | Out-Null
Copy-Item target\release\visualiser.exe $out
Copy-Item -Recurse scenes "$out\scenes"
Copy-Item -Recurse ffmpeg\* $out
Copy-Item ASSUMPTIONS.md, README.md, docs\USAGE.md $out -ErrorAction SilentlyContinue
Compress-Archive -Path "$out\*" -DestinationPath dist\visualiser.zip
Write-Host "dist\visualiser.zip"
