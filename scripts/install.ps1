# Nyra installer for native Windows (PowerShell 5.1+)
# Usage: irm https://raw.githubusercontent.com/nyra-lang/nyra/main/scripts/install.ps1 | iex
#        .\install.ps1 -Version 1.1.0 -InstallDir "$env:USERPROFILE\.nyra"

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:USERPROFILE\.nyra",
    [string]$Repo = "nyra-lang/nyra"
)

$ErrorActionPreference = "Stop"

function Write-Info($msg) { Write-Host $msg }

$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    "AMD64" { "x86_64" }
    "ARM64" { "aarch64" }
    default { throw "unsupported CPU: $($env:PROCESSOR_ARCHITECTURE) (need x86_64 or ARM64)" }
}

$asset = "nyra-${arch}-windows.zip"
$api = "https://api.github.com/repos/$Repo/releases"

if ($Version -eq "latest") {
    $release = Invoke-RestMethod -Uri "$api/latest"
} else {
    $tag = if ($Version -match '^v') { $Version } else { "v$Version" }
    $release = Invoke-RestMethod -Uri "$api/tags/$tag"
}

$assetObj = $release.assets | Where-Object { $_.name -eq $asset } | Select-Object -First 1
if (-not $assetObj) {
    throw "release asset not found: $asset`nPush a tag and wait for the Release workflow, or pass -Version matching an existing release."
}

$tmp = Join-Path $env:TEMP ("nyra-install-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
$zipPath = Join-Path $tmp $asset

Write-Info "Downloading $asset ..."
Invoke-WebRequest -Uri $assetObj.browser_download_url -OutFile $zipPath -UseBasicParsing

# Optional checksum
$sums = $release.assets | Where-Object { $_.name -eq "SHA256SUMS" } | Select-Object -First 1
if ($sums) {
    $sumsPath = Join-Path $tmp "SHA256SUMS"
    Invoke-WebRequest -Uri $sums.browser_download_url -OutFile $sumsPath -UseBasicParsing
    $line = Get-Content $sumsPath | Where-Object { $_ -match [regex]::Escape($asset) } | Select-Object -First 1
    if ($line) {
        $expected = ($line -split '\s+')[0].ToLower()
        $actual = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) {
            throw "checksum mismatch for $asset"
        }
        Write-Info "Checksum verified."
    }
}

if (Test-Path $InstallDir) {
    Remove-Item -Recurse -Force $InstallDir
}
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Expand-Archive -Path $zipPath -DestinationPath $InstallDir -Force

$nyraExe = Join-Path $InstallDir "bin\nyra.exe"
if (-not (Test-Path $nyraExe)) {
    throw "install failed: $nyraExe missing"
}

$env:NYRA_HOME = $InstallDir
$env:PATH = "$InstallDir\bin;$env:PATH"

# User PATH (idempotent)
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir\bin*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir\bin;$userPath", "User")
}
[Environment]::SetEnvironmentVariable("NYRA_HOME", $InstallDir, "User")

Write-Info ""
Write-Info "Nyra installed to $InstallDir"
& $nyraExe --version

Write-Info ""
Write-Info "================================================================"
Write-Info "  Nyra is installed — follow these steps on Windows"
Write-Info "================================================================"
Write-Info ""
Write-Info "1. Close this window and open a new PowerShell or Windows Terminal"
Write-Info "   (user PATH and NYRA_HOME were updated)."
Write-Info ""
Write-Info "2. Verify Nyra is available:"
Write-Info "     Get-Command nyra"
Write-Info "     nyra --version"
Write-Info ""
Write-Info "3. Install LLVM/clang (required to compile and run .ny programs):"
Write-Info "     winget install LLVM.LLVM"
Write-Info "   Or download from: https://releases.llvm.org/"
Write-Info "   Then reopen the terminal so clang is on PATH."
Write-Info ""
Write-Info "4. Optional — load Nyra env in the current session:"
Write-Info "     . `"$InstallDir\env.ps1`""
Write-Info ""
Write-Info "5. Create your first project:"
Write-Info "     mkdir myapp"
Write-Info "     cd myapp"
Write-Info "     nyra pkg init"
Write-Info "     nyra run ."
Write-Info ""
Write-Info "Install location: $InstallDir"
Write-Info "Docs: https://github.com/nyra-lang/nyra"
Write-Info ""
Write-Info "macOS / Linux? Use:"
Write-Info "  curl -fsSL https://raw.githubusercontent.com/nyra-lang/nyra/main/scripts/install.sh | sh"

Remove-Item -Recurse -Force $tmp
