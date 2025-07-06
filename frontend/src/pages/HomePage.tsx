import React, { useState, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { VpsListItemResponse, ServerStatus as ServerStatusType, ViewMode, Tag } from '../types';
import { useServerListStore, type ServerListState, type ConnectionStatus } from '../store/serverListStore';
import { useAuthStore } from '../store/authStore';
import { useShallow } from 'zustand/react/shallow';
import StatCard from '../components/StatCard';
import { Server, CheckCircle, XCircle, Power, AlertTriangle, ArrowUp, ArrowDown, LayoutGrid, List, Loader2, Tag as TagIcon, X } from 'lucide-react';
import { STATUS_ONLINE, STATUS_OFFLINE, STATUS_REBOOTING, STATUS_PROVISIONING, STATUS_ERROR, STATUS_UNKNOWN } from '../types';
import VpsCard from '../components/VpsCard';
import VpsTableRow from '../components/VpsTableRow';
import * as tagService from '../services/tagService';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Button } from '@/components/ui/button';
import { DropdownMenu, DropdownMenuContent, DropdownMenuCheckboxItem, DropdownMenuTrigger, DropdownMenuLabel, DropdownMenuSeparator } from '@/components/ui/dropdown-menu';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Table, TableBody, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';

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
  [STATUS_ONLINE]: 'text-success',
  [STATUS_OFFLINE]: 'text-destructive',
  [STATUS_REBOOTING]: 'text-warning',
  [STATUS_PROVISIONING]: 'text-secondary',
  [STATUS_ERROR]: 'text-destructive',
  [STATUS_UNKNOWN]: 'text-muted',
};

const HomePage: React.FC = () => {
  const { t } = useTranslation();
  const { isAuthenticated } = useAuthStore();
  const [selectedStatusFilter, setSelectedStatusFilter] = useState<ServerStatusType | null>(null);
  const [selectedGroup, setSelectedGroup] = useState<string>('ALL');
  const [availableTags, setAvailableTags] = useState<Tag[]>([]);
  const [selectedTagIds, setSelectedTagIds] = useState<Set<number>>(new Set());
  const [sortKey, setSortKey] = useState('id');
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('asc');

  const {
    servers: vpsList,
    isLoading: isLoadingServers,
    error: wsError,
    connectionStatus,
    viewMode,
    setViewMode,
  } = useServerListStore(useShallow(selectHomePageData));

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
        setAvailableTags([]);
      }
    };
    fetchTags();
  }, [isAuthenticated, vpsList]);

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

  const sortedServers = useMemo(() => {
    const sorted = [...tagFilteredServers];
    sorted.sort((a, b) => {
      const getVal = (server: VpsListItemResponse, key: string) => {
        switch (key) {
          case 'id': return server.id;
          case 'name': return server.name.toLowerCase();
          case 'status': return server.status;
          case 'ipAddress': return server.ipAddress?.toLowerCase() || null;
          case 'osType': return server.osType?.toLowerCase() || null;
          case 'cpu': return server.latestMetrics?.cpuUsagePercent ?? null;
          case 'memory': {
            const memUsage = server.latestMetrics?.memoryUsageBytes;
            const memTotal = server.latestMetrics?.memoryTotalBytes;
            if (memUsage === undefined || memUsage === null || memTotal === undefined || memTotal === null || memTotal === 0) return null;
            return memUsage / memTotal;
          }
          case 'traffic': {
            const rx = server.trafficCurrentCycleRxBytes;
            const tx = server.trafficCurrentCycleTxBytes;
            if (rx === undefined || rx === null || tx === undefined || tx === null) return null;
            return rx + tx;
          }
          case 'networkUp': return server.latestMetrics?.networkTxInstantBps ?? null;
          case 'networkDown': return server.latestMetrics?.networkRxInstantBps ?? null;
          default: return null;
        }
      };

      const valA = getVal(a, sortKey);
      const valB = getVal(b, sortKey);
      const direction = sortDirection === 'asc' ? 1 : -1;

      if (valA === null) return 1;
      if (valB === null) return -1;

      if (typeof valA === 'string' && typeof valB === 'string') {
        return valA.localeCompare(valB) * direction;
      }
      if (typeof valA === 'number' && typeof valB === 'number') {
        return (valA - valB) * direction;
      }
      
      return 0;
    });
    return sorted;
  }, [tagFilteredServers, sortKey, sortDirection]);

  const displayedServers = sortedServers;

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
  const formatNetworkSpeedForDisplay = (bps: number): [string, string] => {
    if (bps < 1024) return [bps.toFixed(0), 'Bps'];
    if (bps < 1024 * 1024) return [(bps / 1024).toFixed(1), 'KBps'];
    if (bps < 1024 * 1024 * 1024) return [(bps / (1024 * 1024)).toFixed(1), 'MBps'];
    return [(bps / (1024 * 1024 * 1024)).toFixed(1), 'GBps'];
  };

  const sortOptions = useMemo(() => [
    { key: 'id', label: t('homePage.serverList.sorting.options.default') },
    { key: 'name', label: t('homePage.serverList.sorting.options.name') },
    { key: 'status', label: t('homePage.serverList.sorting.options.status') },
    { key: 'ipAddress', label: t('homePage.serverList.sorting.options.ipAddress') },
    { key: 'osType', label: t('homePage.serverList.sorting.options.os') },
    { key: 'cpu', label: t('homePage.serverList.sorting.options.cpu') },
    { key: 'memory', label: t('homePage.serverList.sorting.options.memory') },
    { key: 'traffic', label: t('homePage.serverList.sorting.options.trafficUsage') },
    { key: 'networkUp', label: t('homePage.serverList.sorting.options.networkUp') },
    { key: 'networkDown', label: t('homePage.serverList.sorting.options.networkDown') },
  ], [t]);

  if (isLoadingServers && vpsList.length === 0) {
    return <div className="flex flex-col items-center justify-center h-screen"><Loader2 className="h-8 w-8 animate-spin text-primary" /><p className="mt-4 text-muted-foreground">{t('homePage.loadingServers')}</p></div>;
  }

  let statusMessage = '';
  let statusVariant: 'default' | 'destructive' = 'default';
  if (connectionStatus === 'connecting') {
    statusMessage = t('homePage.connection.connecting');
  } else if (connectionStatus === 'reconnecting') {
    statusMessage = t('homePage.connection.reconnecting');
  } else if (wsError && (connectionStatus === 'error' || connectionStatus === 'permanently_failed')) {
    statusMessage = t('homePage.connection.error', { error: wsError });
    statusVariant = 'destructive';
  }

  const handleTagSelection = (tagId: number) => {
    const newSet = new Set(selectedTagIds);
    if (newSet.has(tagId)) {
      newSet.delete(tagId);
    } else {
      newSet.add(tagId);
    }
    setSelectedTagIds(newSet);
  };

  const [totalNetworkUpValue, totalNetworkUpUnit] = formatNetworkSpeedForDisplay(totalNetworkUp);
  const [totalNetworkDownValue, totalNetworkDownUnit] = formatNetworkSpeedForDisplay(totalNetworkDown);

  return (
    <div className="p-4 md:p-6 lg:p-8 space-y-6">
      {statusMessage && <Alert variant={statusVariant} className="mb-6"><AlertTriangle className="h-4 w-4" /><AlertDescription>{statusMessage}</AlertDescription></Alert>}

      <Card className='backdrop-blur'>
        <CardHeader>
          <CardTitle>{t('homePage.overview.title')}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 2xl:grid-cols-8 gap-4 md:gap-6">
            <StatCard title={t('homePage.overview.totalServers')} value={serverStats.total} icon={<Server className="w-6 h-6" />} valueClassName="text-primary" onClick={() => setSelectedStatusFilter(null)} isActive={selectedStatusFilter === null} />
            <StatCard title={t('homePage.overview.online')} value={serverStats[STATUS_ONLINE]} icon={<CheckCircle className="w-6 h-6" />} valueClassName={statusColorMap[STATUS_ONLINE]} onClick={() => setSelectedStatusFilter(STATUS_ONLINE)} isActive={selectedStatusFilter === STATUS_ONLINE} />
            <StatCard title={t('homePage.overview.offline')} value={serverStats[STATUS_OFFLINE]} icon={<XCircle className="w-6 h-6" />} valueClassName={statusColorMap[STATUS_OFFLINE]} onClick={() => setSelectedStatusFilter(STATUS_OFFLINE)} isActive={selectedStatusFilter === STATUS_OFFLINE} />
            {serverStats[STATUS_REBOOTING] > 0 && <StatCard title={t('homePage.overview.rebooting')} value={serverStats[STATUS_REBOOTING]} icon={<Power className="w-6 h-6" />} valueClassName={statusColorMap[STATUS_REBOOTING]} onClick={() => setSelectedStatusFilter(STATUS_REBOOTING)} isActive={selectedStatusFilter === STATUS_REBOOTING} />}
            {serverStats[STATUS_ERROR] > 0 && <StatCard title={t('homePage.overview.error')} value={serverStats[STATUS_ERROR]} icon={<AlertTriangle className="w-6 h-6" />} valueClassName={statusColorMap[STATUS_ERROR]} onClick={() => setSelectedStatusFilter(STATUS_ERROR)} isActive={selectedStatusFilter === STATUS_ERROR} />}
            <StatCard title={t('homePage.overview.totalUpload')} value={totalNetworkUpValue} unit={totalNetworkUpUnit} icon={<ArrowUp className="w-6 h-6" />} valueClassName="text-emerald-500" description={t('homePage.overview.onlineServers')} />
            <StatCard title={t('homePage.overview.totalDownload')} value={totalNetworkDownValue} unit={totalNetworkDownUnit} icon={<ArrowDown className="w-6 h-6" />} valueClassName="text-sky-500" description={t('homePage.overview.onlineServers')} />
          </div>
        </CardContent>
      </Card>

      <Card className='backdrop-blur'>
        <CardHeader>
          <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
            <CardTitle>{t('homePage.serverList.title')}</CardTitle>
            <ToggleGroup type="single" value={viewMode} onValueChange={(value) => value && setViewMode(value as ViewMode)} aria-label="View mode">
              <ToggleGroupItem value="card" aria-label="Card view" className={"px-2"}><LayoutGrid className="h-4 w-4" />{t('homePage.serverList.viewMode.card')}</ToggleGroupItem>
              <ToggleGroupItem value="list" aria-label="List view" className={"px-2"}><List className="h-4 w-4" />{t('homePage.serverList.viewMode.list')}</ToggleGroupItem>
            </ToggleGroup>
          </div>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-4 items-center justify-between p-4 border rounded-lg mb-6">
            <div className="flex flex-wrap gap-4 items-center">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium">{t('homePage.serverList.filters.group')}</span>
                <ToggleGroup type="single" value={selectedGroup} onValueChange={(value) => value && setSelectedGroup(value)} aria-label="Group filter">
                  {uniqueGroups.map(group => (
                    <ToggleGroupItem key={group} value={group} className={"px-4"}>{group === 'ALL' ? t('homePage.serverList.filters.allGroups') : group}</ToggleGroupItem>
                  ))}
                </ToggleGroup>
              </div>
              {currentAvailableTags.length > 0 && (
                <div className="flex items-center gap-2">
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="outline">
                        <TagIcon className="mr-2 h-4 w-4" />
                        {t('homePage.serverList.filters.filterByTag')} {selectedTagIds.size > 0 && `(${selectedTagIds.size})`}
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent className="w-56">
                      <DropdownMenuLabel>{t('homePage.serverList.filters.visibleTags')}</DropdownMenuLabel>
                      <DropdownMenuSeparator />
                      {currentAvailableTags.map(tag => (
                        <DropdownMenuCheckboxItem
                          key={tag.id}
                          checked={selectedTagIds.has(tag.id)}
                          onCheckedChange={() => handleTagSelection(tag.id)}
                        >
                          <span className="inline-block w-2 h-2 mr-2 rounded-full" style={{ backgroundColor: tag.color }}></span>
                          {tag.name}
                        </DropdownMenuCheckboxItem>
                      ))}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              )}
            </div>
            <div className="flex items-center gap-2">
              <Select value={sortKey} onValueChange={setSortKey}>
                <SelectTrigger className="w-[180px]">
                  <SelectValue placeholder={t('homePage.sorting.placeholder')} />
                </SelectTrigger>
                <SelectContent>
                  {sortOptions.map(option => (
                    <SelectItem key={option.key} value={option.key}>{option.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <ToggleGroup type="single" value={sortDirection} onValueChange={(value) => value && setSortDirection(value as 'asc' | 'desc')}>
                <ToggleGroupItem value="asc" aria-label="Ascending"><ArrowUp className="h-4 w-4" /></ToggleGroupItem>
                <ToggleGroupItem value="desc" aria-label="Descending"><ArrowDown className="h-4 w-4" /></ToggleGroupItem>
              </ToggleGroup>
            </div>
          </div>
          
          {selectedTagIds.size > 0 && (
            <div className="flex flex-wrap gap-2 mb-4">
              {Array.from(selectedTagIds).map(tagId => {
                const tag = currentAvailableTags.find(t => t.id === tagId);
                if (!tag) return null;
                return (
                  <Badge key={tag.id} variant="secondary" className="pl-2">
                    <span className="inline-block w-2 h-2 mr-2 rounded-full" style={{ backgroundColor: tag.color }}></span>
                    {tag.name}
                    <button onClick={() => handleTagSelection(tag.id)} className="ml-1 rounded-full hover:bg-muted-foreground/20 p-0.5">
                      <X className="h-3 w-3" />
                    </button>
                  </Badge>
                );
              })}
            </div>
          )}

          {displayedServers.length === 0 && !isLoadingServers ? (
            <div className="text-center py-12 text-muted-foreground">
              <p>{t('homePage.serverList.empty')}</p>
            </div>
          ) : viewMode === 'card' ? (
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-6">
              {displayedServers.map(server => <VpsCard key={server.id} server={server} />)}
            </div>
          ) : (
            <div className="border rounded-lg overflow-hidden">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t('homePage.serverList.table.name')}</TableHead>
                    <TableHead>{t('homePage.serverList.table.status')}</TableHead>
                    <TableHead>{t('homePage.serverList.table.ipAddress')}</TableHead>
                    <TableHead>{t('homePage.serverList.table.os')}</TableHead>
                    <TableHead>{t('homePage.serverList.table.cpu')}</TableHead>
                    <TableHead>{t('homePage.serverList.table.memory')}</TableHead>
                    <TableHead>{t('homePage.serverList.table.traffic')}</TableHead>
                    <TableHead>{t('homePage.serverList.table.renewal')}</TableHead>
                    <TableHead>{t('homePage.serverList.table.upload')}</TableHead>
                    <TableHead>{t('homePage.serverList.table.download')}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {displayedServers.map(server => <VpsTableRow key={server.id} server={server} />)}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
};

export default HomePage;