#requires -Version 5
<#
.SYNOPSIS
    Build the EasyEnglish Windows installer.

.DESCRIPTION
    Assumes a Release build already exists under build/msvc-release/. Stages
    EasyEnglish.exe + GLFW DLL (if dynamic) + OpenSSL DLLs (for HTTPS) +
    the shipped mini_dict.sqlite, then drives Inno Setup (iscc) to produce
    installer/dist/EasyEnglishSetup-<version>.exe.

.PREREQUISITES
    - cmake --preset msvc-release && cmake --build --preset msvc-release
    - Inno Setup 6 installed (iscc.exe on PATH or in default install dir)
#>
[CmdletBinding()]
param(
    [string]$BuildDir = "$PSScriptRoot\..\build\msvc-release",
    [string]$Fixtures = "$PSScriptRoot\..\tests\fixtures\mini_dict.sqlite",
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
# during build via VCPKG_APPLOCAL_DEPS). This typically catches glfw3.dll +
# OpenSSL DLLs without needing windeployqt or manual lookup.
Get-ChildItem (Join-Path $BuildDir 'src') -Filter *.dll -ErrorAction SilentlyContinue |
    ForEach-Object { Copy-Item $_.FullName $Staging }

& $iscc.Source "$PSScriptRoot\..\installer\EasyEnglish.iss"

Write-Host "Done. Installer at installer/dist/EasyEnglishSetup-*.exe"
