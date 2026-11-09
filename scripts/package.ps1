# Builds a release and packages Know Giver, or a branded build, for Windows.
#
#   powershell -ExecutionPolicy Bypass -File scripts\package.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\package.ps1 -Brand branding\acme
#
# -Brand picks the brand folder (brand.toml); without it, branding\default.
# Output in dist\, named after the brand's data_dir (KnowGiver by default):
#   <DataDir>-<version>-windows-x64.zip     portable build (always)
#   <DataDir>-<version>-setup.exe           installer (when Inno Setup 6 is installed)

param([switch]$SkipInstaller, [string]$Brand = '', [switch]$Commercial, [switch]$RequireSignature)

# Reads `key = "value"` from one [section] of a brand.toml.
function Get-BrandValue([string[]]$Lines, [string]$Section, [string]$Key) {
    $current = ''
    foreach ($line in $Lines) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^\[(.+)\]$') { $current = $Matches[1]; continue }
        if ($current -eq $Section -and $trimmed -match ('^' + [regex]::Escape($Key) + '\s*=\s*"([^"]*)"')) {
            return $Matches[1]
        }
    }
    return ''
}

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$previousBrand = $env:KG_BRAND_DIR
Push-Location $root
try {
    $versionLine = Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1
    $version = $versionLine.Matches[0].Groups[1].Value

    $brandDir = if ([System.IO.Path]::IsPathRooted($Brand)) { [System.IO.Path]::GetFullPath($Brand) } elseif ($Brand) { [System.IO.Path]::GetFullPath((Join-Path $root $Brand)) } else { Join-Path $root 'branding\default' }
    $brandFile = Join-Path $brandDir 'brand.toml'
    if (-not (Test-Path -LiteralPath $brandFile)) { throw "Brand file not found: $brandFile" }
    $env:KG_BRAND_DIR = $brandDir
    $target = if ($env:CARGO_TARGET_DIR) { [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) } else { Join-Path $root 'target' }
    cargo build -p kg-brand --example metadata
    if ($LASTEXITCODE -ne 0) { throw 'Invalid brand metadata' }
    $metadata = Join-Path $target 'debug\examples\metadata.exe'
    $appName = & $metadata name
    $appDir = & $metadata data_dir
    $appGuid = & $metadata guid
    $publisher = & $metadata company
    $publisherUrl = & $metadata url
    $icon = & $metadata icon_ico
    if ($Commercial) {
        & $metadata release-ready
        if ($LASTEXITCODE -ne 0) { throw 'Client legal/commercial review is incomplete' }
        $RequireSignature = $true
    }
    if ($RequireSignature -and -not $env:KG_SIGN_CERT_SHA1) { throw 'Signing certificate is required' }
    $iscc = @(
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
    ) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $SkipInstaller -and -not $iscc) { throw 'Inno Setup 6 is required; use -SkipInstaller for a portable preview' }
    Write-Host "Brand: $appName ($brandDir)"

    & (Join-Path $PSScriptRoot 'build-llama.ps1')
    if ($LASTEXITCODE -ne 0) { throw "llama.cpp build failed" }
    cargo build --release -p know-giver
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    $dist = Join-Path $root 'dist'
    $stage = [System.IO.Path]::GetFullPath((Join-Path $dist $appDir))
    $expectedStage = [System.IO.Path]::GetFullPath((Join-Path $root "dist\$appDir"))
    if ($stage -ne $expectedStage -or -not $stage.StartsWith([System.IO.Path]::GetFullPath($dist) + [System.IO.Path]::DirectorySeparatorChar)) {
        throw 'Refusing to clean a staging directory outside dist'
    }
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
    New-Item -ItemType Directory -Force $stage | Out-Null
    Copy-Item (Join-Path $target 'release\know-giver.exe') $stage
    & (Join-Path $PSScriptRoot 'sign-windows.ps1') -Path (Join-Path $stage 'know-giver.exe') -Required:$RequireSignature
    Copy-Item (Join-Path $target 'release\local-runtime') $stage -Recurse
    foreach ($dll in @('vcruntime140.dll', 'vcruntime140_1.dll')) {
        $runtimeDll = Join-Path (Join-Path $target 'release\local-runtime') $dll
        if (-not (Test-Path -LiteralPath $runtimeDll)) {
            throw "The packaged Windows build is missing the app runtime dependency $dll"
        }
        Copy-Item -LiteralPath $runtimeDll -Destination $stage -Force
    }
    Copy-Item '.env.example' (Join-Path $stage '.env.example')
    Copy-Item 'packaging\windows\README.*.txt' $stage
    if (-not (Test-Path -LiteralPath (Join-Path $stage 'local-runtime\vulkan\llama-server.exe'))) {
        throw 'The packaged Windows build is missing the Vulkan llama-server'
    }
    foreach ($dll in @('msvcp140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll', 'vcomp140.dll')) {
        if (-not (Test-Path -LiteralPath (Join-Path $stage "local-runtime\$dll"))) {
            throw "The packaged Windows build is missing the CPU runtime dependency $dll"
        }
        if (-not (Test-Path -LiteralPath (Join-Path $stage "local-runtime\vulkan\$dll"))) {
            throw "The packaged Windows build is missing the Vulkan runtime dependency $dll"
        }
    }
    if (-not (Test-Path -LiteralPath (Join-Path $stage 'local-runtime\vulkan\vulkan-1.dll'))) {
        throw 'The packaged Windows build is missing the Vulkan loader'
    }
    if (-not (Test-Path -LiteralPath (Join-Path $stage 'local-runtime\Vulkan-SDK-LICENSE.txt'))) {
        throw 'The packaged Windows build is missing the Vulkan SDK license registry'
    }

    $zip = Join-Path $dist "$appDir-$version-windows-x64.zip"
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip -Force
    Write-Host "Portable build: $zip"

    if ($SkipInstaller) { return }
    & $iscc "/DAppVersion=$version" "/DSourceDir=$stage" "/DRepoRoot=$root" "/O$dist" `
        "/DAppName=$appName" "/DAppDir=$appDir" "/DAppGuid=$appGuid" `
        "/DAppPublisher=$publisher" "/DAppUrl=$publisherUrl" "/DAppIcon=$icon" `
        'packaging\windows\know-giver.iss'
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed" }
    & (Join-Path $PSScriptRoot 'sign-windows.ps1') -Path (Join-Path $dist "$appDir-$version-setup.exe") -Required:$RequireSignature
}
finally {
    $env:KG_BRAND_DIR = $previousBrand
    Pop-Location
}
