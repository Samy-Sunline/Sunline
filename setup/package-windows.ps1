param(
    [Parameter(Mandatory)] [string]$LauncherExe,
    [Parameter(Mandatory)] [string]$VictoryRepo,
    [Parameter(Mandatory)] [string]$OutputDirectory,
    [Parameter(Mandatory)]
    [ValidatePattern('^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$')]
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$engineBuild = '63b902fe1720a5188ee79d94a88c8de0470f6349 p2'
$marker = Join-Path $VictoryRepo 'sunline-build.txt'
if ((Get-Content -LiteralPath $marker -Raw).Trim() -ne $engineBuild) {
    throw "Expected VIctory-NVIDIA build $engineBuild."
}
$files = [ordered]@{
    'Sunline.exe' = $LauncherExe
    'README.md' = (Join-Path $repo 'README.md')
    'LICENSE' = (Join-Path $repo 'LICENSE')
    'patches/nv-ours.patch' = (Join-Path $repo 'patches/nv-ours.patch')
    'patches/newbase-nvidia.patch' = (Join-Path $repo 'patches/newbase-nvidia.patch')
    'setup/setup-nv.bat' = (Join-Path $PSScriptRoot 'setup-nv.bat')
    'setup/setup-new.bat' = (Join-Path $PSScriptRoot 'setup-new.bat')
    'setup/build-progress.ps1' = (Join-Path $PSScriptRoot 'build-progress.ps1')
    'VIctory-NVIDIA/LICENSE' = (Join-Path $VictoryRepo 'LICENSE')
    'VIctory-NVIDIA/sunline-build.txt' = $marker
    'VIctory-NVIDIA/target/release/victory.exe' = (Join-Path $VictoryRepo 'target/release/victory.exe')
    'VIctory-NVIDIA/scripts/play-foxhound.ps1' = (Join-Path $VictoryRepo 'scripts/play-foxhound.ps1')
    'VIctory-NVIDIA/scripts/get-d3d12.ps1' = (Join-Path $VictoryRepo 'scripts/get-d3d12.ps1')
    'VIctory-NVIDIA/scripts/get-nvidia-sdks.ps1' = (Join-Path $VictoryRepo 'scripts/get-nvidia-sdks.ps1')
}
foreach ($file in $files.Values) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Required package file is missing: $file"
    }
}

if (-not (Test-Path -LiteralPath $OutputDirectory -PathType Container)) {
    New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
}
$output = (Resolve-Path -LiteralPath $OutputDirectory).Path
$zip = Join-Path $output "Sunline-$Version.zip"
$checksum = Join-Path $output "Sunline-$Version.sha256"
if ((Test-Path -LiteralPath $zip) -or (Test-Path -LiteralPath $checksum)) {
    throw "Output already exists for $Version in $output."
}

$staging = Join-Path ([IO.Path]::GetTempPath()) ("sunline-" + [guid]::NewGuid())
$root = Join-Path $staging 'Sunline'
$stagedZip = Join-Path $staging "Sunline-$Version.zip"
try {
    foreach ($entry in $files.GetEnumerator()) {
        $target = Join-Path $root $entry.Key
        [IO.Directory]::CreateDirectory((Split-Path -Parent $target)) | Out-Null
        Copy-Item -LiteralPath $entry.Value -Destination $target
    }
    Compress-Archive -LiteralPath $root -DestinationPath $stagedZip
    $hash = (Get-FileHash -LiteralPath $stagedZip -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::Copy($stagedZip, $zip)
    Set-Content -LiteralPath $checksum -Encoding ascii "$hash  $(Split-Path -Leaf $zip)"
    Write-Output $zip
} finally {
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\', '/')
    if ((Split-Path -Parent $staging) -eq $tempRoot -and
        (Split-Path -Leaf $staging) -match '^sunline-[0-9a-f-]{36}$' -and
        (Test-Path -LiteralPath $staging)) {
        Remove-Item -LiteralPath $staging -Recurse -Force
    }
}
