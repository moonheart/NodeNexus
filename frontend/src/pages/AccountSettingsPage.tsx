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

const AccountSettingsPage: React.FC = () => {
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
            toast.error(error instanceof Error ? error.message : '无法获取账户数据。');
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        if (user) {
            setUsername(user.username);
            fetchData();
        }
    }, [user, fetchData]);

    useEffect(() => {
        const urlParams = new URLSearchParams(window.location.search);
        if (urlParams.get('link_success') === 'true') {
            toast.success('账户关联成功！');
            window.history.replaceState({}, document.title, window.location.pathname);
            fetchData();
        }
    }, [fetchData]);

    const handleUpdateUsername = async (e: React.FormEvent) => {
        e.preventDefault();
        const toastId = toast.loading('正在更新用户名...');
        try {
            const updatedUser = await userService.updateUsername(username);
            toast.success('用户名更新成功！', { id: toastId });
            if (user) {
                setUser({ ...user, username: updatedUser.username });
            }
        } catch (error) {
            toast.error(error instanceof Error ? error.message : '更新用户名失败。', { id: toastId });
        }
    };

    const handleChangePassword = async (e: React.FormEvent) => {
        e.preventDefault();
        if (newPassword !== confirmPassword) {
            toast.error("新密码不匹配！");
            return;
        }
        const toastId = toast.loading('正在修改密码...');
        try {
            await userService.updatePassword({ current_password: currentPassword, new_password: newPassword });
            toast.success('密码修改成功！', { id: toastId });
            setCurrentPassword('');
            setNewPassword('');
            setConfirmPassword('');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : '修改密码失败。', { id: toastId });
        }
    };

    const handleUnlinkClick = (providerName: string) => {
        setUnlinkingProvider(providerName);
        setIsAlertOpen(true);
    };

    const confirmUnlinkAccount = async () => {
        if (!unlinkingProvider) return;
        const toastId = toast.loading(`正在解除 ${unlinkingProvider} 账户关联...`);
        try {
            await userService.unlinkProvider(unlinkingProvider);
            toast.success(`${unlinkingProvider} 账户解除关联成功！`, { id: toastId });
            fetchData();
        } catch (error) {
            toast.error(error instanceof Error ? error.message : `解除 ${unlinkingProvider} 账户关联失败。`, { id: toastId });
        } finally {
            setIsAlertOpen(false);
            setUnlinkingProvider(null);
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
                        <AlertDialogTitle>确定要解除关联吗？</AlertDialogTitle>
                        <AlertDialogDescription>
                            此操作无法撤销。您可能需要重新进行身份验证才能再次关联。
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel onClick={() => setUnlinkingProvider(null)}>取消</AlertDialogCancel>
                        <AlertDialogAction onClick={confirmUnlinkAccount}>确定</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>

            <Card>
                <CardHeader>
                    <CardTitle>账户信息</CardTitle>
                    <CardDescription>管理您的公开个人资料信息。</CardDescription>
                </CardHeader>
                <CardContent>
                    <form onSubmit={handleUpdateUsername} className="space-y-4">
                        <div className="space-y-2">
                            <Label htmlFor="username">用户名</Label>
                            <div className="flex">
                                <Input
                                    id="username"
                                    value={username}
                                    onChange={(e) => setUsername(e.target.value)}
                                    className="rounded-r-none"
                                />
                                <Button type="submit" className="rounded-l-none">保存</Button>
                            </div>
                        </div>
                    </form>
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>安全设置</CardTitle>
                    <CardDescription>管理您的密码和账户安全设置。</CardDescription>
                </CardHeader>
                <CardContent className="space-y-8">
                    {/* Change Password Form */}
                    <form onSubmit={handleChangePassword} className="space-y-4">
                        <h3 className="text-lg font-medium">修改密码</h3>
                        <div className="space-y-4">
                            <div className="space-y-2">
                                <Label htmlFor="current-password">当前密码</Label>
                                <Input
                                    type="password"
                                    id="current-password"
                                    value={currentPassword}
                                    onChange={(e) => setCurrentPassword(e.target.value)}
                                />
                            </div>
                            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                <div className="space-y-2">
                                    <Label htmlFor="new-password">新密码</Label>
                                    <Input
                                        type="password"
                                        id="new-password"
                                        value={newPassword}
                                        onChange={(e) => setNewPassword(e.target.value)}
                                    />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="confirm-password">确认新密码</Label>
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
                            <Button type="submit">修改密码</Button>
                        </CardFooter>
                    </form>

                    <Separator />

                    {/* Connected Accounts */}
                    <div className="space-y-4">
                        <h3 className="text-lg font-medium">关联账户</h3>
                        {loading ? (
                            <p className="text-muted-foreground">正在加载关联账户...</p>
                        ) : (
                            <div className="space-y-4">
                                {connectedAccounts.length > 0 ? (
                                    connectedAccounts.map((account) => (
                                        <div key={account.provider_name} className="flex items-center justify-between p-3 border rounded-md">
                                            <div className="flex items-center gap-4">
                                                <ProviderIcon providerName={account.provider_name} />
                                                <div>
                                                    <p className="font-semibold capitalize">{account.provider_name}</p>
                                                    <p className="text-sm text-muted-foreground">已关联为 {account.provider_user_id}</p>
                                                </div>
                                            </div>
                                            <Button variant="destructive" size="sm" onClick={() => handleUnlinkClick(account.provider_name)}>
                                                解除关联
                                            </Button>
                                        </div>
                                    ))
                                ) : (
                                    <p className="text-sm text-muted-foreground">没有已关联的第三方账户。</p>
                                )}
                                {unlinkedProviders.length > 0 && (
                                    <div className="pt-4">
                                        <h4 className="text-md font-medium">关联新账户</h4>
                                        <div className="mt-2 space-y-2">
                                            {unlinkedProviders.map((provider) => (
                                                <div key={provider.name} className="flex items-center justify-between p-3 border rounded-md">
                                                    <div className="flex items-center gap-4">
                                                        <ProviderIcon providerName={provider.name} />
                                                        <p className="font-semibold capitalize">{provider.name}</p>
                                                    </div>
                                                    <Button variant="outline" size="sm" onClick={() => handleLinkAccount(provider.name)}>
                                                        关联
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
        </div>
    );
};

export default AccountSettingsPage;