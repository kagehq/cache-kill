# CacheKill Install Script for Windows PowerShell
# This script downloads and installs the latest CacheKill release

param(
    [switch]$Force
)

# Configuration
$REPO = "kagehq/cache-kill"
$BINARY_NAME = "cachekill"
$INSTALL_DIR = "$env:USERPROFILE\.local\bin"

# Colors for output (PowerShell 5+)
function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White"
    )
    Write-Host $Message -ForegroundColor $Color
}

function Write-Info {
    param([string]$Message)
    Write-ColorOutput "[INFO] $Message" "Cyan"
}

function Write-Success {
    param([string]$Message)
    Write-ColorOutput "[SUCCESS] $Message" "Green"
}

function Write-Warning {
    param([string]$Message)
    Write-ColorOutput "[WARNING] $Message" "Yellow"
}

function Write-Error {
    param([string]$Message)
    Write-ColorOutput "[ERROR] $Message" "Red"
}

# Detect architecture
function Get-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default { return "x86_64" }
    }
}

# Get the latest release version
function Get-LatestVersion {
    try {
        $apiUrl = "https://api.github.com/repos/$REPO/releases/latest"
        $response = Invoke-RestMethod -Uri $apiUrl -ErrorAction Stop
        return $response.tag_name
    }
    catch {
        Write-Error "Failed to get latest version: $($_.Exception.Message)"
        exit 1
    }
}

# Download and install binary
function Install-Binary {
    param(
        [string]$Version,
        [string]$Architecture
    )
    
    $platform = "${Architecture}-windows"
    $assetName = "cachekill-${platform}.zip"
    $downloadUrl = "https://github.com/$REPO/releases/download/$Version/$assetName"
    $tempDir = [System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString()
    
    Write-Info "Downloading CacheKill $Version for $platform..."
    Write-Info "URL: $downloadUrl"
    
    try {
        # Create temp directory
        New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
        
        # Download the release
        $zipPath = Join-Path $tempDir $assetName
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -ErrorAction Stop
        
        Write-Info "Extracting archive..."
        
        # Extract the archive
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force
        
        # Find the binary
        $binaryPath = Get-ChildItem -Path $tempDir -Recurse -Name "cachekill.exe" | Select-Object -First 1
        
        if (-not $binaryPath) {
            Write-Error "Binary not found in downloaded archive"
            exit 1
        }
        
        $fullBinaryPath = Join-Path $tempDir $binaryPath
        
        # Create install directory
        New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
        
        # Install the binary
        Write-Info "Installing to $INSTALL_DIR..."
        Copy-Item -Path $fullBinaryPath -Destination "$INSTALL_DIR\$BINARY_NAME.exe" -Force
        
        # Clean up
        Remove-Item -Path $tempDir -Recurse -Force
        
        Write-Success "CacheKill $Version installed successfully!"
    }
    catch {
        Write-Error "Failed to download or install: $($_.Exception.Message)"
        exit 1
    }
}

# Check if binary is already installed
function Test-ExistingInstallation {
    if (Get-Command $BINARY_NAME -ErrorAction SilentlyContinue) {
        try {
            $currentVersion = & $BINARY_NAME --version 2>$null | Select-String -Pattern '\d+\.\d+\.\d+' | ForEach-Object { $_.Matches[0].Value }
            Write-Warning "CacheKill is already installed (version: $currentVersion)"
            
            if (-not $Force) {
                $response = Read-Host "Do you want to update to the latest version? [y/N]"
                if ($response -notmatch '^[Yy]$') {
                    Write-Info "Installation cancelled."
                    exit 0
                }
            }
        }
        catch {
            Write-Warning "CacheKill is already installed (version unknown)"
        }
    }
}

# Add to PATH if needed
function Set-Path {
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    
    if ($currentPath -notlike "*$INSTALL_DIR*") {
        Write-Warning "$INSTALL_DIR is not in your PATH"
        Write-Info "Adding $INSTALL_DIR to PATH..."
        
        $newPath = if ($currentPath) { "$currentPath;$INSTALL_DIR" } else { $INSTALL_DIR }
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        
        # Update current session PATH
        $env:PATH = "$env:PATH;$INSTALL_DIR"
        
        Write-Success "PATH updated. Please restart your PowerShell session for changes to take effect."
    }
    else {
        Write-Success "PATH is already configured correctly"
    }
}

# Main installation function
function Main {
    Write-Info "CacheKill Installer"
    Write-Info "=================="
    
    # Detect architecture
    $architecture = Get-Architecture
    Write-Info "Detected architecture: $architecture"
    
    # Check existing installation
    Test-ExistingInstallation
    
    # Get latest version
    Write-Info "Fetching latest version..."
    $version = Get-LatestVersion
    Write-Info "Latest version: $version"
    
    # Install binary
    Install-Binary $version $architecture
    
    # Setup PATH
    Set-Path
    
    # Verify installation
    if (Get-Command $BINARY_NAME -ErrorAction SilentlyContinue) {
        Write-Success "Installation completed successfully!"
        Write-Info "Run '$BINARY_NAME --help' to get started"
    }
    else {
        Write-Warning "Installation completed, but $BINARY_NAME is not in PATH"
        Write-Info "Please restart your PowerShell session"
    }
}

# Run main function
Main
