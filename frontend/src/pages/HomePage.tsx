import React, { useState, useEffect, useCallback } from 'react'; // Added useCallback
import { Link as RouterLink } from 'react-router-dom';
import CreateVpsModal from '../components/CreateVpsModal';
import type { Vps, LatestPerformanceMetric, VpsListItemResponse } from '../types'; // Added VpsListItemResponse
import { getVpsList } from '../services/vpsService';
import { getLatestVpsMetrics } from '../services/metricsService'; // Added

const HomePage: React.FC = () => {
  const [isCreateVpsModalOpen, setIsCreateVpsModalOpen] = useState(false);
  // VpsList now holds VpsListItemResponse which includes optional latestMetrics
  const [vpsList, setVpsList] = useState<VpsListItemResponse[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Stable function to fetch the initial list of VPS and their first metrics
  const fetchVpsList = useCallback(async (fetchInitialMetrics = true) => {
    setIsLoading(true);
    setError(null);
    try {
      const initialVpsData = await getVpsList();
      if (fetchInitialMetrics) {
        const vpsWithInitialMetrics = await Promise.all(
          initialVpsData.map(async (vps) => {
            try {
              const metrics: LatestPerformanceMetric | null = await getLatestVpsMetrics(vps.id);
              // Ensure the vps object from getVpsList() is compatible with VpsListItemResponse structure if needed,
              // or adjust mapping here. Assuming getVpsList now returns VpsListItemResponse[].
              return { ...vps, latestMetrics: metrics };
            } catch (metricError) {
              console.error(`Failed to fetch initial metrics for VPS ${vps.id}`, metricError);
              return { ...vps, latestMetrics: null };
            }
          })
        );
        setVpsList(vpsWithInitialMetrics as VpsListItemResponse[]); // Cast if initialVpsData was Vps[]
      } else {
        // If not fetching metrics, just set the list, preserving any existing metrics
        // Ensure structure matches VpsListItemResponse
        setVpsList(initialVpsData.map(vps => ({ ...vps, latestMetrics: vps.latestMetrics || null })));
      }
    } catch (err) {
      setError('无法获取VPS列表。');
      console.error(err);
      setVpsList([]);
    } finally {
      setIsLoading(false);
    }
  }, []); // Empty dependency array makes fetchVpsList stable

  // Stable function to update metrics for a single VPS
  const updateSingleVpsMetrics = useCallback(async (vpsId: number) => {
    try {
      const metrics = await getLatestVpsMetrics(vpsId);
      setVpsList(prevList =>
        prevList.map(vps =>
          vps.id === vpsId ? { ...vps, latestMetrics: metrics } : vps
        )
      );
    } catch (error) {
      console.error(`Error updating metrics for VPS ${vpsId}:`, error);
      setVpsList(prevList =>
        prevList.map(vps =>
          vps.id === vpsId ? { ...vps, latestMetrics: null } : vps // Set to null on error
        )
      );
    }
  }, []); // Empty dependency array makes updateSingleVpsMetrics stable

  // useRef to hold the callback that updates all VPS metrics.
  // This allows the setInterval effect to not depend on vpsList directly.
  const metricsUpdateCallbackRef = React.useRef<(() => Promise<void>) | undefined>(undefined);

  useEffect(() => {
    // This function is (re)created whenever vpsList or updateSingleVpsMetrics changes.
    // updateSingleVpsMetrics is stable. So this mainly reacts to vpsList changes.
    metricsUpdateCallbackRef.current = async () => {
      // Access the latest vpsList directly from state here
      // Use a functional update for setVpsList if reading and writing to it in the same callback
      // to ensure we're working with the most up-to-date state.
      // However, here we are reading vpsList to get IDs and then calling updateSingleVpsMetrics
      // which itself calls setVpsList. This should be fine.
      const currentVpsIds = vpsList.map(vps => vps.id);
      if (currentVpsIds.length === 0) return;
      // console.log('RefCallback: Updating metrics for VPS IDs:', currentVpsIds);
      await Promise.all(currentVpsIds.map(id => updateSingleVpsMetrics(id)));
    };
  }, [vpsList, updateSingleVpsMetrics]); // Runs when vpsList or updateSingleVpsMetrics changes

  // Effect to fetch the initial list ONCE on mount
  useEffect(() => {
    fetchVpsList(true);
  }, [fetchVpsList]); // fetchVpsList is stable, so this runs once.

  // Effect to set up and clear the interval for periodic metrics updates
  useEffect(() => {
    const tick = () => {
      if (metricsUpdateCallbackRef.current) {
        // console.log('Interval tick: calling metricsUpdateCallbackRef.current');
        metricsUpdateCallbackRef.current();
      }
    };
    // console.log('Setting up metrics update interval.');
    const intervalId = setInterval(tick, 5000); // Refresh every 5 seconds
    return () => {
      // console.log('Clearing metrics update interval on unmount or re-run.');
      clearInterval(intervalId);
    };
  }, []); // Empty dependency array: interval is set up once and cleaned up on unmount.

  const handleOpenCreateVpsModal = () => {
    setIsCreateVpsModalOpen(true);
  };

  const handleCloseCreateVpsModal = () => {
    setIsCreateVpsModalOpen(false);
  };

  const handleVpsCreated = (newVps: Vps) => {
    console.log('VPS Created:', newVps); // newVps is of type Vps
    // fetchVpsList will fetch VpsListItemResponse[], which is fine.
    fetchVpsList(); // Refresh the list
    handleCloseCreateVpsModal(); // Close modal after creation
  };

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
      {isLoading && <p>加载中...</p>}
      {error && <p style={{ color: 'red' }}>{error}</p>}
      {!isLoading && !error && vpsList.length === 0 && (
        <p>您还没有任何VPS。点击上面的按钮创建一个吧！</p>
      )}
      {!isLoading && !error && vpsList.length > 0 && (
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
            {vpsList.map((vps) => { // vps is VpsListItemResponse
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