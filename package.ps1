# Builds release and zips: visualiser.exe + scenes/ + ffmpeg.exe (+ its licence).
# Put a GPL static ffmpeg build in .\ffmpeg\ (ffmpeg.exe, LICENSE, README with source link) first.
$ErrorActionPreference = "Stop"
cargo build --release
$out = "dist\visualiser"
Remove-Item -Recurse -Force dist -ErrorAction SilentlyContinue
New-Item -ItemType Directory $out | Out-Null
Copy-Item target\release\visualiser.exe $out
# Only committed scenes: AI-written or experimental ones in the working tree stay out of the package.
foreach ($f in git ls-files scenes) {
    $dest = Join-Path $out $f
    New-Item -ItemType Directory -Force (Split-Path $dest) | Out-Null
    Copy-Item $f $dest
}
Copy-Item -Recurse ffmpeg\* $out
Copy-Item LICENSE, ASSUMPTIONS.md, README.md, docs\USAGE.md $out -ErrorAction SilentlyContinue
Compress-Archive -Path "$out\*" -DestinationPath dist\visualiser.zip
Write-Host "dist\visualiser.zip"

# Installer (optional): needs Inno Setup 6.
$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "${env:LOCALAPPDATA}\Programs\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($iscc) {
    & $iscc "/DAppVersion=$version" installer\visualiser.iss
    if ($LASTEXITCODE -ne 0) { throw "ISCC failed" }
    Write-Host "dist\visualiser-setup-$version.exe"
} else {
    Write-Host "Inno Setup 6 not found, skipping the installer (winget install JRSoftware.InnoSetup)"
}
