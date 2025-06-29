import React, { useState, useEffect, useMemo } from 'react';
import EditVpsModal from '../components/EditVpsModal';
import CreateVpsModal from '../components/CreateVpsModal';
import CopyCommandModal from '../components/CopyCommandModal';
import type { Vps, VpsListItemResponse, Tag } from '../types';
import { useServerListStore, type ServerListState, type ConnectionStatus } from '../store/serverListStore';
import { useShallow } from 'zustand/react/shallow';
import { Plus, RefreshCw, Pencil, Tag as TagIcon } from 'lucide-react';
import ServerManagementTableRow from '../components/ServerManagementTableRow';
import BulkEditTagsModal from '../components/BulkEditTagsModal';
import * as tagService from '../services/tagService';
import * as vpsService from '../services/vpsService';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { Table, TableBody, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuTrigger,
    DropdownMenuCheckboxItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import toast from 'react-hot-toast';

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
    const [isAlertOpen, setIsAlertOpen] = useState(false);
    const [vpsToDelete, setVpsToDelete] = useState<number | null>(null);

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
                toast.error("Failed to fetch tags.");
            }
        };
        fetchTags();
    }, []);

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

    const handleOpenCreateVpsModal = () => setIsCreateVpsModalOpen(true);
    const handleCloseCreateVpsModal = () => setIsCreateVpsModalOpen(false);
    const handleVpsCreated = (newVps: Vps) => {
        handleCloseCreateVpsModal();
        const vpsForCommand: VpsListItemResponse = { ...newVps, userId: newVps.user_id, agentSecret: newVps.agent_secret, ipAddress: newVps.ip_address, osType: newVps.os_type, createdAt: newVps.created_at, updatedAt: newVps.updated_at, agentVersion: null, latestMetrics: null, configStatus: 'unknown', lastConfigUpdateAt: null, lastConfigError: null, renewalCycle: null, renewalCycleCustomDays: null, renewalPrice: null, renewalCurrency: null, nextRenewalDate: null, lastRenewalDate: null, serviceStartDate: null, paymentMethod: null, autoRenewEnabled: null, renewalNotes: null, reminderActive: null };
        handleOpenCopyCommandModal(vpsForCommand);
    };

    const handleOpenEditModal = (server: VpsListItemResponse) => {
        setEditingVps(server);
        setIsEditModalOpen(true);
    };

    const handleOpenCopyCommandModal = async (server: VpsListItemResponse) => {
        try {
            const fullVpsDetails = await vpsService.getVpsDetail(server.id.toString());
            setVpsForCommand(fullVpsDetails);
            setIsCopyCommandModalOpen(true);
        } catch (error) {
            console.error("Failed to fetch VPS details for command copy:", error);
            toast.error("Could not fetch installation command.");
        }
    };

    const handleCloseEditModal = () => {
        setIsEditModalOpen(false);
        setEditingVps(null);
    };

    const handleVpsUpdated = () => {
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
            toast.success(`Update command sent. Success: ${result.successfulCount}, Failed: ${result.failedCount}`);
        } catch (error) {
            console.error("Failed to trigger agent update:", error);
            toast.error("An error occurred while sending the update command.");
        }
    };

    const confirmDelete = (id: number) => {
        setVpsToDelete(id);
        setIsAlertOpen(true);
    };

    const handleVpsDelete = async () => {
        if (vpsToDelete === null) return;
        try {
            await vpsService.deleteVps(vpsToDelete);
            toast.success('VPS deleted successfully.');
        } catch (error) {
            console.error("Failed to delete VPS:", error);
            toast.error("An error occurred while deleting the VPS.");
        } finally {
            setIsAlertOpen(false);
            setVpsToDelete(null);
        }
    };

    const uniqueGroups = useMemo(() => {
        const groups = new Set(vpsList.map(s => s.group).filter((g): g is string => !!g));
        return ['ALL', ...Array.from(groups).sort()];
    }, [vpsList]);

    const handleSelectAll = (checked: boolean) => {
        setSelectedVpsIds(checked ? new Set(displayedServers.map(s => s.id)) : new Set());
    };

    const handleSelectionChange = (vpsId: number, isSelected: boolean) => {
        const newSet = new Set(selectedVpsIds);
        if (isSelected) newSet.add(vpsId);
        else newSet.delete(vpsId);
        setSelectedVpsIds(newSet);
    };

    if (isLoadingServers && vpsList.length === 0) {
        return <div className="flex items-center justify-center h-64"><p>Loading servers...</p></div>;
    }

    let statusMessage = '';
    if (connectionStatus === 'connecting') statusMessage = 'Connecting to real-time server...';
    else if (connectionStatus === 'reconnecting') statusMessage = 'Connection lost. Reconnecting...';
    else if (wsError && (connectionStatus === 'error' || connectionStatus === 'permanently_failed')) statusMessage = `Could not connect to real-time server: ${wsError}`;

    return (
        <div className="p-4 md:p-6 lg:p-8 space-y-6">
            <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center">
                <h1 className="text-3xl font-bold">Server Management</h1>
                <Button onClick={handleOpenCreateVpsModal} className="mt-3 sm:mt-0">
                    <Plus className="w-5 h-5 mr-2 -ml-1" />
                    Create New VPS
                </Button>
            </div>
            <CreateVpsModal isOpen={isCreateVpsModalOpen} onClose={handleCloseCreateVpsModal} onVpsCreated={handleVpsCreated} />
            <EditVpsModal isOpen={isEditModalOpen} onClose={handleCloseEditModal} vps={editingVps} allVps={vpsList} onVpsUpdated={handleVpsUpdated} />
            <CopyCommandModal isOpen={isCopyCommandModalOpen} onClose={handleCloseCopyCommandModal} vps={vpsForCommand} />

            {statusMessage && <div className={`p-3 rounded-md text-sm text-center ${connectionStatus === 'error' || connectionStatus === 'permanently_failed' ? 'bg-destructive/10 text-destructive' : 'bg-yellow-100 text-yellow-800'}`}>{statusMessage}</div>}

            <Card>
                <CardHeader>
                    <CardTitle>Server Fleet</CardTitle>
                </CardHeader>
                <CardContent>
                    <div className="flex flex-wrap gap-4 items-center justify-between mb-4">
                        <div className="flex flex-wrap gap-2 items-center">
                            <span className="text-sm font-medium text-muted-foreground">Filter by Group:</span>
                            {uniqueGroups.map(group => (
                                <Button key={group} onClick={() => setSelectedGroup(group)} variant={selectedGroup === group ? 'secondary' : 'outline'} size="sm">
                                    {group === 'ALL' ? 'All' : group}
                                </Button>
                            ))}
                        </div>
                        <div className="flex flex-wrap gap-2 items-center">
                            {availableTags.length > 0 && (
                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button variant="outline">
                                            <TagIcon className="mr-2 h-4 w-4" />
                                            Filter by Tags
                                            {selectedTagIds.size > 0 && <Badge variant="secondary" className="ml-2">{selectedTagIds.size}</Badge>}
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="end">
                                        <DropdownMenuLabel>Visible Tags</DropdownMenuLabel>
                                        <DropdownMenuSeparator />
                                        {availableTags.filter(t => t.isVisible).map(tag => (
                                            <DropdownMenuCheckboxItem
                                                key={tag.id}
                                                checked={selectedTagIds.has(tag.id)}
                                                onCheckedChange={(checked) => {
                                                    const newSet = new Set(selectedTagIds);
                                                    if (checked) newSet.add(tag.id);
                                                    else newSet.delete(tag.id);
                                                    setSelectedTagIds(newSet);
                                                }}
                                            >
                                                <span className="inline-block w-3 h-3 mr-3 rounded-full" style={{ backgroundColor: tag.color }}></span>
                                                {tag.name}
                                            </DropdownMenuCheckboxItem>
                                        ))}
                                    </DropdownMenuContent>
                                </DropdownMenu>
                            )}
                            {selectedVpsIds.size > 0 && (
                                <>
                                    <Button variant="outline" onClick={() => setIsBulkEditModalOpen(true)}>
                                        <Pencil className="w-4 h-4 mr-2" />
                                        Edit Tags ({selectedVpsIds.size})
                                    </Button>
                                    <Button onClick={() => handleTriggerUpdate(Array.from(selectedVpsIds))}>
                                        <RefreshCw className="w-4 h-4 mr-2" />
                                        Update Agent ({selectedVpsIds.size})
                                    </Button>
                                </>
                            )}
                        </div>
                    </div>

                    {displayedServers.length === 0 && !isLoadingServers ? (
                        <p className="text-muted-foreground text-center py-8">No servers match the current filters.</p>
                    ) : (
                        <div className="rounded-md border">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHead className="w-12">
                                            <Checkbox
                                                checked={selectedVpsIds.size > 0 && selectedVpsIds.size === displayedServers.length}
                                                onCheckedChange={handleSelectAll}
                                                aria-label="Select all"
                                            />
                                        </TableHead>
                                        <TableHead>Name</TableHead>
                                        <TableHead>Status</TableHead>
                                        <TableHead>IP Address</TableHead>
                                        <TableHead>OS</TableHead>
                                        <TableHead>Agent</TableHead>
                                        <TableHead>Group</TableHead>
                                        <TableHead>Renewal</TableHead>
                                        <TableHead className="text-right">Actions</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {displayedServers.map(server => (
                                        <ServerManagementTableRow
                                            key={server.id}
                                            server={server}
                                            onEdit={handleOpenEditModal}
                                            onCopyCommand={handleOpenCopyCommandModal}
                                            onTriggerUpdate={(vpsId) => handleTriggerUpdate([vpsId])}
                                            onDelete={confirmDelete}
                                            isSelected={selectedVpsIds.has(server.id)}
                                            onSelectionChange={handleSelectionChange}
                                        />
                                    ))}
                                </TableBody>
                            </Table>
                        </div>
                    )}
                </CardContent>
            </Card>

            <BulkEditTagsModal
                isOpen={isBulkEditModalOpen}
                onClose={() => setIsBulkEditModalOpen(false)}
                vpsIds={Array.from(selectedVpsIds)}
                onTagsUpdated={() => setSelectedVpsIds(new Set())}
            />

            <AlertDialog open={isAlertOpen} onOpenChange={setIsAlertOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>Are you absolutely sure?</AlertDialogTitle>
                        <AlertDialogDescription>
                            This action cannot be undone. This will permanently delete the VPS and all its associated data.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>Cancel</AlertDialogCancel>
                        <AlertDialogAction onClick={handleVpsDelete} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">Delete</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
};

export default ServerManagementPage;