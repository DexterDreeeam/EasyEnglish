#requires -Version 5
<#
.SYNOPSIS
    Build the EasyEnglish Windows installer.

.DESCRIPTION
    Assumes a Release build already exists under build/msvc-release/. Stages
    EasyEnglish.exe + runtime DLLs (vcpkg sits them next to the exe) + the
    shipped mini_dict.sqlite + assets/fonts/, then drives Inno Setup (iscc)
    to produce installer/dist/EasyEnglishSetup-<version>.exe.

.PREREQUISITES
    - cmake --preset msvc-release && cmake --build --preset msvc-release
    - Inno Setup 6 installed (iscc.exe on PATH or in default install dir)
    - assets/fonts/ populated (NotoSans-Regular.ttf + NotoSansSC-Regular.otf).
      release.yml fetches them automatically; for a local build, download
      manually from https://fonts.google.com.
#>
[CmdletBinding()]
param(
    [string]$BuildDir = "$PSScriptRoot\..\build\msvc-release",
    [string]$Fixtures = "$PSScriptRoot\..\tests\fixtures\mini_dict.sqlite",
    [string]$FontsDir = "$PSScriptRoot\..\assets\fonts",
    [string]$Staging  = "$PSScriptRoot\..\installer\staging"
)

$ErrorActionPreference = 'Stop'

function Require-File($path) {
    if (-not (Test-Path $path)) {
        throw "Required input not found: $path"
    }
}

$exe = Join-Path $BuildDir 'src\EasyEnglish.exe'
Require-File $exe
Require-File $Fixtures

# Locate Inno Setup compiler.
$iscc = Get-Command iscc.exe -ErrorAction SilentlyContinue
if ($null -eq $iscc) {
    $candidate = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
    if (Test-Path $candidate) {
        $iscc = $candidate
    } else {
        throw "Inno Setup 6 not found. Install it from https://jrsoftware.org/isinfo.php."
    }
}

# Wipe and recreate staging.
if (Test-Path $Staging) { Remove-Item -Recurse -Force $Staging }
New-Item -ItemType Directory -Force -Path $Staging | Out-Null

Copy-Item $exe       (Join-Path $Staging 'EasyEnglish.exe')
Copy-Item $Fixtures  (Join-Path $Staging 'mini_dict.sqlite')

# Copy any DLLs sitting next to the exe (vcpkg copies its runtime DLLs there
# during build via VCPKG_APPLOCAL_DEPS). This catches glfw3.dll + OpenSSL DLLs
# without needing windeployqt or manual lookup.
Get-ChildItem (Join-Path $BuildDir 'src') -Filter *.dll -ErrorAction SilentlyContinue |
    ForEach-Object { Copy-Item $_.FullName $Staging }

# Fonts directory (Noto Sans + Noto Sans SC). If absent, log loudly so the
# packager notices CJK won't render — but don't fail: dev builds may legitimately
# ship without the bundle while the release pipeline always fetches them.
$fontFiles = @()
if (Test-Path $FontsDir) {
    # Get-ChildItem -Include requires either a wildcard path or -Recurse;
    # without one of those it silently returns nothing. Use -Filter per
    # extension to keep the matching robust across PowerShell versions.
    $fontFiles += Get-ChildItem -Path $FontsDir -Filter *.ttf -File -ErrorAction SilentlyContinue
    $fontFiles += Get-ChildItem -Path $FontsDir -Filter *.otf -File -ErrorAction SilentlyContinue
}
if ($fontFiles.Count -gt 0) {
    $fontDst = Join-Path $Staging 'fonts'
    New-Item -ItemType Directory -Force -Path $fontDst | Out-Null
    foreach ($f in $fontFiles) {
        Copy-Item $f.FullName $fontDst
        Write-Host "staged font: $($f.Name) ($($f.Length) bytes)"
    }
} else {
    Write-Warning "No fonts under $FontsDir — CJK characters will render as boxes in the packaged build."
}

& $iscc.Source "$PSScriptRoot\..\installer\EasyEnglish.iss"

Write-Host "Done. Installer at installer/dist/EasyEnglishSetup-*.exe"
