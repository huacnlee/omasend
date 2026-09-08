# Offline installer acceptance tests. Network and Start menu writes are mocked.
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Run these installer tests on Windows.' }
$installer = Join-Path (Split-Path $PSScriptRoot -Parent) 'install.ps1'
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('omasend-installer-test-' + [Guid]::NewGuid().ToString('N'))
$oldArchitecture = $env:PROCESSOR_ARCHITECTURE
$oldWowArchitecture = $env:PROCESSOR_ARCHITEW6432
$script:shortcutSaved = $null
$script:latestCalls = 0
$script:downloadCalls = 0

function Invoke-RestMethod {
    param([string]$Uri)
    if ($Uri -ne 'https://api.github.com/repos/huacnlee/omasend/releases/latest') { throw "Unexpected API request: $Uri" }
    $script:latestCalls++
    return @{ tag_name = 'v0.1.0' }
}

function Invoke-WebRequest {
    param([switch]$UseBasicParsing, [string]$Uri, [string]$OutFile)
    $prefix = 'https://github.com/huacnlee/omasend/releases/download/v0.1.0/'
    if (-not $Uri.StartsWith($prefix)) { throw "Unexpected download: $Uri" }
    $script:downloadCalls++
    Copy-Item -LiteralPath (Join-Path $testRoot $Uri.Substring($prefix.Length)) -Destination $OutFile
}

function New-Object {
    param([string]$ComObject)
    if ($ComObject -ne 'WScript.Shell') { throw "Unexpected COM object: $ComObject" }
    $shell = [PSCustomObject]@{}
    $shell | Add-Member -MemberType ScriptMethod -Name CreateShortcut -Value {
        param($Path)
        $shortcut = [PSCustomObject]@{ Path = $Path; TargetPath = ''; WorkingDirectory = ''; IconLocation = '' }
        $shortcut | Add-Member -MemberType ScriptMethod -Name Save -Value { $script:shortcutSaved = $this }
        return $shortcut
    }
    return $shell
}

function Assert-True($Condition, $Message) {
    if (-not $Condition) { throw "Assertion failed: $Message" }
}

function Assert-Fails($Action, $Message) {
    $failure = $null
    try { & $Action } catch { $failure = $_.Exception.Message }
    Assert-True ($null -ne $failure -and $failure.Contains($Message)) "Expected '$Message', received '$failure'"
}

function Write-Checksums {
    $digest = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath (Join-Path $testRoot 'SHA256SUMS') -Value "$digest  $asset" -Encoding ascii
}

try {
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    $env:PROCESSOR_ARCHITECTURE = 'AMD64'
    $env:PROCESSOR_ARCHITEW6432 = $null
    $payload = Join-Path $testRoot 'payload'
    New-Item -ItemType Directory -Path $payload | Out-Null
    Set-Content -LiteralPath (Join-Path $payload 'omasend.exe') -Value 'fixture executable v1'
    Set-Content -LiteralPath (Join-Path $payload 'LICENSE') -Value 'fixture license'
    $asset = 'omasend-0.1.0-x86_64-pc-windows-msvc.zip'
    $archive = Join-Path $testRoot $asset
    Compress-Archive -Path (Join-Path $payload '*') -DestinationPath $archive
    Write-Checksums
    $destination = Join-Path $testRoot 'install with spaces'

    & $installer -InstallDir $destination
    Assert-True ($script:latestCalls -eq 1) 'latest release lookup'
    Assert-True (Test-Path -LiteralPath (Join-Path $destination 'omasend.exe')) 'executable installed'
    Assert-True (Test-Path -LiteralPath (Join-Path $destination 'LICENSE')) 'supporting files installed'
    Assert-True ($script:shortcutSaved.TargetPath -eq (Join-Path $destination 'omasend.exe')) 'shortcut targets installed executable'
    Assert-True ($script:shortcutSaved.WorkingDirectory -eq $destination) 'shortcut working directory'
    Write-Host 'PASS latest release installation and shortcut'

    Set-Content -LiteralPath (Join-Path $destination 'omasend.exe') -Value 'old executable'
    & $installer -Version v0.1.0 -InstallDir $destination
    Assert-True ($script:latestCalls -eq 1) 'explicit version avoids latest lookup'
    Assert-True ((Get-Content (Join-Path $destination 'omasend.exe')) -eq 'fixture executable v1') 'upgrade replaces executable'
    Write-Host 'PASS explicit version upgrade'

    $before = Get-Content (Join-Path $destination 'omasend.exe')
    Set-Content -LiteralPath (Join-Path $testRoot 'SHA256SUMS') -Value ((('0' * 64) + "  $asset")) -Encoding ascii
    Assert-Fails { & $installer -Version 0.1.0 -InstallDir $destination } 'Checksum verification failed'
    Assert-True ((Get-Content (Join-Path $destination 'omasend.exe')) -eq $before) 'failed verification preserves installed version'
    Write-Host 'PASS checksum rejection preserves installation'

    Write-Checksums
    Add-Content -LiteralPath (Join-Path $testRoot 'SHA256SUMS') -Value (Get-Content (Join-Path $testRoot 'SHA256SUMS'))
    Assert-Fails { & $installer -Version 0.1.0 -InstallDir $destination } 'Missing or ambiguous checksum'
    Write-Host 'PASS duplicate checksum rejection'

    $downloadsBefore = $script:downloadCalls
    Assert-Fails { & $installer -Version '../bad' -InstallDir $destination } 'Version must be'
    $env:PROCESSOR_ARCHITECTURE = 'ARM64'
    Assert-Fails { & $installer -Version 0.1.0 -InstallDir $destination } 'x86_64 only'
    Assert-True ($script:downloadCalls -eq $downloadsBefore) 'invalid input avoids downloads'
    Write-Host 'PASS invalid version and unsupported architecture'

    $env:PROCESSOR_ARCHITECTURE = 'AMD64'
    Remove-Item -LiteralPath (Join-Path $payload 'omasend.exe')
    Remove-Item -LiteralPath $archive
    Compress-Archive -Path (Join-Path $payload '*') -DestinationPath $archive
    Write-Checksums
    Assert-Fails { & $installer -Version 0.1.0 -InstallDir $destination } 'missing omasend.exe'
    Assert-True ((Get-Content (Join-Path $destination 'omasend.exe')) -eq $before) 'missing executable preserves installed version'
    Write-Host 'PASS incomplete release rejection'
} finally {
    $env:PROCESSOR_ARCHITECTURE = $oldArchitecture
    $env:PROCESSOR_ARCHITEW6432 = $oldWowArchitecture
    Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
}
