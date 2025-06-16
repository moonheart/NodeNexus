import React, { useState, useEffect, useMemo, useRef } from 'react';
import EditVpsModal from '../components/EditVpsModal';
import CreateVpsModal from '../components/CreateVpsModal';
import type { Vps, VpsListItemResponse, ServerStatus as ServerStatusType, ViewMode, Tag } from '../types';
import { useServerListStore, type ServerListState, type ConnectionStatus } from '../store/serverListStore';
import { useAuthStore } from '../store/authStore';
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
  XMarkIcon,
  CheckIcon,
} from '../components/Icons';
import { STATUS_ONLINE, STATUS_OFFLINE, STATUS_ERROR, STATUS_REBOOTING, STATUS_PROVISIONING, STATUS_UNKNOWN } from '../types';
import VpsCard from '../components/VpsCard';
import VpsTableRow from '../components/VpsTableRow';
import * as tagService from '../services/tagService';

interface HomePageStateSlice {
  servers: VpsListItemResponse[];
  isLoading: boolean;
  error: string | null;
  connectionStatus: ConnectionStatus;
  viewMode: ViewMode;
  setViewMode: (mode: ViewMode) => void;
}

const selectHomePageData = (state: ServerListState): HomePageStateSlice => ({
  servers: state.servers,
  isLoading: state.isLoading,
  error: state.error,
  connectionStatus: state.connectionStatus,
  viewMode: state.viewMode,
  setViewMode: state.setViewMode,
});

const statusColorMap: Record<ServerStatusType, string> = {
  [STATUS_ONLINE]: 'text-green-500',
  [STATUS_OFFLINE]: 'text-red-500',
  [STATUS_REBOOTING]: 'text-yellow-500',
  [STATUS_PROVISIONING]: 'text-blue-500',
  [STATUS_ERROR]: 'text-red-700',
  [STATUS_UNKNOWN]: 'text-slate-500',
};

const getContrastingTextColor = (hexColor: string): string => {
    if (!hexColor) return '#000000';
    const hex = hexColor.replace('#', '');
    if (hex.length !== 6) return '#000000';
    const r = parseInt(hex.substring(0, 2), 16);
    const g = parseInt(hex.substring(2, 4), 16);
    const b = parseInt(hex.substring(4, 6), 16);
    const yiq = ((r * 299) + (g * 587) + (b * 114)) / 1000;
    return (yiq >= 128) ? '#000000' : '#ffffff';
};


const HomePage: React.FC = () => {
  const { isAuthenticated } = useAuthStore();
  // WebSocket connection management is now fully handled by the serverListStore,
  // which listens to authStore changes. This component is now only responsible for
  // displaying the state from the store.

  const [isCreateVpsModalOpen, setIsCreateVpsModalOpen] = useState(false);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [editingVps, setEditingVps] = useState<VpsListItemResponse | null>(null);
  const [selectedStatusFilter, setSelectedStatusFilter] = useState<ServerStatusType | null>(null);
  const [selectedGroup, setSelectedGroup] = useState<string>('ALL');
  const [availableTags, setAvailableTags] = useState<Tag[]>([]);
  const [selectedTagIds, setSelectedTagIds] = useState<Set<number>>(new Set());
  const [isTagDropdownOpen, setIsTagDropdownOpen] = useState(false);
  const tagDropdownRef = useRef<HTMLDivElement>(null);

  const {
    servers: vpsList,
    isLoading: isLoadingServers,
    error: wsError,
    connectionStatus,
    viewMode,
    setViewMode,
  } = useServerListStore(useShallow(selectHomePageData));

  // Effect for managing WebSocket connections based on authentication state
  // The useEffect for managing WebSocket connection has been removed.
  // The logic is now centralized in `serverListStore.ts` and triggered
  // by the `init()` call in `App.tsx`.


  useEffect(() => {
    const fetchTags = async () => {
      if (isAuthenticated) {
        try {
          const tags = await tagService.getTags();
          setAvailableTags(tags);
        } catch (error) {
          console.error("获取标签失败:", error);
        }
      } else {
        // For public view, tags are derived from the server data itself within the filter logic
        // We can clear local state to be safe
        setAvailableTags([]);
      }
    };
    fetchTags();
  }, [isAuthenticated, vpsList]); // Depend on vpsList for public view updates

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (tagDropdownRef.current && !tagDropdownRef.current.contains(event.target as Node)) {
        setIsTagDropdownOpen(false);
      }
    };
    if (isTagDropdownOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isTagDropdownOpen]);

  // --- Filtering Logic ---
  const groupFilteredServers = useMemo(() => {
    if (selectedGroup === 'ALL') return vpsList;
    return vpsList.filter(s => s.group === selectedGroup);
  }, [vpsList, selectedGroup]);

  const statusFilteredServers = useMemo(() => {
    if (!selectedStatusFilter) return groupFilteredServers;
    return groupFilteredServers.filter(s => s.status === selectedStatusFilter);
  }, [groupFilteredServers, selectedStatusFilter]);

  const tagFilteredServers = useMemo(() => {
    if (selectedTagIds.size === 0) return statusFilteredServers;
    return statusFilteredServers.filter(server =>
      server.tags?.some(tag => selectedTagIds.has(tag.id))
    );
  }, [statusFilteredServers, selectedTagIds]);

  const displayedServers = tagFilteredServers;

  // --- Modal Handlers ---
  const handleOpenCreateVpsModal = () => setIsCreateVpsModalOpen(true);
  const handleCloseCreateVpsModal = () => setIsCreateVpsModalOpen(false);
  const handleVpsCreated = (newVps: Vps) => {
    console.log('VPS 已创建:', newVps);
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
    console.log('VPS 已更新，状态将通过 WebSocket 刷新。');
    handleCloseEditModal();
  };

  // --- Bulk Edit Logic ---
  // This logic is now encapsulated within the BulkEditTagsModal component.


  // --- Derived Data for Display ---
  const serverStats = useMemo(() => {
    const stats = { total: vpsList.length, [STATUS_ONLINE]: 0, [STATUS_OFFLINE]: 0, [STATUS_REBOOTING]: 0, [STATUS_PROVISIONING]: 0, [STATUS_ERROR]: 0, [STATUS_UNKNOWN]: 0 };
    vpsList.forEach(server => { stats[server.status as ServerStatusType] = (stats[server.status as ServerStatusType] || 0) + 1; });
    return stats;
  }, [vpsList]);

  const uniqueGroups = useMemo(() => {
    const groups = new Set(vpsList.map(s => s.group).filter((g): g is string => !!g));
    return ['ALL', ...Array.from(groups).sort()];
  }, [vpsList]);

  const uniqueTagsForPublicView = useMemo(() => {
    if (isAuthenticated) return [];
    const allTags = vpsList.flatMap(s => s.tags || []);
    const uniqueTags = Array.from(new Map(allTags.map(tag => [tag.id, tag])).values());
    return uniqueTags.filter(t => t.isVisible);
  }, [isAuthenticated, vpsList]);

  const currentAvailableTags = isAuthenticated ? availableTags : uniqueTagsForPublicView;

  const onlineServersForNetworkTotals = useMemo(() => displayedServers.filter(s => s.status === STATUS_ONLINE && s.latestMetrics), [displayedServers]);
  const totalNetworkUp = useMemo(() => onlineServersForNetworkTotals.reduce((acc, server) => acc + (server.latestMetrics?.networkTxInstantBps || 0), 0), [onlineServersForNetworkTotals]);
  const totalNetworkDown = useMemo(() => onlineServersForNetworkTotals.reduce((acc, server) => acc + (server.latestMetrics?.networkRxInstantBps || 0), 0), [onlineServersForNetworkTotals]);
  const formatNetworkSpeedForDisplay = (bps: number): string => {
    if (bps < 1024) return `${bps.toFixed(0)} Bps`;
    if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KBps`;
    if (bps < 1024 * 1024 * 1024) return `${(bps / (1024 * 1024)).toFixed(1)} MBps`;
    return `${(bps / (1024 * 1024 * 1024)).toFixed(1)} GBps`;
  };

  if (isLoadingServers && vpsList.length === 0) {
    return <div className="flex flex-col items-center justify-center h-64"><p className="mt-4 text-slate-600">正在加载服务器...</p></div>;
  }

  let statusMessage = '';
  if (connectionStatus === 'connecting') statusMessage = '正在连接到实时服务器...';
  else if (connectionStatus === 'reconnecting') statusMessage = '连接已断开，正在尝试重新连接...';
  else if (wsError && (connectionStatus === 'error' || connectionStatus === 'permanently_failed')) statusMessage = `无法连接到实时服务器: ${wsError}`;

  return (
    <div className="p-4 md:p-6 lg:p-8 space-y-8 bg-slate-50 min-h-screen">
      {/* Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center">
        <h1 className="text-3xl font-bold text-slate-800">{isAuthenticated ? "VPS 管理面板" : "服务器状态"}</h1>
        {isAuthenticated && (
          <button onClick={handleOpenCreateVpsModal} className="mt-3 sm:mt-0 btn btn-primary">创建新的VPS</button>
        )}
      </div>
      {isAuthenticated && (
        <>
          <CreateVpsModal isOpen={isCreateVpsModalOpen} onClose={handleCloseCreateVpsModal} onVpsCreated={handleVpsCreated} />
          <EditVpsModal isOpen={isEditModalOpen} onClose={handleCloseEditModal} vps={editingVps} allVps={vpsList} onVpsUpdated={handleVpsUpdated} />
        </>
      )}

      {/* Connection Status */}
      {statusMessage && <div className={`p-3 rounded-md text-sm text-center ${connectionStatus === 'error' || connectionStatus === 'permanently_failed' ? 'bg-red-100 text-red-700' : 'bg-yellow-100 text-yellow-700'}`}>{statusMessage}</div>}

      {/* Overview Stats */}
      <section>
        <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-6">
            <h2 className="text-2xl font-semibold text-slate-700">概览</h2>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 md:gap-6">
          <StatCard title="总服务器数" value={serverStats.total} icon={<ServerIcon className="w-6 h-6" />} colorClass="text-indigo-600" onClick={() => setSelectedStatusFilter(null)} isActive={selectedStatusFilter === null} />
          <StatCard title="在线" value={serverStats[STATUS_ONLINE]} icon={<CheckCircleIcon className="w-6 h-6" />} colorClass={statusColorMap[STATUS_ONLINE]} onClick={() => setSelectedStatusFilter(STATUS_ONLINE)} isActive={selectedStatusFilter === STATUS_ONLINE} />
          <StatCard title="离线" value={serverStats[STATUS_OFFLINE]} icon={<XCircleIcon className="w-6 h-6" />} colorClass={statusColorMap[STATUS_OFFLINE]} onClick={() => setSelectedStatusFilter(STATUS_OFFLINE)} isActive={selectedStatusFilter === STATUS_OFFLINE} />
          {serverStats[STATUS_REBOOTING] > 0 && <StatCard title="重启中" value={serverStats[STATUS_REBOOTING]} icon={<ExclamationTriangleIcon className="w-6 h-6" />} colorClass={statusColorMap[STATUS_REBOOTING]} onClick={() => setSelectedStatusFilter(STATUS_REBOOTING)} isActive={selectedStatusFilter === STATUS_REBOOTING} />}
          {serverStats[STATUS_ERROR] > 0 && <StatCard title="错误" value={serverStats[STATUS_ERROR]} icon={<ExclamationTriangleIcon className="w-6 h-6" />} colorClass={statusColorMap[STATUS_ERROR]} onClick={() => setSelectedStatusFilter(STATUS_ERROR)} isActive={selectedStatusFilter === STATUS_ERROR} />}
          <StatCard title="总上传" value={formatNetworkSpeedForDisplay(totalNetworkUp)} icon={<ArrowUpIcon className="w-6 h-6" />} colorClass="text-emerald-500" description="在线服务器" />
          <StatCard title="总下载" value={formatNetworkSpeedForDisplay(totalNetworkDown)} icon={<ArrowDownIcon className="w-6 h-6" />} colorClass="text-sky-500" description="在线服务器" />
        </div>
      </section>

      {/* Server Fleet */}
      <section>
        <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-4">
          <h2 className="text-2xl font-semibold text-slate-700">服务器列表</h2>
          <div className="flex items-center space-x-1 mt-3 sm:mt-0 p-1 bg-slate-200 rounded-lg">
              <button onClick={() => setViewMode('card')} aria-pressed={viewMode === 'card'} className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${viewMode === 'card' ? 'bg-white text-indigo-600 shadow' : 'text-slate-600 hover:bg-slate-300'}`}><Squares2X2Icon className="w-5 h-5 inline mr-1.5" /> 卡片视图</button>
              <button onClick={() => setViewMode('list')} aria-pressed={viewMode === 'list'} className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${viewMode === 'list' ? 'bg-white text-indigo-600 shadow' : 'text-slate-600 hover:bg-slate-300'}`}><ListBulletIcon className="w-5 h-5 inline mr-1.5" /> 列表视图</button>
          </div>
        </div>
        <div className="p-4 bg-white rounded-lg shadow-sm mb-6">
            <div className="flex flex-wrap gap-4 items-center justify-between">
                <div className="flex flex-wrap gap-4 items-center">
                    <div className="flex flex-wrap gap-2 items-center">
                        <span className="text-sm font-medium text-slate-600">按分组筛选:</span>
                        {uniqueGroups.map(group => (
                            <button key={group} onClick={() => setSelectedGroup(group)} aria-pressed={selectedGroup === group} className={`px-4 py-1.5 text-xs sm:text-sm font-medium rounded-full transition-all duration-200 ease-in-out ${selectedGroup === group ? 'bg-indigo-600 text-white shadow-md scale-105' : 'bg-slate-200 text-slate-700 hover:bg-slate-300'}`}>{group === 'ALL' ? '全部' : group}</button>
                        ))}
                    </div>
                    <div className="w-full md:w-auto border-t md:border-t-0 md:border-l border-slate-200 my-2 md:my-0 md:mx-4 h-auto md:h-8"></div>
                    {currentAvailableTags.length > 0 && (
                        <div className="flex flex-wrap gap-2 items-center">
                            <span className="text-sm font-medium text-slate-600">按标签筛选:</span>
                            {/* Display selected tags */}
                            {Array.from(selectedTagIds).map(tagId => {
                                const tag = currentAvailableTags.find(t => t.id === tagId);
                                if (!tag) return null;
                                return (
                                    <span key={tag.id} className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium" style={{ backgroundColor: tag.color, color: getContrastingTextColor(tag.color) }}>
                                        {tag.name}
                                        <button
                                            onClick={() => {
                                                const newSet = new Set(selectedTagIds);
                                                newSet.delete(tag.id);
                                                setSelectedTagIds(newSet);
                                            }}
                                            className="flex-shrink-0 ml-1.5 h-4 w-4 rounded-full inline-flex items-center justify-center hover:bg-black/20 focus:outline-none"
                                            style={{ color: getContrastingTextColor(tag.color) }}
                                        >
                                            <span className="sr-only">Remove {tag.name}</span>
                                            <XMarkIcon className="h-2 w-2" />
                                        </button>
                                    </span>
                                );
                            })}

                            {/* Dropdown for adding new tags */}
                            <div className="relative" ref={tagDropdownRef}>
                                <button
                                    onClick={() => setIsTagDropdownOpen(!isTagDropdownOpen)}
                                    className="px-3 py-1 text-xs font-medium rounded-full transition-all duration-200 ease-in-out bg-slate-200 text-slate-700 hover:bg-slate-300"
                                >
                                    + 添加筛选
                                </button>
                                {isTagDropdownOpen && (
                                    <div className="origin-top-left absolute left-0 mt-2 w-56 rounded-md shadow-lg bg-white ring-1 ring-black ring-opacity-5 z-10">
                                        <div className="py-1" role="menu" aria-orientation="vertical" aria-labelledby="options-menu">
                                            {currentAvailableTags
                                                .map(tag => (
                                                    <a
                                                        key={tag.id}
                                                        href="#"
                                                        onClick={(e) => {
                                                            e.preventDefault();
                                                            const newSet = new Set(selectedTagIds);
                                                            if (newSet.has(tag.id)) {
                                                                newSet.delete(tag.id);
                                                            } else {
                                                                newSet.add(tag.id);
                                                            }
                                                            setSelectedTagIds(newSet);
                                                        }}
                                                        className={`flex items-center justify-between px-4 py-2 text-sm ${selectedTagIds.has(tag.id) ? 'font-semibold text-indigo-600' : 'text-gray-700'} hover:bg-gray-100`}
                                                        role="menuitem"
                                                    >
                                                        <div className="flex items-center">
                                                            <span className="inline-block w-3 h-3 mr-3 rounded-full" style={{ backgroundColor: tag.color }}></span>
                                                            {tag.name}
                                                        </div>
                                                        {selectedTagIds.has(tag.id) && (
                                                            <CheckIcon className="w-5 h-5 text-indigo-600" />
                                                        )}
                                                    </a>
                                                ))
                                            }
                                        </div>
                                    </div>
                                )}
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>

        {displayedServers.length === 0 && !isLoadingServers ? (
          <p className="text-slate-500 text-center py-8 bg-white rounded-lg shadow">没有找到符合当前筛选条件的服务器。</p>
        ) : viewMode === 'card' ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
            {displayedServers.map(server => <VpsCard key={server.id} server={server} onEdit={isAuthenticated ? handleOpenEditModal : undefined} />)}
          </div>
        ) : (
          <div className="bg-white rounded-xl shadow-lg overflow-x-auto">
            <table className="w-full min-w-[1000px]">
              <thead className="bg-slate-100">
                <tr>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">
                      <span>名称</span>
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">状态</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">IP 地址</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">操作系统</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">CPU</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">内存</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">流量使用</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">续费状态</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">上传</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">下载</th>
                  {isAuthenticated && <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">操作</th>}
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-200">
                {displayedServers.map(server => <VpsTableRow key={server.id} server={server} onEdit={isAuthenticated ? handleOpenEditModal : undefined} showActions={isAuthenticated} />)}
              </tbody>
            </table>
          </div>
        )}
      </section>

    </div>
  );
};

export default HomePage;