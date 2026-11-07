$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$target = 'x86_64-pc-windows-msvc'
$releaseExe = Join-Path $repoRoot "target\$target\release\autoclicker.exe"
$distDir = Join-Path $repoRoot 'dist'
$stageDir = Join-Path $distDir 'AutoClicker-windows-x64'
$zipPath = Join-Path $distDir 'AutoClicker-windows-x64.zip'
$standaloneExe = Join-Path $distDir 'AutoClicker.exe'
$sourceZipPath = Join-Path $distDir 'AutoClicker-source.zip'

Push-Location $repoRoot
try {
    cargo build --release --target $target
    if (-not (Test-Path $releaseExe)) {
        throw "Release executable was not produced: $releaseExe"
    }

    New-Item -ItemType Directory -Path $distDir -Force | Out-Null
    if (Test-Path $stageDir) {
        Remove-Item -Path $stageDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $stageDir -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $stageDir 'assets') -Force | Out-Null

    Copy-Item $releaseExe (Join-Path $stageDir 'AutoClicker.exe')
    Copy-Item $releaseExe $standaloneExe
    Copy-Item (Join-Path $repoRoot 'README.md') $stageDir
    Copy-Item (Join-Path $repoRoot 'LICENSE') $stageDir
    Copy-Item (Join-Path $repoRoot 'assets\autoclicker.ico') (Join-Path $stageDir 'assets')
    Copy-Item (Join-Path $repoRoot 'assets\autoclicker-logo.png') (Join-Path $stageDir 'assets')

    if (Test-Path $zipPath) {
        Remove-Item -Path $zipPath -Force
    }
    Compress-Archive -Path (Join-Path $stageDir '*') -DestinationPath $zipPath -CompressionLevel Optimal

    if (Test-Path $sourceZipPath) {
        Remove-Item -Path $sourceZipPath -Force
    }
    & git archive --format=zip "--output=$sourceZipPath" HEAD
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not create the source archive from the current Git commit.'
    }

    Write-Host "Standalone executable: $standaloneExe"
    Write-Host "Runtime package: $zipPath"
    Write-Host "Source archive: $sourceZipPath"
}
finally {
    Pop-Location
}
