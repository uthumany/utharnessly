[CmdletBinding()]
param(
  [string]$Version = $env:UTHARNESS_VERSION,
  [string]$InstallDir = $env:UTHARNESS_INSTALL_DIR
)

$ErrorActionPreference = 'Stop'
$Repository = if ($env:UTHARNESS_REPOSITORY) { $env:UTHARNESS_REPOSITORY } else { 'uthumany/utharnessly' }
if (-not $Version) { $Version = 'latest' }
if (-not $InstallDir) { $InstallDir = Join-Path $HOME '.local\bin' }

if ($Version -eq 'latest') {
  $ReleaseUrl = "https://github.com/$Repository/releases/latest/download"
  $VersionLabel = 'latest'
} else {
  $Version = $Version.TrimStart('v')
  $ReleaseUrl = "https://github.com/$Repository/releases/download/v$Version"
  $VersionLabel = "v$Version"
}

$asset = 'utharnessly-windows-x64.zip'
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("utharnessly-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temp | Out-Null
try {
  $archive = Join-Path $temp $asset
  $checksums = Join-Path $temp 'SHA256SUMS'
  Write-Host "Downloading utharnessly $VersionLabel (windows/x64)…"
  try {
    Invoke-WebRequest "$ReleaseUrl/$asset" -OutFile $archive
  } catch {
    throw "No matching Windows release archive was found. Build from source with Git, Rust, Node 22, and pnpm: git clone https://github.com/$Repository.git; cd utharnessly; cargo build --release; pnpm --dir ui install; pnpm --dir ui build"
  }
  try {
    Invoke-WebRequest "$ReleaseUrl/SHA256SUMS" -OutFile $checksums
    $line = Get-Content $checksums | Where-Object { $_ -match [regex]::Escape($asset) } | Select-Object -First 1
    if ($line) {
      $expected = ($line -split '\s+')[0]
      $actual = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
      if ($expected.ToLowerInvariant() -ne $actual) { throw 'checksum verification failed' }
    }
  } catch [System.Net.WebException] {
    Write-Warning 'SHA256SUMS was not available; continuing with the signed HTTPS download.'
  }
  Expand-Archive -Path $archive -DestinationPath $temp -Force
  $package = Get-ChildItem $temp -Directory | Where-Object { $_.Name -like 'utharnessly-*' } | Select-Object -First 1
  if (-not $package) { throw 'release archive has no utharnessly directory' }
  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  Copy-Item (Join-Path $package.FullName 'utharness.exe') (Join-Path $InstallDir 'utharness.exe') -Force
  New-Item -ItemType Directory -Path (Join-Path $InstallDir 'utharnessly-ui') -Force | Out-Null
  Copy-Item (Join-Path $package.FullName 'ui\*') (Join-Path $InstallDir 'utharnessly-ui') -Recurse -Force
  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  if (($userPath -split ';') -notcontains $InstallDir) {
    [Environment]::SetEnvironmentVariable('Path', (($userPath, $InstallDir) -join ';'), 'User')
  }
  Write-Host "Installed utharness.exe to $InstallDir. Open a new terminal, then run: utharness"
} finally {
  Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
}
