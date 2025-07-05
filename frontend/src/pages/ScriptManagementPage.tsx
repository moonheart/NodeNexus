import React, { useState, useEffect, useMemo } from 'react';
import toast from 'react-hot-toast';
import { useTranslation } from 'react-i18next';
import { scriptService, type ScriptPayload } from '../services/scriptService';
import type { CommandScript } from '../types';
import ScriptFormModal from '../components/ScriptFormModal';
import { Plus, Search, MoreHorizontal } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
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

const ScriptManagementPage: React.FC = () => {
    const { t } = useTranslation();
    const [scripts, setScripts] = useState<CommandScript[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [editingScript, setEditingScript] = useState<CommandScript | undefined>(undefined);
    const [searchQuery, setSearchQuery] = useState('');
    const [isAlertOpen, setIsAlertOpen] = useState(false);
    const [scriptToDelete, setScriptToDelete] = useState<number | null>(null);

    const fetchScripts = async () => {
        try {
            setLoading(true);
            const data = await scriptService.getScripts();
            setScripts(data);
            setError(null);
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'An unknown error occurred.';
            setError(errorMessage);
            toast.error(t('common.notifications.fetchFailed', { error: errorMessage }));
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchScripts();
    }, []);

    const handleAdd = () => {
        setEditingScript(undefined);
        setIsModalOpen(true);
    };

    const handleEdit = (script: CommandScript) => {
        setEditingScript(script);
        setIsModalOpen(true);
    };

    const confirmDelete = (id: number) => {
        setScriptToDelete(id);
        setIsAlertOpen(true);
    };

    const handleDelete = async () => {
        if (scriptToDelete === null) return;
        try {
            await scriptService.deleteScript(scriptToDelete);
            toast.success(t('common.notifications.deleted'));
            fetchScripts();
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'An unknown error occurred.';
            toast.error(t('common.notifications.deleteFailed', { error: errorMessage }));
        } finally {
            setIsAlertOpen(false);
            setScriptToDelete(null);
        }
    };

    const handleSave = async (data: ScriptPayload) => {
        try {
            if (editingScript) {
                await scriptService.updateScript(editingScript.id, data);
                toast.success(t('common.notifications.updated'));
            } else {
                await scriptService.createScript(data);
                toast.success(t('common.notifications.created'));
            }
            setIsModalOpen(false);
            fetchScripts();
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'An unknown error occurred.';
            toast.error(t('common.notifications.saveFailed', { error: errorMessage }));
        }
    };

    const filteredScripts = useMemo(() => {
        return scripts.filter(script =>
            script.name.toLowerCase().includes(searchQuery.toLowerCase())
        );
    }, [scripts, searchQuery]);

    const renderContent = () => {
        if (loading) return <p>{t('common.status.loading')}</p>;
        if (error) return <p className="text-destructive">{t('common.notifications.error', { error })}</p>;

        if (scripts.length === 0) {
            return (
                <div className="text-center py-10">
                    <h3 className="text-lg font-medium">{t('scriptManagement.empty.title')}</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{t('scriptManagement.empty.description')}</p>
                    <div className="mt-6">
                        <Button onClick={handleAdd}>
                            <Plus className="-ml-1 mr-2 h-5 w-5" />
                            {t('scriptManagement.empty.newButton')}
                        </Button>
                    </div>
                </div>
            );
        }

        return (
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHead>{t('common.table.name')}</TableHead>
                        <TableHead>{t('common.table.description')}</TableHead>
                        <TableHead>{t('common.table.language')}</TableHead>
                        <TableHead><span className="sr-only">{t('common.table.actions')}</span></TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {filteredScripts.map((script) => (
                        <TableRow key={script.id}>
                            <TableCell className="font-medium">{script.name}</TableCell>
                            <TableCell className="max-w-xs truncate">{script.description}</TableCell>
                            <TableCell>
                                <Badge variant={script.language === 'shell' ? 'default' : 'secondary'}>
                                    {script.language}
                                </Badge>
                            </TableCell>
                            <TableCell className="text-right">
                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button variant="ghost" className="h-8 w-8 p-0">
                                            <span className="sr-only">{t('common.actions.openMenu')}</span>
                                            <MoreHorizontal className="h-4 w-4" />
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="end">
                                        <DropdownMenuItem onClick={() => handleEdit(script)}>{t('common.actions.edit')}</DropdownMenuItem>
                                        <DropdownMenuItem onClick={() => confirmDelete(script.id)} className="text-destructive">{t('common.actions.delete')}</DropdownMenuItem>
                                    </DropdownMenuContent>
                                </DropdownMenu>
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        );
    };

    return (
        <div className="space-y-6">
            <Card>
                <CardHeader>
                    <div className="flex justify-between items-start">
                        <div>
                            <CardTitle>{t('scriptManagement.title')}</CardTitle>
                            <CardDescription>{t('scriptManagement.description')}</CardDescription>
                        </div>
                        {scripts.length > 0 && (
                            <Button onClick={handleAdd}>{t('scriptManagement.addNew')}</Button>
                        )}
                    </div>
                </CardHeader>
                <CardContent>
                    {scripts.length > 0 && (
                        <div className="relative mb-4">
                            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-5 w-5 text-muted-foreground" />
                            <Input
                                type="search"
                                placeholder={t('common.placeholders.search')}
                                className="pl-10"
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                            />
                        </div>
                    )}
                    {renderContent()}
                </CardContent>
            </Card>

            <ScriptFormModal
                isOpen={isModalOpen}
                onClose={() => setIsModalOpen(false)}
                onSave={handleSave}
                initialData={editingScript}
            />

            <AlertDialog open={isAlertOpen} onOpenChange={setIsAlertOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('common.dialogs.delete.title')}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t('scriptManagement.deleteDialog.description')}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.actions.cancel')}</AlertDialogCancel>
                        <AlertDialogAction onClick={handleDelete}>{t('common.actions.delete')}</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
};

export default ScriptManagementPage;