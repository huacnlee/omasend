# Install the published OmaSend app for the current Windows user.
[CmdletBinding()]
param(
    [string]$Version = 'latest',
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\OmaSend')
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
if ($env:OS -ne 'Windows_NT') { throw 'This installer requires Windows. Use install.sh on macOS or Linux.' }
$architecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
$target = switch ($architecture) {
    'AMD64' { 'x86_64-pc-windows-msvc' }
    'ARM64' { throw 'Windows releases currently support x86_64 only. Build from source on ARM64.' }
    default { throw "Unsupported CPU architecture: $architecture" }
}
$repository = 'https://github.com/huacnlee/omasend'
if ($Version -eq 'latest') {
    $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/huacnlee/omasend/releases/latest'
    $Version = $release.tag_name
}
$Version = $Version -replace '^v', ''
if ($Version -notmatch '^\d+\.\d+\.\d+([.-][A-Za-z0-9.-]+)?$') { throw 'Version must be a release version such as 0.1.0 or v0.1.0.' }
$asset = "omasend-$Version-$target.zip"
$base = "$repository/releases/download/v$Version"
$work = Join-Path ([IO.Path]::GetTempPath()) ('omasend-install-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    Write-Host "Downloading OmaSend $Version for $target…"
    $archive = Join-Path $work $asset
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile $archive
    $checksums = Join-Path $work 'SHA256SUMS'
    Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS" -OutFile $checksums
    $expected = @(Get-Content $checksums | ForEach-Object {
        if ($_ -match '^([a-fA-F0-9]{64})\s+\*?(.+)$' -and $Matches[2] -ceq $asset) { $Matches[1] }
    })
    if ($expected.Count -ne 1) { throw "Missing or ambiguous checksum for $asset" }
    if ((Get-FileHash $archive -Algorithm SHA256).Hash -ine $expected[0]) { throw 'Checksum verification failed; nothing was installed.' }
    $unpacked = Join-Path $work 'unpacked'
    Expand-Archive -LiteralPath $archive -DestinationPath $unpacked
    if (-not (Test-Path -LiteralPath (Join-Path $unpacked 'omasend.exe') -PathType Leaf)) { throw 'Release archive is missing omasend.exe.' }
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    # Windows refuses replacement while the app is running; report the failure.
    Copy-Item -Path (Join-Path $unpacked '*') -Destination $InstallDir -Recurse -Force
    $startMenu = Join-Path ([Environment]::GetFolderPath('Programs')) 'OmaSend.lnk'
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($startMenu)
    $shortcut.TargetPath = Join-Path $InstallDir 'omasend.exe'
    $shortcut.WorkingDirectory = $InstallDir
    $shortcut.IconLocation = "$InstallDir\omasend.exe,0"
    $shortcut.Save()
    Write-Host "Installed $InstallDir. Launch OmaSend from the Start menu."
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
