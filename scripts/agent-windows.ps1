<#
.SYNOPSIS
    Installs, updates, or uninstalls the NodeNexus Agent on Windows.
.DESCRIPTION
    This script manages the NodeNexus Agent lifecycle. It can download the latest version
    from GitHub, install it as a Windows service, configure it with the necessary
    server details, and also clean up the installation.
.PARAMETER Command
    Specifies the action to perform. Can be 'install' or 'uninstall'.
    'install' is the default action.
.PARAMETER ServerAddress
    The HTTP/HTTPS URL of the NodeNexus server (e.g., "http://server.example.com:8080").
    This parameter is required for the initial installation.
.PARAMETER VpsId
    The ID of the VPS, used for identification with the server. Required for installation.
.PARAMETER AgentSecret
    The secret key for authenticating the agent. Required for installation.
.PARAMETER DownloadUrl
    Optional. A direct URL to the agent binary. If provided, it bypasses the
    GitHub release check.
.EXAMPLE
    .\agent-windows.ps1 -Command install -ServerAddress "http://10.0.0.1:8080" -VpsId 123 -AgentSecret "your-secret-key"
    Installs the latest agent and configures it to connect to the specified server.
.EXAMPLE
    .\agent-windows.ps1 -Command uninstall
    Stops and removes the NodeNexus Agent service and deletes its files.
#>
[CmdletBinding()]
param(
    [ValidateSet('install', 'uninstall')]
    [string]$Command = 'install',

    [string]$ServerAddress,

    [string]$VpsId,

    [string]$AgentSecret,

    [string]$DownloadUrl
)

# --- Configuration ---
$ErrorActionPreference = 'Stop'
$serviceName = "NodeNexusAgent"
$installDir = "C:\NodeNexusAgent"
$githubRepo = "moonheart/NodeNexus"

# --- Helper Functions ---
function Write-Log {
    param([string]$Level, [string]$Message)
    $color = switch ($Level) {
        "INFO" { "Cyan" }
        "SUCCESS" { "Green" }
        "ERROR" { "Red" }
        default { "White" }
    }
    Write-Host "[$Level] $Message" -ForegroundColor $color
}

function Check-Admin {
    Write-Log "INFO" "Checking for administrator privileges..."
    if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Write-Log "ERROR" "This script must be run as an Administrator."
        exit 1
    }
    Write-Log "INFO" "Administrator privileges confirmed."
}

function Get-LatestReleaseInfo {
    param([string]$repo)
    try {
        $url = "https://api.github.com/repos/$repo/releases/latest"
        Write-Log "INFO" "Fetching latest release information from $url..."
        return Invoke-RestMethod -Uri $url -Method Get -UseBasicParsing
    }
    catch {
        Write-Log "ERROR" "Failed to get latest release info from GitHub. Error: $_"
        exit 1
    }
}

function Get-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { return "amd64" }
        "ARM64" { return "arm64" }
        default {
            Write-Log "ERROR" "Unsupported architecture: $arch"
            exit 1
        }
    }
}

# --- Main Logic Functions ---

function Create-ConfigFile {
    param(
        [string]$ConfigPath
    )
    # Prompt for required parameters if not provided
    if (-not $ServerAddress) {
        $ServerAddress = Read-Host -Prompt "Enter the NodeNexus Server URL (e.g., http://192.168.1.100:8080)"
    }
    if (-not ($ServerAddress -match "^https?://")) {
        Write-Log "ERROR" "Invalid URL format. It must start with http:// or https://"
        exit 1
    }
    if (-not $VpsId) {
        $VpsId = Read-Host -Prompt "Enter the VPS ID"
    }
    if (-not $AgentSecret) {
        $AgentSecret = Read-Host -Prompt "Enter the Agent Secret"
    }

    Write-Log "INFO" "Creating configuration file at '$ConfigPath'..."

    $configContent = @"
# Node-Nexus Agent Configuration
server_address = "$ServerAddress"
vps_id = $VpsId
agent_secret = "$AgentSecret"

# Default values, can be adjusted later
log_level = "info"
heartbeat_interval_seconds = 30
metrics_collect_interval_seconds = 5
metrics_upload_interval_seconds = 7
metrics_upload_batch_max_size = 10
data_collection_interval_seconds = 15
generic_metrics_upload_interval_seconds = 300
generic_metrics_upload_batch_max_size = 100

[docker_monitoring]
enabled = true
docker_info_collect_interval_seconds = 600
docker_info_upload_interval_seconds = 900
"@
    Set-Content -Path $ConfigPath -Value $configContent
    Write-Log "SUCCESS" "Configuration file created."
}


function Install-Agent {
    # If service exists, stop and remove it for a clean (re)installation
    if (Get-Service -Name $serviceName -ErrorAction SilentlyContinue) {
        Write-Log "INFO" "Service '$serviceName' is already installed. Re-installing..."
        Uninstall-Agent
    }

    # Setup directory
    Write-Log "INFO" "Setting up installation directory: '$installDir'..."
    if (-not (Test-Path -Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir | Out-Null
    }

    # Create Config File
    $configPath = Join-Path $installDir "config.toml"
    Create-ConfigFile -ConfigPath $configPath

    # Determine download URL
    $actualDownloadUrl = $DownloadUrl
    if (-not $actualDownloadUrl) {
        $releaseInfo = Get-LatestReleaseInfo -repo $githubRepo
        $arch = Get-Architecture
        $assetName = "agent-windows-$arch.exe"
        $asset = $releaseInfo.assets | Where-Object { $_.name -eq $assetName }

        if (-not $asset) {
            Write-Log "ERROR" "Could not find a release asset named '$assetName' for version $($releaseInfo.tag_name)."
            exit 1
        }
        $actualDownloadUrl = $asset.browser_download_url
    }
    
    # Download
    $tempDir = $env:TEMP
    $downloadPath = Join-Path $tempDir "agent.exe"
    Write-Log "INFO" "Downloading agent from '$actualDownloadUrl'..."
    Invoke-WebRequest -Uri $actualDownloadUrl -OutFile $downloadPath -UseBasicParsing
    Write-Log "INFO" "Download complete."

    # Install
    $exePath = Join-Path $installDir "agent.exe"
    Move-Item -Path $downloadPath -Destination $exePath -Force

    # Create Service
    Write-Log "INFO" "Creating Windows service '$serviceName'..."
    $quotedExePath = "`"$exePath`""
    $quotedConfigPath = "`"$configPath`""
    $binPath = "$quotedExePath --config $quotedConfigPath"
    
    sc.exe create $serviceName binPath= $binPath start= auto
    sc.exe failure $serviceName reset= 86400 actions= restart/60000/restart/60000/restart/120000
    
    # Configure Environment Variables for the service (for updater)
    $envPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$serviceName\Environment"
    if (-not (Test-Path -Path $envPath)) {
        New-Item -Path $envPath -Force | Out-Null
    }
    Set-ItemProperty -Path $envPath -Name "NEXUS_AGENT_SERVICE_NAME" -Value $serviceName
    Write-Log "INFO" "Service environment variables configured for auto-update."

    # Start Service
    Write-Log "INFO" "Starting service '$serviceName'..."
    Start-Service -Name $serviceName

    Write-Log "SUCCESS" "Installation complete. The NodeNexus Agent is now running."
}

function Uninstall-Agent {
    Write-Log "INFO" "Starting uninstallation process..."
    
    if (Get-Service -Name $serviceName -ErrorAction SilentlyContinue) {
        Write-Log "INFO" "Stopping service '$serviceName'..."
        Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
        Write-Log "INFO" "Removing service '$serviceName'..."
        sc.exe delete $serviceName
        Start-Sleep -Seconds 5 # Wait for service to be removed
    } else {
        Write-Log "INFO" "Service '$serviceName' not found."
    }

    if (Test-Path -Path $installDir) {
        Write-Log "INFO" "Removing installation directory: $installDir"
        Remove-Item -Recurse -Force -Path $installDir
    }

    Write-Log "SUCCESS" "Uninstallation complete."
}


# --- Script Entry Point ---
Check-Admin

switch ($Command) {
    "install" {
        Install-Agent
    }
    "uninstall" {
        Uninstall-Agent
    }
}