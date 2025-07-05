import React, { useState, useEffect } from 'react';
import toast from 'react-hot-toast';
import ProviderFormModal, { type ProviderFormData } from '../components/ProviderFormModal';
import apiClient from '../services/apiClient';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardAction } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Avatar, AvatarImage, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Trash2, Edit } from 'lucide-react';
import { Skeleton } from '@/components/ui/skeleton';
import { useTranslation } from 'react-i18next';

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
    const { t } = useTranslation();
    const [providers, setProviders] = useState<OAuthProvider[]>([]);
    const [loading, setLoading] = useState(true);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [editingProvider, setEditingProvider] = useState<Partial<ProviderFormData> | undefined>(undefined);
    const fetchProviders = async () => {
        setLoading(true);
        try {
            const response = await apiClient.get<OAuthProvider[]>('/admin/oauth/providers');
            setProviders(response.data);
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'An unknown error occurred.';
            toast.error(errorMessage);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchProviders();
    }, []);

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
        if (!window.confirm(t('adminOAuthProviders.deleteDialog.description', { providerName }))) return;
        try {
            await apiClient.delete(`/admin/oauth/providers/${providerName}`);
            toast.success(t('adminOAuthProviders.notifications.providerDeleted'));
            fetchProviders();
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'An unknown error occurred.';
            toast.error(errorMessage);
        }
    };

    const handleSaveProvider = async (providerData: Partial<ProviderFormData>) => {
        const isEditing = !!editingProvider;
        const providerName = isEditing ? editingProvider.provider_name : providerData.provider_name;
        if (!providerName) {
            toast.error("Provider name is required.");
            return;
        }

        const url = isEditing ? `/admin/oauth/providers/${providerName}` : '/admin/oauth/providers';
        const method = isEditing ? 'put' : 'post';

        try {
            await apiClient[method](url, providerData);
            toast.success(t('adminOAuthProviders.notifications.providerSaved', {
                status: isEditing ? t('adminOAuthProviders.notifications.status.updated') : t('adminOAuthProviders.notifications.status.created')
            }));
            setIsModalOpen(false);
            fetchProviders();
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'An unknown error occurred.';
            toast.error(errorMessage);
        }
    };

    const SkeletonRow = () => (
        <TableRow>
            <TableCell><Skeleton className="h-6 w-32" /></TableCell>
            <TableCell><Skeleton className="h-6 w-48" /></TableCell>
            <TableCell><Skeleton className="h-6 w-16" /></TableCell>
            <TableCell><Skeleton className="h-8 w-24" /></TableCell>
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

                    <CardTitle>{t('adminOAuthProviders.title')}</CardTitle>
                    <CardDescription>{t('adminOAuthProviders.description')}</CardDescription>
                    <CardAction>
                        <Button onClick={handleAddProvider}>{t('adminOAuthProviders.addNew')}</Button>
                    </CardAction>
                </CardHeader>
                <CardContent>
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHead>{t('adminOAuthProviders.table.provider')}</TableHead>
                                <TableHead>{t('adminOAuthProviders.table.clientId')}</TableHead>
                                <TableHead>{t('adminOAuthProviders.table.enabled')}</TableHead>
                                <TableHead className="text-right">{t('adminOAuthProviders.table.actions')}</TableHead>
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
                                                {provider.enabled ? t('adminOAuthProviders.status.enabled') : t('adminOAuthProviders.status.disabled')}
                                            </Badge>
                                        </TableCell>
                                        <TableCell className="text-right">
                                            <div className="flex items-center justify-end gap-2">
                                                <Button variant="ghost" size="icon" onClick={() => handleEditProvider(provider)}>
                                                    <Edit className="h-4 w-4" />
                                                </Button>
                                                <Button variant="ghost" size="icon" onClick={() => handleDeleteProvider(provider.provider_name)} className="text-destructive hover:text-destructive">
                                                    <Trash2 className="h-4 w-4" />
                                                </Button>
                                            </div>
                                        </TableCell>
                                    </TableRow>
                                ))
                            ) : (
                                <TableRow>
                                    <TableCell colSpan={4} className="text-center text-muted-foreground">
                                        {t('adminOAuthProviders.empty.title')}
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