# CI only: import a client certificate into the runner's ephemeral user store.
$ErrorActionPreference = 'Stop'
if (-not $env:RUNNER_TEMP -or -not $env:KG_CERT_PFX_B64 -or -not $env:KG_CERT_PASSWORD) { throw 'Signing certificate and password secrets are required' }
$certFile = Join-Path $env:RUNNER_TEMP ('signing-' + [guid]::NewGuid() + '.pfx')
try {
    [IO.File]::WriteAllBytes($certFile, [Convert]::FromBase64String($env:KG_CERT_PFX_B64))
    $password = ConvertTo-SecureString $env:KG_CERT_PASSWORD -AsPlainText -Force
    $cert = Import-PfxCertificate -FilePath $certFile -CertStoreLocation Cert:\CurrentUser\My -Password $password
    if ($cert.Thumbprint -notmatch '^[0-9A-Fa-f]{40}$') { throw 'Invalid imported certificate' }
    "KG_SIGN_CERT_SHA1=$($cert.Thumbprint)" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
    $sdk = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $tool = Get-ChildItem -LiteralPath $sdk -Filter signtool.exe -File -Recurse | Where-Object FullName -Match '\\x64\\signtool.exe$' | Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $tool) { throw 'signtool.exe was not found in the Windows SDK' }
    "KG_SIGNTOOL=$($tool.FullName)" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
}
finally {
    Remove-Item -LiteralPath $certFile -Force -ErrorAction SilentlyContinue
}
