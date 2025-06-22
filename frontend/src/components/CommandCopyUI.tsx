import React, { useState, useEffect } from 'react';
import type { Vps, VpsListItemResponse } from '../types';
import { generateInstallCommand, detectOsType } from '../utils/commandUtils';

type OsType = 'linux' | 'macos' | 'windows';

interface CommandCopyUIProps {
  vps: Vps | VpsListItemResponse;
}

const CommandCopyUI: React.FC<CommandCopyUIProps> = ({ vps }) => {
  const [activeTab, setActiveTab] = useState<OsType>('linux');
  const [copySuccess, setCopySuccess] = useState('');

  useEffect(() => {
    // Detect and set the default OS tab based on vps info
    const detectedOs = detectOsType('osType' in vps ? vps.osType : null);
    setActiveTab(detectedOs);
  }, [vps]);

  const handleCopyToClipboard = (command: string) => {
    navigator.clipboard.writeText(command).then(() => {
      setCopySuccess('已复制!');
      setTimeout(() => setCopySuccess(''), 2000);
    }, (err) => {
      console.error('Failed to copy command:', err);
      // Optionally, set an error message state to display to the user
    });
  };

  const command = generateInstallCommand(vps, activeTab);

  return (
    <div>
      <div style={{ marginBottom: '1rem' }}>
        {(['linux', 'macos', 'windows'] as OsType[]).map(os => (
          <button
            key={os}
            onClick={() => setActiveTab(os)}
            style={{
              padding: '8px 12px',
              marginRight: '8px',
              border: `1px solid ${activeTab === os ? '#007bff' : '#ccc'}`,
              backgroundColor: activeTab === os ? '#007bff' : 'white',
              color: activeTab === os ? 'white' : 'black',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            {os.charAt(0).toUpperCase() + os.slice(1)}
          </button>
        ))}
      </div>

      <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all', backgroundColor: '#f0f0f0', padding: '10px', borderRadius: '4px', border: '1px solid #ddd', maxHeight: '150px', overflowY: 'auto' }}>
        <code>{command}</code>
      </pre>
      <div style={{ marginTop: '15px', display: 'flex', alignItems: 'center' }}>
        <button onClick={() => handleCopyToClipboard(command)} style={{ padding: '8px 12px', cursor: 'pointer', backgroundColor: '#28a745', color: 'white', border: 'none', borderRadius: '4px' }}>
          复制命令
        </button>
        {copySuccess && <span style={{ color: 'green', marginLeft: '15px' }}>{copySuccess}</span>}
      </div>
    </div>
  );
};

export default CommandCopyUI;