import React, { useState, useEffect, useCallback } from 'react';
import { useAuthStore } from '../store/authStore';
import toast from 'react-hot-toast';
import * as userService from '../services/userService';
import type { ConnectedAccount, OAuthProvider } from '../services/userService';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
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
import { useTranslation } from 'react-i18next';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

const AccountSettingsPage: React.FC = () => {
    const { t, i18n } = useTranslation();
    const { user, setUser } = useAuthStore();
    const [username, setUsername] = useState(user?.username || '');
    
    const [currentPassword, setCurrentPassword] = useState('');
    const [newPassword, setNewPassword] = useState('');
    const [confirmPassword, setConfirmPassword] = useState('');

    const [connectedAccounts, setConnectedAccounts] = useState<ConnectedAccount[]>([]);
    const [availableProviders, setAvailableProviders] = useState<OAuthProvider[]>([]);
    const [loading, setLoading] = useState(true);
    const [isAlertOpen, setIsAlertOpen] = useState(false);
    const [unlinkingProvider, setUnlinkingProvider] = useState<string | null>(null);
    const [selectedLanguage, setSelectedLanguage] = useState(i18n.language);

    const fetchData = useCallback(async () => {
        try {
            setLoading(true);
            const [accounts, providers] = await Promise.all([
                userService.getConnectedAccounts(),
                userService.getAvailableProviders(),
            ]);
            setConnectedAccounts(accounts);
            setAvailableProviders(providers);
        } catch (error) {
            toast.error(error instanceof Error ? error.message : t('accountSettings.fetchError'));
        } finally {
            setLoading(false);
        }
    }, [t]);

    useEffect(() => {
        if (user) {
            setUsername(user.username);
            fetchData();
        }
    }, [user, fetchData]);

    useEffect(() => {
        const urlParams = new URLSearchParams(window.location.search);
        if (urlParams.get('link_success') === 'true') {
            toast.success(t('accountSettings.linkSuccess'));
            window.history.replaceState({}, document.title, window.location.pathname);
            fetchData();
        }
    }, [fetchData, t]);

    const handleUpdateUsername = async (e: React.FormEvent) => {
        e.preventDefault();
        const toastId = toast.loading(t('common.status.updating'));
        try {
            const updatedUser = await userService.updateUsername(username);
            toast.success(t('accountSettings.updateUsernameSuccess'), { id: toastId });
            if (user) {
                setUser({ ...user, username: updatedUser.username });
            }
        } catch (error) {
            toast.error(error instanceof Error ? error.message : t('accountSettings.updateUsernameError'), { id: toastId });
        }
    };

    const handleChangePassword = async (e: React.FormEvent) => {
        e.preventDefault();
        if (newPassword !== confirmPassword) {
            toast.error(t('common.errors.passwordMismatch'));
            return;
        }
        const toastId = toast.loading(t('common.status.changing'));
        try {
            await userService.updatePassword({ current_password: currentPassword, new_password: newPassword });
            toast.success(t('accountSettings.changePasswordSuccess'), { id: toastId });
            setCurrentPassword('');
            setNewPassword('');
            setConfirmPassword('');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : t('accountSettings.changePasswordError'), { id: toastId });
        }
    };

    const handleUnlinkClick = (providerName: string) => {
        setUnlinkingProvider(providerName);
        setIsAlertOpen(true);
    };

    const confirmUnlinkAccount = async () => {
        if (!unlinkingProvider) return;
        const toastId = toast.loading(t('common.status.unlinking', { provider: unlinkingProvider }));
        try {
            await userService.unlinkProvider(unlinkingProvider);
            toast.success(t('accountSettings.unlinkSuccess', { provider: unlinkingProvider }), { id: toastId });
            fetchData();
        } catch (error) {
            toast.error(error instanceof Error ? error.message : t('accountSettings.unlinkError', { provider: unlinkingProvider }), { id: toastId });
        } finally {
            setIsAlertOpen(false);
            setUnlinkingProvider(null);
        }
    };

    const handleLanguageChange = async (lang: string) => {
        setSelectedLanguage(lang);
        const toastId = toast.loading(t('common.status.updating'));
        try {
            await userService.updateUserLanguage(lang);
            if (lang === 'auto') {
                localStorage.removeItem('i18nextLng');
                // After removing the item, we need to instruct i18next to re-detect the language
                i18n.changeLanguage(undefined);
            } else {
                i18n.changeLanguage(lang);
            }
            toast.success(t('accountSettings.preferences.updateLanguageSuccess'), { id: toastId });
        } catch (error) {
            toast.error(error instanceof Error ? error.message : t('accountSettings.updateLanguageError'), { id: toastId });
            setSelectedLanguage(i18n.language); // Revert on error
        }
    };

    const ProviderIcon = ({ providerName }: { providerName?: string }) => {
        if (!providerName) return null;
        const provider = availableProviders.find(p => p.name === providerName);
        if (provider && provider.iconUrl) {
            return <img src={provider.iconUrl} alt={`${provider.name} icon`} className="w-6 h-6" />;
        }
        return <div className="w-6 h-6 bg-gray-200 rounded-full" />;
    };

    const handleLinkAccount = (providerName: string) => {
        window.location.href = `/api/auth/${providerName}/link`;
    };

    const unlinkedProviders = availableProviders.filter(
        (p) => p.name && !connectedAccounts.some((a) => a.provider_name === p.name)
    );

    return (
        <div className="space-y-6">
            <AlertDialog open={isAlertOpen} onOpenChange={setIsAlertOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('common.dialogs.unlink.title')}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t('common.dialogs.unlink.description')}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel onClick={() => setUnlinkingProvider(null)}>{t('common.actions.cancel')}</AlertDialogCancel>
                        <AlertDialogAction onClick={confirmUnlinkAccount}>{t('common.actions.confirm')}</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>

            <Card>
                <CardHeader>
                    <CardTitle>{t('accountSettings.title')}</CardTitle>
                    <CardDescription>{t('accountSettings.description')}</CardDescription>
                </CardHeader>
                <CardContent>
                    <form onSubmit={handleUpdateUsername} className="space-y-4">
                        <div className="space-y-2">
                            <Label htmlFor="username">{t('accountSettings.usernameLabel')}</Label>
                            <div className="flex">
                                <Input
                                    id="username"
                                    value={username}
                                    onChange={(e) => setUsername(e.target.value)}
                                    className="rounded-r-none"
                                />
                                <Button type="submit" className="rounded-l-none">{t('common.actions.save')}</Button>
                            </div>
                        </div>
                    </form>
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>{t('accountSettings.securityTitle')}</CardTitle>
                    <CardDescription>{t('accountSettings.securityDescription')}</CardDescription>
                </CardHeader>
                <CardContent className="space-y-8">
                    {/* Change Password Form */}
                    <form onSubmit={handleChangePassword} className="space-y-4">
                        <h3 className="text-lg font-medium">{t('accountSettings.changePasswordTitle')}</h3>
                        <div className="space-y-4">
                            <div className="space-y-2">
                                <Label htmlFor="current-password">{t('common.labels.currentPassword')}</Label>
                                <Input
                                    type="password"
                                    id="current-password"
                                    value={currentPassword}
                                    onChange={(e) => setCurrentPassword(e.target.value)}
                                />
                            </div>
                            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                <div className="space-y-2">
                                    <Label htmlFor="new-password">{t('common.labels.newPassword')}</Label>
                                    <Input
                                        type="password"
                                        id="new-password"
                                        value={newPassword}
                                        onChange={(e) => setNewPassword(e.target.value)}
                                    />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="confirm-password">{t('common.labels.confirmNewPassword')}</Label>
                                    <Input
                                        type="password"
                                        id="confirm-password"
                                        value={confirmPassword}
                                        onChange={(e) => setConfirmPassword(e.target.value)}
                                    />
                                </div>
                            </div>
                        </div>
                        <CardFooter className="px-0 pt-4">
                            <Button type="submit">{t('common.actions.change')}</Button>
                        </CardFooter>
                    </form>

                    <Separator />

                    {/* Connected Accounts */}
                    <div className="space-y-4">
                        <h3 className="text-lg font-medium">{t('accountSettings.connectedAccountsTitle')}</h3>
                        {loading ? (
                            <p className="text-muted-foreground">{t('common.status.loading')}</p>
                        ) : (
                            <div className="space-y-4">
                                {connectedAccounts.length > 0 ? (
                                    connectedAccounts.map((account) => (
                                        <div key={account.provider_name} className="flex items-center justify-between p-3 border rounded-md">
                                            <div className="flex items-center gap-4">
                                                <ProviderIcon providerName={account.provider_name} />
                                                <div>
                                                    <p className="font-semibold capitalize">{account.provider_name}</p>
                                                    <p className="text-sm text-muted-foreground">{t('accountSettings.linkedAs', { id: account.provider_user_id })}</p>
                                                </div>
                                            </div>
                                            <Button variant="destructive" size="sm" onClick={() => handleUnlinkClick(account.provider_name)}>
                                                {t('common.actions.unlink')}
                                            </Button>
                                        </div>
                                    ))
                                ) : (
                                    <p className="text-sm text-muted-foreground">{t('accountSettings.noConnectedAccounts')}</p>
                                )}
                                {unlinkedProviders.length > 0 && (
                                    <div className="pt-4">
                                        <h4 className="text-md font-medium">{t('accountSettings.linkNewAccountTitle')}</h4>
                                        <div className="mt-2 space-y-2">
                                            {unlinkedProviders.map((provider) => (
                                                <div key={provider.name} className="flex items-center justify-between p-3 border rounded-md">
                                                    <div className="flex items-center gap-4">
                                                        <ProviderIcon providerName={provider.name} />
                                                        <p className="font-semibold capitalize">{provider.name}</p>
                                                    </div>
                                                    <Button variant="outline" size="sm" onClick={() => handleLinkAccount(provider.name)}>
                                                        {t('common.actions.link')}
                                                    </Button>
                                                </div>
                                            ))}
                                        </div>
                                    </div>
                                )}
                            </div>
                        )}
                    </div>
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>{t('accountSettings.preferences.title')}</CardTitle>
                    <CardDescription>{t('accountSettings.preferences.description')}</CardDescription>
                </CardHeader>
                <CardContent>
                    <div className="space-y-2">
                        <Label htmlFor="language-select">{t('accountSettings.preferences.languageLabel')}</Label>
                        <Select value={selectedLanguage} onValueChange={handleLanguageChange}>
                            <SelectTrigger id="language-select" className="w-[280px]">
                                <SelectValue placeholder={t('accountSettings.preferences.languageLabel')} />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="auto">{t('accountSettings.preferences.languageOptions.auto')}</SelectItem>
                                <SelectItem value="en">{t('accountSettings.preferences.languageOptions.en')}</SelectItem>
                                <SelectItem value="zh-CN">{t('accountSettings.preferences.languageOptions.zh-CN')}</SelectItem>
                            </SelectContent>
                        </Select>
                    </div>
                </CardContent>
            </Card>
        </div>
    );
};

export default AccountSettingsPage;