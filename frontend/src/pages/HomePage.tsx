import React, { useState } from 'react'; // Removed useEffect and useCallback
import { Link as RouterLink } from 'react-router-dom';
import CreateVpsModal from '../components/CreateVpsModal';
import type { Vps, VpsListItemResponse } from '../types'; // Added VpsListItemResponse back
// import { getVpsList } from '../services/vpsService'; // No longer needed for initial list
// import { getLatestVpsMetrics } from '../services/metricsService'; // No longer needed
import { useServerListStore, type ServerListState, type ConnectionStatus } from '../store/serverListStore'; // Import ServerListState and ConnectionStatus
import { useShallow } from 'zustand/react/shallow'; // Import useShallow

interface HomePageStateSlice {
  servers: VpsListItemResponse[];
  isLoading: boolean;
  error: string | null;
  connectionStatus: ConnectionStatus;
}

// Define the selector function outside the component for stability
const selectHomePageData = (state: ServerListState): HomePageStateSlice => ({
  servers: state.servers,
  isLoading: state.isLoading,
  error: state.error,
  connectionStatus: state.connectionStatus,
});

const HomePage: React.FC = () => {
  const [isCreateVpsModalOpen, setIsCreateVpsModalOpen] = useState(false);
  
  const {
    servers: vpsList,
    isLoading,
    error: wsError,
    connectionStatus
  } = useServerListStore(useShallow(selectHomePageData)); // Correctly use useShallow to wrap the selector

  // The useEffect for initializing WebSocket is now in App.tsx
  // Old data fetching and interval logic is removed.

  const handleOpenCreateVpsModal = () => {
    setIsCreateVpsModalOpen(true);
  };

  const handleCloseCreateVpsModal = () => {
    setIsCreateVpsModalOpen(false);
  };

  const handleVpsCreated = (newVps: Vps) => {
    console.log('VPS Created:', newVps);
    // The list will be updated via WebSocket push. No explicit fetch needed here.
    // If an immediate fetch is desired for some reason (e.g. backend doesn't push instantly after creation),
    // a separate action in the store could be triggered, or rely on the next WebSocket push.
    handleCloseCreateVpsModal();
  };

  // Display connection status messages
  let statusMessage = '';
  if (connectionStatus === 'connecting') {
    statusMessage = '正在连接到实时服务器...';
  } else if (connectionStatus === 'reconnecting') {
    statusMessage = '连接已断开，正在尝试重新连接...';
  } else if (connectionStatus === 'error' || connectionStatus === 'permanently_failed') {
    statusMessage = `无法连接到实时服务器: ${wsError || '未知错误'}`;
  }


  return (
    <div style={{ padding: '20px' }}>
      <h1>欢迎来到首页!</h1>
      <p>您已成功登录。</p>
      <button onClick={handleOpenCreateVpsModal} style={{ padding: '10px 15px', margin: '20px 0', cursor: 'pointer', backgroundColor: '#007bff', color: 'white', border: 'none', borderRadius: '4px' }}>
        创建新的VPS
      </button>
      <CreateVpsModal
        isOpen={isCreateVpsModalOpen}
        onClose={handleCloseCreateVpsModal}
        onVpsCreated={handleVpsCreated}
      />

      <h2>您的VPS列表</h2>
      {statusMessage && <p style={{ color: connectionStatus === 'error' || connectionStatus === 'permanently_failed' ? 'red' : 'orange', textAlign: 'center', marginBottom: '10px' }}>{statusMessage}</p>}
      
      <h2>您的VPS列表</h2>
      {isLoading && connectionStatus !== 'connected' && <p>加载中...</p>} {/* Show loading only if not yet connected or initial data not received */}
      {wsError && connectionStatus !== 'reconnecting' && <p style={{ color: 'red' }}>{wsError}</p>} {/* Show error if not in reconnecting state */}
      
      {!isLoading && vpsList.length === 0 && connectionStatus === 'connected' && (
        <p>您还没有任何VPS。点击上面的按钮创建一个吧！</p>
      )}
      {vpsList.length > 0 && (
        <table style={{ width: '100%', borderCollapse: 'collapse', marginTop: '20px' }}>
          <thead>
            <tr style={{ backgroundColor: '#f0f0f0' }}>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>ID</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>名称</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>IP地址</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>状态</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>CPU</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>内存</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>下行速率</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>上行速率</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>创建时间</th>
            </tr>
          </thead>
          <tbody>
            {vpsList.map((vps: VpsListItemResponse) => { // Explicitly type vps
              const metrics = vps.latestMetrics; // Use camelCase
              const cpuUsage = metrics ? `${metrics.cpuUsagePercent.toFixed(1)}%` : 'N/A';
              const memUsage = metrics && metrics.memoryTotalBytes > 0
                ? `${(metrics.memoryUsageBytes / (1024 * 1024)).toFixed(1)}MB / ${(metrics.memoryTotalBytes / (1024 * 1024)).toFixed(1)}MB`
                : 'N/A';
              const formatSpeed = (bps: number | undefined | null) => { // Allow null
                if (typeof bps !== 'number' || bps === null) return 'N/A';
                if (bps < 1024) return `${bps.toFixed(0)} B/s`;
                if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
                return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
              };
              const downSpeed = formatSpeed(metrics?.networkRxInstantBps);
              const upSpeed = formatSpeed(metrics?.networkTxInstantBps);

              return (
                <tr key={vps.id}>
                  <td style={{ border: '1px solid #ddd', padding: '8px' }}>
                    <RouterLink to={`/vps/${vps.id}`}>{vps.id}</RouterLink>
                  </td>
                  <td style={{ border: '1px solid #ddd', padding: '8px' }}>
                    <RouterLink to={`/vps/${vps.id}`}>{vps.name}</RouterLink>
                  </td>
                  <td style={{ border: '1px solid #ddd', padding: '8px' }}>{vps.ipAddress || 'N/A'}</td>
                  <td style={{ border: '1px solid #ddd', padding: '8px' }}>{vps.status}</td>
                  <td style={{ border: '1px solid #ddd', padding: '8px' }}>{cpuUsage}</td>
                  <td style={{ border: '1px solid #ddd', padding: '8px' }}>{memUsage}</td>
                  <td style={{ border: '1px solid #ddd', padding: '8px' }}>{downSpeed}</td>
                  <td style={{ border: '1px solid #ddd', padding: '8px' }}>{upSpeed}</td>
                  <td style={{ border: '1px solid #ddd', padding: '8px' }}>{new Date(vps.createdAt).toLocaleString()}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
};

export default HomePage;