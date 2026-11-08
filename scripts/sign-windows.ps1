param([Parameter(Mandatory=$true)][string]$Path, [switch]$Required)
$ErrorActionPreference = 'Stop'
if (-not $env:KG_SIGN_CERT_SHA1) {
    if ($Required) { throw 'KG_SIGN_CERT_SHA1 is required for a signed client release' }
    Write-Host 'Unsigned preview build (no signing certificate configured).'
    return
}
if ($env:KG_SIGN_CERT_SHA1 -notmatch '^[a-fA-F0-9]{40}$') { throw 'Invalid certificate thumbprint' }
if (-not $env:KG_SIGN_TIMESTAMP -or $env:KG_SIGN_TIMESTAMP -notmatch '^https://') { throw 'Set KG_SIGN_TIMESTAMP to the certificate provider HTTPS timestamp service' }
$signTool = if ($env:KG_SIGNTOOL) { $env:KG_SIGNTOOL } else { (Get-Command signtool.exe -ErrorAction Stop).Source }
& $signTool sign /sha1 $env:KG_SIGN_CERT_SHA1 /fd SHA256 /tr $env:KG_SIGN_TIMESTAMP /td SHA256 $Path
if ($LASTEXITCODE -ne 0) { throw 'Code signing failed' }
& $signTool verify /pa $Path
if ($LASTEXITCODE -ne 0) { throw 'Signature verification failed' }
