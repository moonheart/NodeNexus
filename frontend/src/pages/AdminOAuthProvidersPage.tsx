import React, { useState, useEffect } from 'react';
import { useAuthStore } from '../store/authStore';
import toast from 'react-hot-toast';
import ProviderFormModal, { type ProviderFormData } from '../components/ProviderFormModal';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Avatar, AvatarImage, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { MoreHorizontal, Trash2, Edit } from 'lucide-react';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { Skeleton } from '@/components/ui/skeleton';

export interface OAuthProvider {
    provider_name: string;
    client_id: string;
    client_secret: string;
    auth_url: string;
    token_url: string;
    user_info_url: string;
    scopes: string | null;
    icon_url: string | null;
    user_info_mapping: {
        id_field: string;
        username_field: string;
    } | null;
    enabled: boolean;
}

const AdminOAuthProvidersPage: React.FC = () => {
    const [providers, setProviders] = useState<OAuthProvider[]>([]);
    const [loading, setLoading] = useState(true);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [editingProvider, setEditingProvider] = useState<Partial<ProviderFormData> | undefined>(undefined);
    const { token } = useAuthStore();

    const fetchProviders = async () => {
        setLoading(true);
        try {
            const response = await fetch('/api/admin/oauth/providers', {
                headers: { 'Authorization': `Bearer ${token}` },
            });
            if (!response.ok) throw new Error('Failed to fetch OAuth providers.');
            const data: OAuthProvider[] = await response.json();
            setProviders(data);
        } catch (error) {
            toast.error(error instanceof Error ? error.message : 'An unknown error occurred.');
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchProviders();
    }, [token]);

    const handleAddProvider = () => {
        setEditingProvider(undefined);
        setIsModalOpen(true);
    };

    const handleEditProvider = (provider: OAuthProvider) => {
        const providerDataForForm: Partial<ProviderFormData> = {
            ...provider,
            scopes: provider.scopes || undefined,
            icon_url: provider.icon_url || undefined,
            user_info_mapping: provider.user_info_mapping || undefined,
        };
        setEditingProvider(providerDataForForm);
        setIsModalOpen(true);
    };

    const handleDeleteProvider = async (providerName: string) => {
        if (!window.confirm(`Are you sure you want to delete the provider "${providerName}"?`)) return;
        try {
            const response = await fetch(`/api/admin/oauth/providers/${providerName}`, {
                method: 'DELETE',
                headers: { 'Authorization': `Bearer ${token}` },
            });
            if (!response.ok) {
                const errorData = await response.json();
                throw new Error(errorData.message || 'Failed to delete provider.');
            }
            toast.success('Provider deleted successfully!');
            fetchProviders();
        } catch (error) {
            toast.error(error instanceof Error ? error.message : 'An unknown error occurred.');
        }
    };

    const handleSaveProvider = async (providerData: Partial<ProviderFormData>) => {
        const isEditing = !!editingProvider;
        const providerName = isEditing ? editingProvider.provider_name : providerData.provider_name;
        if (!providerName) {
            toast.error("Provider name is required.");
            return;
        }
        const url = isEditing ? `/api/admin/oauth/providers/${providerName}` : '/api/admin/oauth/providers';
        const method = isEditing ? 'PUT' : 'POST';
        try {
            const response = await fetch(url, {
                method: method,
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${token}`,
                },
                body: JSON.stringify(providerData),
            });
            if (!response.ok) {
                const errorData = await response.json();
                throw new Error(errorData.message || 'Failed to save provider.');
            }
            toast.success(`Provider ${isEditing ? 'updated' : 'created'} successfully!`);
            setIsModalOpen(false);
            fetchProviders();
        } catch (error) {
            toast.error(error instanceof Error ? error.message : 'An unknown error occurred.');
        }
    };

    const SkeletonRow = () => (
        <TableRow>
            <TableCell><Skeleton className="h-6 w-32" /></TableCell>
            <TableCell><Skeleton className="h-6 w-48" /></TableCell>
            <TableCell><Skeleton className="h-6 w-16" /></TableCell>
            <TableCell><Skeleton className="h-8 w-8" /></TableCell>
        </TableRow>
    );

    return (
        <div className="space-y-6">
            <ProviderFormModal
                isOpen={isModalOpen}
                onClose={() => setIsModalOpen(false)}
                onSave={handleSaveProvider}
                initialData={editingProvider}
            />
            <Card>
                <CardHeader>
                    <div className="flex justify-between items-center">
                        <div>
                            <CardTitle>OAuth Provider Management</CardTitle>
                            <CardDescription>Manage third-party login providers for your application.</CardDescription>
                        </div>
                        <Button onClick={handleAddProvider}>Add New Provider</Button>
                    </div>
                </CardHeader>
                <CardContent>
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHead>Provider</TableHead>
                                <TableHead>Client ID</TableHead>
                                <TableHead>Enabled</TableHead>
                                <TableHead className="text-right">Actions</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {loading ? (
                                <>
                                    <SkeletonRow />
                                    <SkeletonRow />
                                    <SkeletonRow />
                                </>
                            ) : providers.length > 0 ? (
                                providers.map((provider) => (
                                    <TableRow key={provider.provider_name}>
                                        <TableCell className="font-medium">
                                            <div className="flex items-center gap-2">
                                                <Avatar className="h-6 w-6">
                                                    <AvatarImage src={provider.icon_url || ''} alt={provider.provider_name} />
                                                    <AvatarFallback>{provider.provider_name.charAt(0).toUpperCase()}</AvatarFallback>
                                                </Avatar>
                                                {provider.provider_name}
                                            </div>
                                        </TableCell>
                                        <TableCell className="font-mono text-muted-foreground">{provider.client_id}</TableCell>
                                        <TableCell>
                                            <Badge variant={provider.enabled ? 'default' : 'outline'}>
                                                {provider.enabled ? 'Enabled' : 'Disabled'}
                                            </Badge>
                                        </TableCell>
                                        <TableCell className="text-right">
                                            <DropdownMenu>
                                                <DropdownMenuTrigger asChild>
                                                    <Button variant="ghost" size="icon">
                                                        <MoreHorizontal className="h-4 w-4" />
                                                    </Button>
                                                </DropdownMenuTrigger>
                                                <DropdownMenuContent align="end">
                                                    <DropdownMenuItem onClick={() => handleEditProvider(provider)}>
                                                        <Edit className="mr-2 h-4 w-4" />
                                                        <span>Edit</span>
                                                    </DropdownMenuItem>
                                                    <DropdownMenuItem onClick={() => handleDeleteProvider(provider.provider_name)} className="text-destructive focus:text-destructive">
                                                        <Trash2 className="mr-2 h-4 w-4" />
                                                        <span>Delete</span>
                                                    </DropdownMenuItem>
                                                </DropdownMenuContent>
                                            </DropdownMenu>
                                        </TableCell>
                                    </TableRow>
                                ))
                            ) : (
                                <TableRow>
                                    <TableCell colSpan={4} className="text-center text-muted-foreground">
                                        No providers configured.
                                    </TableCell>
                                </TableRow>
                            )}
                        </TableBody>
                    </Table>
                </CardContent>
            </Card>
        </div>
    );
};

export default AdminOAuthProvidersPage;