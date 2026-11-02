# Builds the installer for Windows: target\installer\openpacketmon-<version>-setup.exe
#
# Builds openpacketmon and openpacketmon-capture in release mode with the version from Cargo.toml, checks the list
# of third-party licenses and calls the compiler of Inno Setup 6 (resources\packaging\windows\openpacketmon.iss).
# Raise the version in Cargo.toml and capture\Cargo.toml first (then `cargo update --workspace --offline`), the same
# way as for the .deb (dev/build-deb.sh); the user names it.
#
# Needs: Rust with the MSVC tool chain, the Npcap SDK in LIB (docs/windows.md 3), Python 3 and Inno Setup 6
# (winget install JRSoftware.InnoSetup). Usage, from any folder:
#     powershell -ExecutionPolicy Bypass -File dev\build-installer.ps1

# not 'Stop': PowerShell 5.1 would take the progress cargo writes to stderr for errors; every step checks its exit code
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Get-Version([string]$manifest) {
    # the first `version = "..."` is the one of [package] (ASCII only: PowerShell 5.1 reads this file as ANSI)
    $line = Select-String -Path $manifest -Pattern '^version = "([^"]+)"' | Select-Object -First 1
    if (-not $line) { throw "no version in $manifest" }
    $line.Matches[0].Groups[1].Value
}

$version = Get-Version 'Cargo.toml'
$helperVersion = Get-Version 'capture\Cargo.toml'
if ($version -ne $helperVersion) {
    throw "Cargo.toml has $version, capture\Cargo.toml ${helperVersion}: raise both"
}

# the Npcap SDK, if the variable is only set for the user and this shell is older
if (-not $env:LIB -or $env:LIB -notmatch 'npcap') {
    $env:LIB = [Environment]::GetEnvironmentVariable('LIB', 'User')
}
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (($env:PATH -split ';') -notcontains $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }

$iscc = @(
    (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
    (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 is missing: winget install JRSoftware.InnoSetup' }

Write-Host "Building openpacketmon $version"
cargo build --release --locked --workspace
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

python -I dev\third-party-licenses.py --check
if ($LASTEXITCODE -ne 0) { throw 'THIRD-PARTY-LICENSES.txt is out of date: python -I dev\third-party-licenses.py' }

& $iscc /Qp "/DAppVersion=$version" 'resources\packaging\windows\openpacketmon.iss'
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup failed' }

$setup = Join-Path $root "target\installer\openpacketmon-$version-setup.exe"
$hash = (Get-FileHash -Algorithm SHA256 $setup).Hash.ToLower()
Write-Host ''
Write-Host "Installer: $setup"
Write-Host "SHA-256:   $hash"
Write-Host 'Unsigned: SmartScreen warns, Smart App Control may block it (docs/windows.md 6).'
