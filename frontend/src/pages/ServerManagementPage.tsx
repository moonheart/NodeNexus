import React, { useState, useEffect, useMemo, useRef } from 'react';
import EditVpsModal from '../components/EditVpsModal';
import CreateVpsModal from '../components/CreateVpsModal';
import CopyCommandModal from '../components/CopyCommandModal';
import type { Vps, VpsListItemResponse, Tag } from '../types';
import { useServerListStore, type ServerListState, type ConnectionStatus } from '../store/serverListStore';
import { useShallow } from 'zustand/react/shallow';
import {
  PencilSquareIcon,
  XMarkIcon,
  CheckIcon,
  PlusIcon,
  RefreshCwIcon,
} from '../components/Icons';
import ServerManagementTableRow from '../components/ServerManagementTableRow';
import BulkEditTagsModal from '../components/BulkEditTagsModal';
import * as tagService from '../services/tagService';
import * as vpsService from '../services/vpsService';


interface ServerManagementPageStateSlice {
  servers: VpsListItemResponse[];
  isLoading: boolean;
  error: string | null;
  connectionStatus: ConnectionStatus;
}

const selectServerManagementPageData = (state: ServerListState): ServerManagementPageStateSlice => ({
  servers: state.servers,
  isLoading: state.isLoading,
  error: state.error,
  connectionStatus: state.connectionStatus,
});


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


const ServerManagementPage: React.FC = () => {
  const [isCreateVpsModalOpen, setIsCreateVpsModalOpen] = useState(false);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [editingVps, setEditingVps] = useState<VpsListItemResponse | null>(null);
  const [vpsForCommand, setVpsForCommand] = useState<VpsListItemResponse | null>(null);
  const [isCopyCommandModalOpen, setIsCopyCommandModalOpen] = useState(false);
  const [selectedGroup, setSelectedGroup] = useState<string>('ALL');
  const [availableTags, setAvailableTags] = useState<Tag[]>([]);
  const [selectedTagIds, setSelectedTagIds] = useState<Set<number>>(new Set());
  const [selectedVpsIds, setSelectedVpsIds] = useState<Set<number>>(new Set());
  const [isBulkEditModalOpen, setIsBulkEditModalOpen] = useState(false);
  const [isTagDropdownOpen, setIsTagDropdownOpen] = useState(false);
  const tagDropdownRef = useRef<HTMLDivElement>(null);


  const {
    servers: vpsList,
    isLoading: isLoadingServers,
    error: wsError,
    connectionStatus,
  } = useServerListStore(useShallow(selectServerManagementPageData));

  useEffect(() => {
    const fetchTags = async () => {
      try {
        const tags = await tagService.getTags();
        setAvailableTags(tags);
      } catch (error) {
        console.error("Failed to fetch tags:", error);
      }
    };
    fetchTags();
  }, []);


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

  const tagFilteredServers = useMemo(() => {
    if (selectedTagIds.size === 0) return groupFilteredServers;
    return groupFilteredServers.filter(server =>
      server.tags?.some(tag => selectedTagIds.has(tag.id))
    );
  }, [groupFilteredServers, selectedTagIds]);

  const displayedServers = tagFilteredServers;


  // --- Modal Handlers ---
  const handleOpenCreateVpsModal = () => setIsCreateVpsModalOpen(true);
  const handleCloseCreateVpsModal = () => setIsCreateVpsModalOpen(false);
  const handleVpsCreated = (newVps: Vps) => {
    console.log('VPS creation successful, closing create modal and opening command modal.');
    handleCloseCreateVpsModal();
    // The backend returns a Vps model, but the copy command modal expects a VpsListItemResponse.
    // We can create a temporary VpsListItemResponse-like object for the modal.
    // The full, correct data will arrive shortly via WebSocket.
    const vpsForCommand: VpsListItemResponse = {
      ...newVps,
      userId: newVps.user_id,
      agentSecret: newVps.agent_secret,
      ipAddress: newVps.ip_address,
      osType: newVps.os_type,
      createdAt: newVps.created_at,
      updatedAt: newVps.updated_at,
      agentVersion: null,
      latestMetrics: null,
      configStatus: 'unknown',
      lastConfigUpdateAt: null,
      lastConfigError: null,
      // Renewal info fields
      renewalCycle: null,
      renewalCycleCustomDays: null,
      renewalPrice: null,
      renewalCurrency: null,
      nextRenewalDate: null,
      lastRenewalDate: null,
      serviceStartDate: null,
      paymentMethod: null,
      autoRenewEnabled: null,
      renewalNotes: null,
      reminderActive: null,
    };
    handleOpenCopyCommandModal(vpsForCommand);
  };

  const handleOpenEditModal = (server: VpsListItemResponse) => {
    setEditingVps(server);
    setIsEditModalOpen(true);
  };

  const handleOpenCopyCommandModal = async (server: VpsListItemResponse) => {
    // The server object from the list might not have the agent_secret for security reasons.
    // Fetch the full details to ensure we have it before opening the modal.
    try {
      // Consider showing a loading indicator here
      const fullVpsDetails = await vpsService.getVpsDetail(server.id.toString());
      setVpsForCommand(fullVpsDetails);
      setIsCopyCommandModalOpen(true);
    } catch (error) {
      console.error("Failed to fetch VPS details for command copy:", error);
      // TODO: Replace with a proper toast notification
      alert("无法获取安装命令。请稍后再试。");
    }
  };

  const handleCloseEditModal = () => {
    setIsEditModalOpen(false);
    setEditingVps(null);
  };

  const handleVpsUpdated = () => {
    console.log('VPS updated, store should refresh via WebSocket.');
    handleCloseEditModal();
  };

  const handleCloseCopyCommandModal = () => {
    setIsCopyCommandModalOpen(false);
    setVpsForCommand(null);
  };

  const handleTriggerUpdate = async (vpsIds: number[]) => {
    if (vpsIds.length === 0) return;
    try {
      const result = await vpsService.triggerAgentUpdate(vpsIds);
      // TODO: Replace with a proper toast notification system
      alert(`Update command sent. Success: ${result.successfulCount}, Failed: ${result.failedCount}`);
    } catch (error) {
      console.error("Failed to trigger agent update:", error);
      alert("An error occurred while sending the update command.");
    }
  };

  const handleVpsDelete = async (vpsId: number) => {
    // eslint-disable-next-line no-restricted-globals
    if (confirm('Are you sure you want to delete this VPS? This action cannot be undone.')) {
      try {
        await vpsService.deleteVps(vpsId);
        // The store will be updated via WebSocket, so no need to manually refetch
        alert('VPS deleted successfully.');
      } catch (error) {
        console.error("Failed to delete VPS:", error);
        alert("An error occurred while deleting the VPS.");
      }
    }
  };

  // --- Bulk Edit Logic ---
  // This logic is now encapsulated within the BulkEditTagsModal component.


  // --- Derived Data for Display ---
  const uniqueGroups = useMemo(() => {
    const groups = new Set(vpsList.map(s => s.group).filter((g): g is string => !!g));
    return ['ALL', ...Array.from(groups).sort()];
  }, [vpsList]);

  if (isLoadingServers && vpsList.length === 0) {
    return <div className="flex flex-col items-center justify-center h-64"><p className="mt-4 text-slate-600">Loading servers...</p></div>;
  }

  let statusMessage = '';
  if (connectionStatus === 'connecting') statusMessage = '正在连接到实时服务器...';
  else if (connectionStatus === 'reconnecting') statusMessage = '连接已断开，正在尝试重新连接...';
  else if (wsError && (connectionStatus === 'error' || connectionStatus === 'permanently_failed')) statusMessage = `无法连接到实时服务器: ${wsError}`;

  return (
    <div className="p-4 md:p-6 lg:p-8 space-y-8 bg-slate-50 min-h-screen">
      {/* Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center">
        <h1 className="text-3xl font-bold text-slate-800">服务器管理</h1>
        <button
          onClick={handleOpenCreateVpsModal}
          className="mt-3 sm:mt-0 inline-flex items-center justify-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-all duration-150"
        >
          <PlusIcon className="w-5 h-5 mr-2 -ml-1" />
          创建新的VPS
        </button>
      </div>
      <CreateVpsModal isOpen={isCreateVpsModalOpen} onClose={handleCloseCreateVpsModal} onVpsCreated={handleVpsCreated} />
      <EditVpsModal isOpen={isEditModalOpen} onClose={handleCloseEditModal} vps={editingVps} allVps={vpsList} onVpsUpdated={handleVpsUpdated} />
      <CopyCommandModal isOpen={isCopyCommandModalOpen} onClose={handleCloseCopyCommandModal} vps={vpsForCommand} />

      {/* Connection Status */}
      {statusMessage && <div className={`p-3 rounded-md text-sm text-center ${connectionStatus === 'error' || connectionStatus === 'permanently_failed' ? 'bg-red-100 text-red-700' : 'bg-yellow-100 text-yellow-700'}`}>{statusMessage}</div>}


      {/* Server Fleet */}
      <section>
        <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-4">
          <h2 className="text-2xl font-semibold text-slate-700">服务器列表</h2>
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
                    {availableTags.length > 0 && (
                        <div className="flex flex-wrap gap-2 items-center">
                            <span className="text-sm font-medium text-slate-600">按标签筛选:</span>
                            {/* Display selected tags */}
                            {Array.from(selectedTagIds).map(tagId => {
                                const tag = availableTags.find(t => t.id === tagId);
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
                                            {availableTags
                                                .filter(t => t.isVisible)
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
                {selectedVpsIds.size > 0 && (
                <button
                  onClick={() => setIsBulkEditModalOpen(true)}
                  className="bg-slate-200 hover:bg-slate-300 text-slate-700 font-medium py-1.5 px-4 rounded-md transition-colors duration-200 text-sm flex items-center"
                >
                    <PencilSquareIcon className="w-4 h-4 mr-2" />
                    批量编辑标签 ({selectedVpsIds.size})
               </button>
                )}
                {selectedVpsIds.size > 0 && (
                  <button
                    onClick={() => handleTriggerUpdate(Array.from(selectedVpsIds))}
                    className="bg-blue-600 hover:bg-blue-700 text-white font-medium py-1.5 px-4 rounded-md transition-colors duration-200 text-sm flex items-center"
                  >
                    <RefreshCwIcon className="w-4 h-4 mr-2" />
                    更新 Agent ({selectedVpsIds.size})
                  </button>
                )}
           </div>
        </div>

        {displayedServers.length === 0 && !isLoadingServers ? (
          <p className="text-slate-500 text-center py-8 bg-white rounded-lg shadow">没有找到符合当前筛选条件的服务器。</p>
        ) : (
          <div className="bg-white rounded-xl shadow-lg overflow-x-auto">
            <table className="w-full min-w-[1000px]">
              <thead className="bg-slate-100">
                <tr>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider w-8">
                    <input
                      type="checkbox"
                      className="h-4 w-4 text-indigo-600 border-slate-300 rounded focus:ring-indigo-500"
                      onChange={(e) => {
                        const allIds = new Set(displayedServers.map(s => s.id));
                        setSelectedVpsIds(e.target.checked ? allIds : new Set());
                      }}
                      checked={selectedVpsIds.size > 0 && selectedVpsIds.size === displayedServers.length}
                      ref={el => {
                        if (el) {
                          el.indeterminate = selectedVpsIds.size > 0 && selectedVpsIds.size < displayedServers.length;
                        }
                      }}
                    />
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">名称</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">状态</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">IP 地址</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">操作系统</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">Agent 版本</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">分组</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">续费状态</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-slate-500 uppercase tracking-wider">操作</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-200">
                {displayedServers.map(server => (
                  <ServerManagementTableRow
                    key={server.id}
                    server={server}
                    onEdit={handleOpenEditModal}
                    onCopyCommand={handleOpenCopyCommandModal}
                    onTriggerUpdate={(vpsId) => handleTriggerUpdate([vpsId])}
                    onDelete={handleVpsDelete}
                    isSelected={selectedVpsIds.has(server.id)}
                    onSelectionChange={(vpsId, isSelected) => {
                      const newSet = new Set(selectedVpsIds);
                      if (isSelected) {
                        newSet.add(vpsId);
                      } else {
                        newSet.delete(vpsId);
                      }
                      setSelectedVpsIds(newSet);
                    }}
                  />
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <BulkEditTagsModal
        isOpen={isBulkEditModalOpen}
        onClose={() => setIsBulkEditModalOpen(false)}
        vpsIds={Array.from(selectedVpsIds)}
        onTagsUpdated={() => {
          // The backend will push updates via WebSocket.
          // Clearing selection is a good practice after the action is done.
          setSelectedVpsIds(new Set());
        }}
      />
    </div>
  );
};

export default ServerManagementPage;