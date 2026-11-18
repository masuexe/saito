# Velopack's release of the app, on Windows, for one architecture: target\package\windows\<arch>\releases\
# (saito-<channel>-Setup.exe, the installer; saito-<channel>-Portable.zip, the app alone; and the update package and
# its feed the installed app's updater reads from the GitHub release: saito-<version>-<channel>-full.nupkg,
# releases.<channel>.json, assets.<channel>.json, RELEASES-<channel>).
#
#   pwsh packaging\windows\package.ps1 [-Arch x64|arm64] [-Version <version>] [-Icon <saito.ico>]
#       [-Ffmpeg <ffmpeg.exe>]
#
# Each architecture is its own package on its own Velopack channel, named for the system and the architecture as
# every system's is (which is also Velopack's runtime, `vpk pack --runtime`), so that each one's files and feed sit
# beside the other's in one release. Each is built natively, on a machine of its architecture:
#
#   x64      win-x64      saito-win-x64-Setup.exe, saito-win-x64-Portable.zip
#   arm64    win-arm64    saito-win-arm64-Setup.exe, saito-win-arm64-Portable.zip
#
# (Up to 0.19.1 x64's app was packed a second time on the channel `win`, the only one up to 0.19.0: the bridge,
# docs/app.md §19, "Updates".)
#
# The app: saito.exe, ffmpeg.exe, LICENSE.txt, THIRD-PARTY-NOTICES.txt, ffmpeg-NOTICE.txt. No content, no ROM and no
# extracted asset is in it: the app downloads each game's content (docs/design/content-deployment.md §5) and extracts
# each game's pack from the player's own copy. The installer is the
# player's alone (%LOCALAPPDATA%\saito, no administrator), with a Start menu and a desktop shortcut; uninstalling
# leaves the player's data folder (%APPDATA%\saito: builds, replays, settings, the extracted packs, the downloaded
# content) as it is.
#
# -Arch      x64 or arm64 (above): this machine's own, which is the default
# -Version   the version, if set; else $env:SAITO_VERSION, else the app crate's
# -Icon      the .ico packaging/icons.sh makes, for the program and the installer; without it neither has one
# -Ffmpeg    the ffmpeg.exe to bundle (for the architecture packaged); else $env:SAITO_BUNDLE_FFMPEG, else the release
#            packaging\ffmpeg.sha256 pins, downloaded and checked (a static program that needs no DLL)
# $env:WINDOWS_CERTIFICATE_PATH, $env:WINDOWS_CERTIFICATE_PASSWORD
#            a code-signing certificate (.pfx) vpk signs every program of the release with (the app's, ffmpeg,
#            Velopack's installer and updater); else none is signed
# $env:VPK   Velopack's vpk (the version crates/saito's velopack is: 1.3.39-g8d2844f); else vpk on the PATH
param(
    [ValidateSet('x64', 'arm64')]
    [string]$Arch,
    [string]$Version = $env:SAITO_VERSION,
    [string]$Icon,
    [string]$Ffmpeg = $env:SAITO_BUNDLE_FFMPEG
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# The machine's architecture (the system's, whatever this PowerShell runs as), which the package's must be.
$machine = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    'X64' { 'x64' }
    'Arm64' { 'arm64' }
    default { throw "saito is packaged on x64 or arm64 Windows, not $_" }
}
if (-not $Arch) { $Arch = $machine }
if ($Arch -ne $machine) { throw "the $Arch package is built on $Arch Windows, not on this $machine machine" }
# The architecture's channel, which is also Velopack's runtime; and the ffmpeg release's program for the architecture.
$package = @{
    'x64' = @{ Channel = 'win-x64'; Ffmpeg = 'ffmpeg-windows-x86_64.exe' }
    'arm64' = @{ Channel = 'win-arm64'; Ffmpeg = 'ffmpeg-windows-arm64.exe' }
}[$Arch]

$root = (Resolve-Path "$PSScriptRoot\..\..").Path
if (-not $Version) {
    $Version = (Select-String -Path "$root\Cargo.toml" -Pattern '^version = "(.*)"$' | Select-Object -First 1).Matches[0].Groups[1].Value
}
$out = "$root\target\package\windows\$Arch"
$app = "$out\saito"
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $app | Out-Null

# The release build, for a packaged app (`SAITO_PACKAGED`: it plays the games' content it downloads into its
# store, none of the repository's), with
# the icon. Cargo's messages name the program and build.rs's OUT_DIR, where it wrote the third-party notices.
$env:SAITO_PACKAGED = '1'
if ($Icon) { $env:SAITO_WINDOWS_ICON = (Resolve-Path $Icon).Path }
$messages = & cargo build --release --locked -p saito --message-format=json-render-diagnostics --manifest-path "$root\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
Remove-Item Env:SAITO_PACKAGED
if ($Icon) { Remove-Item Env:SAITO_WINDOWS_ICON }
$program = $null
$notices = $null
foreach ($line in $messages) {
    if (-not $line.StartsWith('{')) { continue }
    $message = $line | ConvertFrom-Json
    if ($message.reason -eq 'compiler-artifact' -and $message.executable) { $program = $message.executable }
    if ($message.reason -eq 'build-script-executed' -and (Test-Path "$($message.out_dir)\licenses.txt")) { $notices = "$($message.out_dir)\licenses.txt" }
}
if (-not $program) { throw "the build named no program" }
if (-not $notices) { throw "the build named no licenses.txt (crates/saito/build.rs)" }

# A file of the ffmpeg release packaging\ffmpeg.sha256 pins (nettaisen/ffmpeg-build's, a minimal static
# build of what the replay export runs), downloaded once into target\package\ffmpeg\<tag>\ and checked: its path.
function Get-FfmpegFile([string]$file) {
    $pins = Get-Content "$root\packaging\ffmpeg.sha256"
    $release = ($pins | Select-String -Pattern '^# release: (.*)$' | Select-Object -First 1).Matches[0].Groups[1].Value
    $sum = $pins | ForEach-Object { if ($_ -match "^([0-9a-f]{64})  $([regex]::Escape($file))$") { $Matches[1] } } | Select-Object -First 1
    if (-not $sum) { throw "packaging\ffmpeg.sha256 pins no $file" }
    $dir = "$root\target\package\ffmpeg\$release"
    New-Item -ItemType Directory -Force $dir | Out-Null
    $path = "$dir\$file"
    if (-not (Test-Path $path) -or (Get-FileHash -Algorithm SHA256 $path).Hash.ToLower() -ne $sum) {
        Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/nettaisen/ffmpeg-build/releases/download/$release/$file" -OutFile $path
        if ((Get-FileHash -Algorithm SHA256 $path).Hash.ToLower() -ne $sum) {
            Remove-Item $path
            throw "$file of $release isn't the one pinned"
        }
    }
    return $path
}

Copy-Item $program "$app\saito.exe"
# ffmpeg beside the program, where the export looks first (saito_video::mp4::find_ffmpeg): the release's program for
# the architecture; and its notice (GPL: what it is built from, where that source is).
if (-not $Ffmpeg) { $Ffmpeg = Get-FfmpegFile $package.Ffmpeg }
Copy-Item $Ffmpeg "$app\ffmpeg.exe"
Copy-Item "$root\LICENSE" "$app\LICENSE.txt"
Copy-Item $notices "$app\THIRD-PARTY-NOTICES.txt"
Copy-Item (Get-FfmpegFile 'ffmpeg-NOTICE.txt') "$app\ffmpeg-NOTICE.txt"

# The release (Velopack's: docs/app.md §19), on the architecture's channel, every program in it signed where there is
# a certificate.
$vpk = if ($env:VPK) { $env:VPK } else { 'vpk' }
$pack = @('pack', '--packId', 'saito', '--packVersion', $Version, '--packTitle', 'saito', '--packAuthors', 'EXESSR Hub',
    '--packDir', $app, '--mainExe', 'saito.exe', '--channel', $package.Channel, '--runtime', $package.Channel,
    '--outputDir', "$out\releases")
if ($Icon) { $pack += @('--icon', (Resolve-Path $Icon).Path) }
if ($env:WINDOWS_CERTIFICATE_PATH) {
    $pack += @('--signParams', "/f `"$env:WINDOWS_CERTIFICATE_PATH`" /p `"$env:WINDOWS_CERTIFICATE_PASSWORD`" /fd SHA256 /tr http://timestamp.digicert.com /td SHA256")
}
& $vpk @pack
if ($LASTEXITCODE -ne 0) { throw "vpk pack failed" }

Write-Output "$out\releases"
