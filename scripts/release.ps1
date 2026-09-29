# Builds a release: the NSIS installer and a portable zip, both in dist/.
# Needs Rust and Node.js (the Tauri CLI runs through npx, nothing to install).
#
#   pwsh scripts/release.ps1

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

$version = (Get-Content settings/tauri.conf.json -Raw | ConvertFrom-Json).version
$triple = (rustc -vV | Select-String '^host: (.+)$').Matches[0].Groups[1].Value
Write-Host "Halo Battery $version ($triple)"

# 1. the tray, shipped inside the installer as an external binary
cargo build --release --bin halo-battery
if ($LASTEXITCODE) { throw 'building the tray failed' }
New-Item -ItemType Directory -Force settings/binaries | Out-Null
Copy-Item target/release/halo-battery.exe "settings/binaries/halo-battery-$triple.exe" -Force

# 2. the settings window and the installer
Push-Location settings
try {
    npx --yes '@tauri-apps/cli@^2' build --config tauri.release.json
    if ($LASTEXITCODE) { throw 'building the installer failed' }
} finally {
    Pop-Location
}

# 3. collect the installer and a portable zip
Remove-Item dist -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory dist | Out-Null
Copy-Item target/release/bundle/nsis/*-setup.exe dist/
$portable = Join-Path dist "halo-battery-$version-portable"
New-Item -ItemType Directory $portable | Out-Null
Copy-Item target/release/halo-battery.exe, target/release/halo-settings.exe, README.md, LICENSE $portable
Compress-Archive -Path "$portable/*" -DestinationPath "$portable.zip"
Remove-Item $portable -Recurse

Get-ChildItem dist | Format-Table Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
