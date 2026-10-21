param([Parameter(Mandatory = $true)][string]$NotesFile)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repoRoot
$version = (Get-Content -LiteralPath 'package.json' -Raw | ConvertFrom-Json).version
if (!$env:PIEALL_TAURI_PUBLIC_KEY -or !$env:TAURI_SIGNING_PRIVATE_KEY) { throw 'Configure the PieAll public-key variable and private-key secret before packaging.' }
node scripts/release-version.cjs --check
if ($LASTEXITCODE -ne 0) { throw 'Version check failed' }
python scripts/bundle-notices.py
if ($LASTEXITCODE -ne 0) { throw 'Notice consolidation failed' }

# This script builds only when explicitly invoked by the user or release CI.
# The CLI checks its new updater signature against the configured pubkey, which the repo keeps empty.
$keyConfig = Join-Path ([IO.Path]::GetTempPath()) "pieall-updater-$version.json"
[IO.File]::WriteAllText($keyConfig, (@{ plugins = @{ updater = @{ pubkey = $env:PIEALL_TAURI_PUBLIC_KEY.Trim() } } } | ConvertTo-Json -Depth 4 -Compress))
pnpm tauri build --target x86_64-pc-windows-msvc --bundles nsis --config $keyConfig -- --locked
if ($LASTEXITCODE -ne 0) { throw 'Tauri build/signing failed' }
$releaseRoot = Join-Path $repoRoot ".release/v$version"
if (Test-Path -LiteralPath $releaseRoot) { throw 'Release staging already exists. Inspect it and use a fresh checkout before retrying.' }
$assets = Join-Path $releaseRoot 'assets'
$portable = Join-Path $releaseRoot 'portable'
New-Item -ItemType Directory -Path $assets, $portable | Out-Null
$binary = Join-Path $repoRoot 'target/x86_64-pc-windows-msvc/release/pieall-desktop.exe'
$installer = "PieAll_${version}_x64-setup.exe"
$installerSource = Join-Path $repoRoot "target/x86_64-pc-windows-msvc/release/bundle/nsis/$installer"
Copy-Item -LiteralPath $installerSource -Destination (Join-Path $assets $installer)
Copy-Item -LiteralPath "$installerSource.sig" -Destination (Join-Path $assets "$installer.sig")
Copy-Item -LiteralPath $binary -Destination (Join-Path $portable 'PieAll.exe')
Copy-Item -LiteralPath 'THIRD-PARTY-NOTICES.txt' -Destination $portable
@'
PieAll portable — Windows 11 x64
Run PieAll.exe. The system WebView2 runtime must be installed.
No startup-at-login setting is enabled. Files remain in their existing locations.
The update command launches the standard per-user installer and switches to the installed app;
it does not replace this portable executable. Do not run both copies together.
Exit PieAll from its tray menu before manually installing/uninstalling or moving this folder.
'@ | Set-Content -LiteralPath (Join-Path $portable 'README.txt') -Encoding utf8
$zip = Join-Path $assets "PieAll_${version}_x64-portable.zip"
Compress-Archive -Path (Join-Path $portable '*') -DestinationPath $zip -CompressionLevel Optimal
pnpm tauri signer sign --app-version $version $zip
if ($LASTEXITCODE -ne 0) { throw 'Portable ZIP signing failed' }
node scripts/release-artifacts.cjs finalize $assets $version $NotesFile
if ($LASTEXITCODE -ne 0) { throw 'Release artifact preparation failed' }
cargo run -p pieall-desktop --locked --features release-tools --bin release-verify -- $assets
if ($LASTEXITCODE -ne 0) { throw 'Public/private key or artifact signature verification failed' }
Write-Output "Verified release assets: $assets"
