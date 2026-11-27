param(
    [switch]$FromSource,
    [switch]$Uninstall,
    [string]$Version = $(if ($env:TPHPC_VERSION) { $env:TPHPC_VERSION } else { "latest" }),
    [string]$TypePhpHome = $(if ($env:TPHPC_HOME) { $env:TPHPC_HOME } else { "$env:USERPROFILE\.typephp" }),
    [string]$InstallDir = $(if ($env:TPHPC_INSTALL_DIR) { $env:TPHPC_INSTALL_DIR } else { "$env:USERPROFILE\.typephp\bin" }),
    [string]$Repo = $(if ($env:TPHPC_REPO) { $env:TPHPC_REPO } else { "chaungoclong/typephp" })
)

$ErrorActionPreference = "Stop"

if ($Uninstall) {
    if (Test-Path $TypePhpHome) {
        Remove-Item -Recurse -Force $TypePhpHome
        Write-Host "[typephp] Uninstalled TypePHP from $TypePhpHome"
    }
    exit 0
}

function Get-TargetArch {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default { throw "Unsupported Windows architecture: $env:PROCESSOR_ARCHITECTURE" }
    }
}

function Ensure-Rust {
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path (Join-Path $cargoBin "cargo.exe")) {
        $env:Path = "$cargoBin;$env:Path"
        return
    }
    if (Get-Command cargo -ErrorAction SilentlyContinue) { return }

    Write-Host "[typephp] Rust/Cargo not found. Installing Rust automatically via rustup-init.exe..."
    $tmpExe = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $tmpExe -UseBasicParsing
    & $tmpExe -y --no-modify-path
    Remove-Item -Force $tmpExe -ErrorAction SilentlyContinue
    $env:Path = "$cargoBin;$env:Path"
}

function Ensure-PhpAndComposer {
    if (-not (Get-Command php -ErrorAction SilentlyContinue)) {
        if (Get-Command winget -ErrorAction SilentlyContinue) {
            Write-Host "[typephp] PHP not found. Installing PHP via winget..."
            winget install --id PHP.PHP -e --accept-package-agreements --accept-source-agreements
        } else {
            Write-Warning "[typephp] PHP not found. Please install PHP 8.1+."
        }
    }
    if (-not (Get-Command composer -ErrorAction SilentlyContinue)) {
        if (Get-Command winget -ErrorAction SilentlyContinue) {
            Write-Host "[typephp] Composer not found. Installing Composer via winget..."
            winget install --id Composer.Composer -e --accept-package-agreements --accept-source-agreements
        }
    }
    $phpstanBin = Join-Path $TypePhpHome "vendor\bin\phpstan"
    if ((Get-Command composer -ErrorAction SilentlyContinue) -and (-not (Test-Path $phpstanBin))) {
        Write-Host "[typephp] Installing global PHPStan Level 9 into $TypePhpHome..."
        New-Item -ItemType Directory -Force -Path $TypePhpHome | Out-Null
        $composerJson = Join-Path $TypePhpHome "composer.json"
        if (-not (Test-Path $composerJson)) {
            '{"name":"typephp/global-tools","require":{"phpstan/phpstan":"^2.0"}}' | Set-Content -Path $composerJson -Encoding UTF8
        }
        composer --working-dir="$TypePhpHome" install --no-interaction --prefer-dist --quiet
    }
}

function Build-FromSource {
    Ensure-Rust
    $scriptDir = Split-Path -Parent $MyInvocation.ScriptName
    $localCargo = if ($scriptDir) { Join-Path (Split-Path -Parent $scriptDir) "Cargo.toml" } else { "" }

    if ($localCargo -and (Test-Path $localCargo)) {
        Write-Host "[typephp] Building tphpc from local source..."
        cargo build --release --manifest-path $localCargo
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
        New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        Copy-Item -Force (Join-Path (Split-Path -Parent $localCargo) "target\release\tphpc.exe") (Join-Path $InstallDir "tphpc.exe")
    } else {
        $tmpDir = Join-Path $env:TEMP ("tphpc-build-" + [Guid]::NewGuid().ToString("N"))
        New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
        try {
            Write-Host "[typephp] Cloning https://github.com/$Repo.git..."
            git clone --depth 1 "https://github.com/$Repo.git" "$tmpDir\typephp"
            cargo build --release --manifest-path "$tmpDir\typephp\Cargo.toml"
            if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
            New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
            Copy-Item -Force "$tmpDir\typephp\target\release\tphpc.exe" (Join-Path $InstallDir "tphpc.exe")
        } finally {
            Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
        }
    }
}

function Install-FromRelease {
    param([string]$Target)
    $asset = "tphpc-windows-$Target.zip"
    $url = if ($Version -eq "latest") {
        "https://github.com/$Repo/releases/latest/download/$asset"
    } else {
        "https://github.com/$Repo/releases/download/$Version/$asset"
    }
    $tmpDir = Join-Path $env:TEMP ("tphpc-dl-" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    try {
        $zipPath = Join-Path $tmpDir $asset
        Write-Host "[typephp] Downloading $url..."
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
        Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force
        New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        Copy-Item -Force (Join-Path $tmpDir "tphpc.exe") (Join-Path $InstallDir "tphpc.exe")
        return $true
    } catch {
        return $false
    } finally {
        Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
    }
}

$arch = Get-TargetArch
Write-Host "[typephp] Detected platform: windows-$arch"
Ensure-PhpAndComposer

if ($FromSource) {
    Build-FromSource
} else {
    if (-not (Install-FromRelease -Target $arch)) {
        Write-Host "[typephp] Prebuilt release binary not available; auto-installing Rust & building from source..."
        Build-FromSource
    }
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) { $userPath = "" }
if ($userPath.Split(";") -notcontains $InstallDir) {
    $newPath = if ($userPath.Trim() -eq "") { $InstallDir } else { "$userPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "[typephp] Added $InstallDir to User PATH."
}

Write-Host "[typephp] Installed tphpc.exe to: $(Join-Path $InstallDir 'tphpc.exe')"
