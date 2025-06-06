import React, { useState, useMemo } from 'react';
import EditVpsModal from '../components/EditVpsModal';
import CreateVpsModal from '../components/CreateVpsModal';
import type { Vps, VpsListItemResponse, ServerStatus as ServerStatusType } from '../types';
import { useServerListStore, type ServerListState, type ConnectionStatus } from '../store/serverListStore';
import { useShallow } from 'zustand/react/shallow';
import StatCard from '../components/StatCard';
import {
  ServerIcon,
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
  ArrowUpIcon,
  ArrowDownIcon,
  ListBulletIcon,
  Squares2X2Icon,
  // ChevronUpIcon,
  // ChevronDownIcon,
  // ArrowsUpDownIcon // For sorting later
} from '../components/Icons';
import { STATUS_ONLINE, STATUS_OFFLINE, STATUS_ERROR, STATUS_REBOOTING, STATUS_PROVISIONING, STATUS_UNKNOWN } from '../types'; // Import status constants
import VpsCard from '../components/VpsCard'; // Import VpsCard
import VpsTableRow from '../components/VpsTableRow'; // Import VpsTableRow

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

type ViewMode = 'card' | 'list';
// type SortDirection = 'ascending' | 'descending'; // For sorting later
// interface SortConfig {
//   key: string;
//   direction: SortDirection;
// }

// Mapping our ServerStatusType to colors for StatCard
const statusColorMap: Record<ServerStatusType, string> = {
  [STATUS_ONLINE]: 'text-green-500',
  [STATUS_OFFLINE]: 'text-red-500',
  [STATUS_REBOOTING]: 'text-yellow-500',
  [STATUS_PROVISIONING]: 'text-blue-500',
  [STATUS_ERROR]: 'text-red-700',
  [STATUS_UNKNOWN]: 'text-slate-500',
};

const HomePage: React.FC = () => {
  const [isCreateVpsModalOpen, setIsCreateVpsModalOpen] = useState(false);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [editingVps, setEditingVps] = useState<VpsListItemResponse | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('card');
  const [selectedStatusFilter, setSelectedStatusFilter] = useState<ServerStatusType | null>(null);
  const [selectedGroup, setSelectedGroup] = useState<string>('ALL');
  // const [sortConfig, setSortConfig] = useState<SortConfig | null>(null); // For sorting later

  const {
    servers: vpsList, // Renamed from allServers for clarity with existing code
    isLoading: isLoadingServers, // Renamed to avoid conflict with potential local loading states
    error: wsError,
    connectionStatus
  } = useServerListStore(useShallow(selectHomePageData));

  const handleOpenCreateVpsModal = () => setIsCreateVpsModalOpen(true);
  const handleCloseCreateVpsModal = () => setIsCreateVpsModalOpen(false);
  const handleVpsCreated = (newVps: Vps) => {
    console.log('VPS Created:', newVps);
    handleCloseCreateVpsModal();
  };

 const handleOpenEditModal = (server: VpsListItemResponse) => {
   setEditingVps(server);
   setIsEditModalOpen(true);
 };

 const handleCloseEditModal = () => {
   setIsEditModalOpen(false);
   setEditingVps(null);
 };

 const handleVpsUpdated = () => {
   // The websocket connection should update the store automatically.
   // We can add a manual refetch here if needed as a fallback.
   console.log('VPS updated, store should refresh via WebSocket.');
   handleCloseEditModal();
 };

  // Derived data for StatCards and filtering
  const serverStats = useMemo(() => {
    const stats = {
      total: vpsList.length,
      [STATUS_ONLINE]: 0,
      [STATUS_OFFLINE]: 0,
      [STATUS_REBOOTING]: 0,
      [STATUS_PROVISIONING]: 0,
      [STATUS_ERROR]: 0,
      [STATUS_UNKNOWN]: 0,
    };
    vpsList.forEach(server => {
      stats[server.status as ServerStatusType] = (stats[server.status as ServerStatusType] || 0) + 1;
    });
    return stats;
  }, [vpsList]);

  const uniqueGroups = useMemo(() => {
   const groups = new Set(vpsList.map(s => s.group).filter((g): g is string => !!g));
   return ['ALL', ...Array.from(groups).sort()];
  }, [vpsList]);

  const groupFilteredServers = useMemo(() => {
   if (selectedGroup === 'ALL') return vpsList;
   return vpsList.filter(s => s.group === selectedGroup);
  }, [vpsList, selectedGroup]);

  const statusFilteredServers = useMemo(() => {
    if (!selectedStatusFilter) return groupFilteredServers;
    return groupFilteredServers.filter(s => s.status === selectedStatusFilter);
  }, [groupFilteredServers, selectedStatusFilter]);

  // TODO: Implement sortedServers once sorting UI is added
  const displayedServers = statusFilteredServers; // Placeholder for now

  const onlineServersForNetworkTotals = useMemo(() => {
    // Filter by selectedStatusFilter first, then by 'online' if no specific status is selected for network totals,
    // or just use all 'online' servers from the original list if that's preferred.
    // For now, let's base it on the currently displayed servers that are online.
    return displayedServers.filter(s => s.status === STATUS_ONLINE && s.latestMetrics);
  }, [displayedServers]);


  const totalNetworkUp = useMemo(() => {
    return onlineServersForNetworkTotals.reduce((acc, server) => acc + (server.latestMetrics?.networkTxInstantBps || 0), 0);
  }, [onlineServersForNetworkTotals]);

  const totalNetworkDown = useMemo(() => {
    return onlineServersForNetworkTotals.reduce((acc, server) => acc + (server.latestMetrics?.networkRxInstantBps || 0), 0);
  }, [onlineServersForNetworkTotals]);

  const formatNetworkSpeedForDisplay = (bps: number): string => {
    if (bps < 1024) return `${bps.toFixed(0)} Bps`;
    if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KBps`;
    if (bps < 1024 * 1024 * 1024) return `${(bps / (1024 * 1024)).toFixed(1)} MBps`;
    return `${(bps / (1024 * 1024 * 1024)).toFixed(1)} GBps`;
  };


  // Loading and error display
  if (isLoadingServers && vpsList.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-64">
        {/* Placeholder for a spinner icon if available */}
        <p className="mt-4 text-slate-600">Loading servers...</p>
      </div>
    );
  }

  let statusMessage = '';
  if (connectionStatus === 'connecting') statusMessage = '正在连接到实时服务器...';
  else if (connectionStatus === 'reconnecting') statusMessage = '连接已断开，正在尝试重新连接...';
  else if (wsError && (connectionStatus === 'error' || connectionStatus === 'permanently_failed')) {
    statusMessage = `无法连接到实时服务器: ${wsError}`;
  }


  return (
    <div className="p-4 md:p-6 lg:p-8 space-y-8 bg-slate-50 min-h-screen">
      {/* Header and Create VPS Button */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center">
        <h1 className="text-3xl font-bold text-slate-800">VPS Dashboard</h1>
        <button
          onClick={handleOpenCreateVpsModal}
          className="mt-3 sm:mt-0 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold py-2 px-4 rounded-lg shadow-md transition-colors duration-150"
        >
          创建新的VPS
        </button>
      </div>
      <CreateVpsModal
        isOpen={isCreateVpsModalOpen}
        onClose={handleCloseCreateVpsModal}
        onVpsCreated={handleVpsCreated}
      />
     <EditVpsModal
       isOpen={isEditModalOpen}
       onClose={handleCloseEditModal}
       vps={editingVps}
       allVps={vpsList}
       onVpsUpdated={handleVpsUpdated}
     />

      {/* Connection Status Message */}
      {statusMessage && (
        <div className={`p-3 rounded-md text-sm text-center ${
            connectionStatus === 'error' || connectionStatus === 'permanently_failed'
              ? 'bg-red-100 text-red-700'
              : 'bg-yellow-100 text-yellow-700'
          }`}
        >
          {statusMessage}
        </div>
      )}

      {/* Overview Stats Section */}
      <section>
        <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-6">
            <h2 className="text-2xl font-semibold text-slate-700">概览</h2>
            <div className="flex items-center space-x-1 mt-3 sm:mt-0 p-1 bg-slate-200 rounded-lg">
                <button
                    onClick={() => setViewMode('card')}
                    aria-pressed={viewMode === 'card'}
                    className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${viewMode === 'card' ? 'bg-white text-indigo-600 shadow' : 'text-slate-600 hover:bg-slate-300'}`}
                >
                    <Squares2X2Icon className="w-5 h-5 inline mr-1.5" /> 卡片视图
                </button>
                <button
                    onClick={() => setViewMode('list')}
                    aria-pressed={viewMode === 'list'}
                    className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${viewMode === 'list' ? 'bg-white text-indigo-600 shadow' : 'text-slate-600 hover:bg-slate-300'}`}
                >
                    <ListBulletIcon className="w-5 h-5 inline mr-1.5" /> 列表视图
                </button>
            </div>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 md:gap-6">
          <StatCard title="总服务器数" value={serverStats.total} icon={<ServerIcon className="w-6 h-6" />} colorClass="text-indigo-600" onClick={() => setSelectedStatusFilter(null)} isActive={selectedStatusFilter === null} />
          <StatCard title="在线" value={serverStats[STATUS_ONLINE]} icon={<CheckCircleIcon className="w-6 h-6" />} colorClass={statusColorMap[STATUS_ONLINE]} onClick={() => setSelectedStatusFilter(STATUS_ONLINE)} isActive={selectedStatusFilter === STATUS_ONLINE} />
          <StatCard title="离线" value={serverStats[STATUS_OFFLINE]} icon={<XCircleIcon className="w-6 h-6" />} colorClass={statusColorMap[STATUS_OFFLINE]} onClick={() => setSelectedStatusFilter(STATUS_OFFLINE)} isActive={selectedStatusFilter === STATUS_OFFLINE} />
          {/* Optional: Add other statuses if they are common */}
          {serverStats[STATUS_REBOOTING] > 0 && <StatCard title="重启中" value={serverStats[STATUS_REBOOTING]} icon={<ExclamationTriangleIcon className="w-6 h-6" />} colorClass={statusColorMap[STATUS_REBOOTING]} onClick={() => setSelectedStatusFilter(STATUS_REBOOTING)} isActive={selectedStatusFilter === STATUS_REBOOTING} />}
          {serverStats[STATUS_ERROR] > 0 && <StatCard title="错误" value={serverStats[STATUS_ERROR]} icon={<ExclamationTriangleIcon className="w-6 h-6" />} colorClass={statusColorMap[STATUS_ERROR]} onClick={() => setSelectedStatusFilter(STATUS_ERROR)} isActive={selectedStatusFilter === STATUS_ERROR} />}
          
          <StatCard title="总上传" value={formatNetworkSpeedForDisplay(totalNetworkUp)} icon={<ArrowUpIcon className="w-6 h-6" />} colorClass="text-emerald-500" description="在线服务器" />
          <StatCard title="总下载" value={formatNetworkSpeedForDisplay(totalNetworkDown)} icon={<ArrowDownIcon className="w-6 h-6" />} colorClass="text-sky-500" description="在线服务器" />
        </div>
      </section>

      {/* Server Fleet Section - Placeholder for now */}
      <section>
        <div className="flex flex-col md:flex-row justify-between items-start md:items-center mb-6">
          <h2 className="text-2xl font-semibold text-slate-700">服务器列表</h2>
          {vpsList.length > 0 && (
           <div className="mt-3 md:mt-0 flex flex-wrap gap-2">
             {uniqueGroups.map(group => (
               <button
                 key={group}
                 onClick={() => setSelectedGroup(group)}
                 aria-pressed={selectedGroup === group}
                 className={`px-4 py-1.5 text-xs sm:text-sm font-medium rounded-full transition-all duration-200 ease-in-out
                   ${selectedGroup === group
                     ? 'bg-indigo-600 text-white shadow-md scale-105'
                     : 'bg-slate-200 text-slate-700 hover:bg-slate-300 hover:text-slate-900'
                   }`}
               >
                 {group === 'ALL' ? '全部' : group}
               </button>
             ))}
           </div>
          )}
        </div>

        {displayedServers.length === 0 && !isLoadingServers ? (
          <p className="text-slate-500 text-center py-8 bg-white rounded-lg shadow">
            没有找到符合当前筛选条件的服务器 (状态: {selectedStatusFilter || '所有'}).
            {vpsList.length === 0 && " 您还没有任何VPS，尝试创建一个吧！"}
          </p>
        ) : viewMode === 'card' ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
            {displayedServers.map(server => (
              <VpsCard key={server.id} server={server} onEdit={handleOpenEditModal} />
            ))}
          </div>
        ) : (
          // Placeholder for ServerTable
          <div className="bg-white rounded-xl shadow-lg overflow-x-auto">
            <table className="w-full min-w-[1000px]">
              <thead className="bg-slate-100">
                <tr>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">名称</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">状态</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">IP 地址</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">CPU</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">内存</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">上传</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">下载</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">操作</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-200">
                {displayedServers.map(server => (
                  <VpsTableRow key={server.id} server={server} onEdit={handleOpenEditModal} />
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
};

export default HomePage;