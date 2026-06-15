<#
.SYNOPSIS
    Self-sign EasyEnglish build artifacts with a locally-trusted development
    certificate.

.DESCRIPTION
    Creates (once) a self-signed code-signing certificate, trusts it for the
    current user, and Authenticode-signs the given files. This removes the
    "unknown publisher" prompt and lets the locally-built installer be installed
    and run on THIS machine.

    The signature is trusted ONLY on machines where this self-signed certificate
    has been added to the trust store (done automatically here for the current
    user). It is NOT a substitute for a CA / Azure Trusted Signing certificate
    when distributing to other users — for Microsoft Store / public distribution
    swap the certificate source for a trusted one (see plan.md). The signing and
    trust logic here is intentionally a no-op-friendly, idempotent building block
    that a trusted-signing flow can reuse.

    Uses the built-in Set-AuthenticodeSignature so no Windows SDK / signtool is
    required.

.PARAMETER Files
    One or more paths to sign. Missing files are skipped with a warning.

.PARAMETER Subject
    Common name of the self-signed certificate. A stable value lets the cert be
    reused across builds instead of creating a new one each time.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string[]] $Files,

    [string] $Subject = 'EasyEnglish Self-Signed (Dev)'
)

$ErrorActionPreference = 'Stop'
$subjectDn = "CN=$Subject"

function Get-OrCreateSigningCert {
    param([string] $SubjectDn)

    # Reuse an existing, still-valid code-signing cert with this subject so we do
    # not pile up a new certificate (and a new trust entry) on every build.
    $existing = Get-ChildItem Cert:\CurrentUser\My |
        Where-Object {
            $_.Subject -eq $SubjectDn -and
            $_.NotAfter -gt (Get-Date) -and
            $_.HasPrivateKey
        } |
        Sort-Object NotAfter -Descending |
        Select-Object -First 1
    if ($existing) {
        Write-Host "[sign] Reusing certificate $($existing.Thumbprint)"
        return $existing
    }

    Write-Host "[sign] Creating self-signed code-signing certificate: $SubjectDn"
    # 10-year validity, no timestamp needed: a self-signed cert has no trusted
    # timestamp authority, so a long lifetime keeps signatures valid offline.
    return New-SelfSignedCertificate `
        -Type CodeSigningCert `
        -Subject $SubjectDn `
        -KeyUsage DigitalSignature `
        -KeyAlgorithm RSA -KeyLength 3072 `
        -CertStoreLocation Cert:\CurrentUser\My `
        -NotAfter (Get-Date).AddYears(10) `
        -FriendlyName 'EasyEnglish development code-signing certificate'
}

function Set-CertTrusted {
    param($Cert)

    # Trust the certificate on THIS machine for the current user so its
    # signatures validate: add the public certificate to Trusted Root (so the
    # chain is trusted) and Trusted Publishers (so the installer runs without an
    # "unknown publisher" prompt). Idempotent — skip stores that already have it.
    foreach ($store in @('Root', 'TrustedPublisher')) {
        $path = "Cert:\CurrentUser\$store"
        $present = Get-ChildItem $path -ErrorAction SilentlyContinue |
            Where-Object { $_.Thumbprint -eq $Cert.Thumbprint }
        if ($present) {
            continue
        }
        Write-Host "[sign] Trusting certificate in CurrentUser\$store"
        $tmp = Join-Path $env:TEMP "ee-signcert-$($Cert.Thumbprint).cer"
        Export-Certificate -Cert $Cert -FilePath $tmp -Force | Out-Null
        Import-Certificate -FilePath $tmp -CertStoreLocation $path | Out-Null
        Remove-Item $tmp -ErrorAction SilentlyContinue
    }
}

$cert = Get-OrCreateSigningCert -SubjectDn $subjectDn
Set-CertTrusted -Cert $cert

$signed = 0
foreach ($file in $Files) {
    if (-not (Test-Path $file)) {
        Write-Warning "[sign] Skipping missing file: $file"
        continue
    }
    Write-Host "[sign] Signing $file"
    $result = Set-AuthenticodeSignature -FilePath $file -Certificate $cert -HashAlgorithm SHA256
    if ($result.Status -ne 'Valid') {
        throw "Signing failed for $file : $($result.Status) - $($result.StatusMessage)"
    }
    $signed++
}

Write-Host "[sign] Done. Signed $signed file(s) with $subjectDn."
