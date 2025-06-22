import type { Vps, VpsListItemResponse } from '../types';

const GITHUB_RAW_BASE_URL = 'https://github.com/moonheart/NodeNexus/raw/refs/heads/master';

const SCRIPT_URLS = {
  linux: `${GITHUB_RAW_BASE_URL}/scripts/agent.sh`,
  macos: `${GITHUB_RAW_BASE_URL}/scripts/agent-macos.sh`,
  windows: `${GITHUB_RAW_BASE_URL}/scripts/agent-windows.ps1`,
};

type OsType = 'linux' | 'macos' | 'windows';

/**
 * Generates the installation command for a given VPS and OS type.
 * @param vps - The VPS object (must contain id and agent_secret).
 * @param osType - The target operating system.
 * @returns The installation command string.
 */
export const generateInstallCommand = (vps: Vps | VpsListItemResponse, osType: OsType): string => {
  // Use window.location to build the base server address.
  const serverAddress = `${window.location.protocol}//${window.location.host}`;
  
  const scriptUrl = SCRIPT_URLS[osType];
  const { id } = vps;
  const agent_secret = 'agent_secret' in vps ? vps.agent_secret : vps.agentSecret;

  switch (osType) {
    case 'linux':
      return `curl -sSL ${scriptUrl} | sudo bash -s -- --server-address ${serverAddress} --vps-id ${id} --agent-secret ${agent_secret}`;
    case 'macos':
      // Assuming macOS command is similar to Linux
      return `curl -sSL ${scriptUrl} | bash -s -- --server-address ${serverAddress} --vps-id ${id} --agent-secret ${agent_secret}`;
    case 'windows':
      // Using PowerShell to download and execute the script
      return `powershell -Command "Invoke-WebRequest -Uri ${scriptUrl} -OutFile .\\agent-windows.ps1; .\\agent-windows.ps1 -Command install -ServerAddress ${serverAddress} -VpsId ${id} -AgentSecret ${agent_secret}"`;
    default:
      // This case should not be reached with the given OsType union
      return 'Unsupported OS type specified.';
  }
};

/**
 * Determines the OS type from the vps.osType string.
 * @param osTypeString - The osType string from the VPS object.
 * @returns The detected OsType or 'linux' as a fallback.
 */
export const detectOsType = (osTypeString: string | null | undefined): OsType => {
  const lowerOsType = osTypeString?.toLowerCase() || '';
  if (lowerOsType.includes('windows')) {
    return 'windows';
  }
  if (lowerOsType.includes('darwin') || lowerOsType.includes('macos')) {
    return 'macos';
  }
  // Default to Linux for any other case (including 'linux', 'ubuntu', 'centos', etc.)
  return 'linux';
};