import React, { useState, useEffect, useCallback } from 'react';
import { useAuthStore } from '../store/authStore';
import toast from 'react-hot-toast';
import * as userService from '../services/userService';
import type { ConnectedAccount, OAuthProvider } from '../services/userService';

const AccountSettingsPage: React.FC = () => {
    const { user, setUser } = useAuthStore();
    const [username, setUsername] = useState(user?.username || '');
    
    const [currentPassword, setCurrentPassword] = useState('');
    const [newPassword, setNewPassword] = useState('');
    const [confirmPassword, setConfirmPassword] = useState('');

    const [connectedAccounts, setConnectedAccounts] = useState<ConnectedAccount[]>([]);
    const [availableProviders, setAvailableProviders] = useState<OAuthProvider[]>([]);
    const [loading, setLoading] = useState(true);

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
            toast.error(error instanceof Error ? error.message : 'Failed to fetch account data.');
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
        // Check for linking success message
        const urlParams = new URLSearchParams(window.location.search);
        if (urlParams.get('link_success') === 'true') {
            toast.success('Account linked successfully!');
            // Clean up the URL
            window.history.replaceState({}, document.title, window.location.pathname);
            fetchData(); // Refresh data
        }
    }, [fetchData]);

    const handleUpdateUsername = async (e: React.FormEvent) => {
        e.preventDefault();
        const toastId = toast.loading('Updating username...');
        try {
            const updatedUser = await userService.updateUsername(username);
            toast.success('Username updated successfully!', { id: toastId });
            if (user) {
                setUser({ ...user, username: updatedUser.username });
            }
        } catch (error) {
            toast.error(error instanceof Error ? error.message : 'Failed to update username.', { id: toastId });
        }
    };

    const handleChangePassword = async (e: React.FormEvent) => {
        e.preventDefault();
        if (newPassword !== confirmPassword) {
            toast.error("New passwords don't match!");
            return;
        }
        const toastId = toast.loading('Changing password...');
        try {
            await userService.updatePassword({ current_password: currentPassword, new_password: newPassword });
            toast.success('Password changed successfully!', { id: toastId });
            setCurrentPassword('');
            setNewPassword('');
            setConfirmPassword('');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : 'Failed to change password.', { id: toastId });
        }
    };

    const handleUnlinkAccount = async (providerName: string) => {
        if (!window.confirm(`Are you sure you want to unlink your ${providerName} account?`)) {
            return;
        }
        const toastId = toast.loading(`Unlinking ${providerName} account...`);
        try {
            await userService.unlinkProvider(providerName);
            toast.success(`${providerName} account unlinked successfully!`, { id: toastId });
            fetchData(); // Refresh the list
        } catch (error) {
            toast.error(error instanceof Error ? error.message : `Failed to unlink ${providerName} account.`, { id: toastId });
        }
    };

    const ProviderIcon = ({ providerName }: { providerName?: string }) => {
        if (!providerName) return null;

        const provider = availableProviders.find(p => p.name === providerName);

        if (provider && provider.iconUrl) {
            return <img src={provider.iconUrl} alt={`${provider.name} icon`} className="w-6 h-6" />;
        }

        return <div className="w-6 h-6 bg-gray-200 rounded-full" />; // Generic fallback icon
    };

    const handleLinkAccount = (providerName: string) => {
        window.location.href = `/api/auth/${providerName}/link`;
    };

    const unlinkedProviders = availableProviders.filter(
        (p) => p.name && !connectedAccounts.some((a) => a.provider_name === p.name)
    );

    return (
        <div className="space-y-8 max-w-4xl mx-auto px-4 py-8">
            <h1 className="text-3xl font-bold text-slate-900">账户设置</h1>

            {/* Account Information Card */}
            <div className="bg-white p-6 rounded-lg shadow-md">
                <h2 className="text-xl font-semibold mb-4">账户信息</h2>
                <form onSubmit={handleUpdateUsername} className="space-y-4">
                    <div>
                        <label htmlFor="username" className="block text-sm font-medium text-slate-700">
                            用户名
                        </label>
                        <div className="mt-1 flex rounded-md shadow-sm">
                            <input
                                type="text"
                                name="username"
                                id="username"
                                value={username}
                                onChange={(e) => setUsername(e.target.value)}
                                className="flex-1 block w-full min-w-0 rounded-none rounded-l-md border-gray-300 px-3 py-2 focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
                            />
                            <button
                                type="submit"
                                className="inline-flex items-center rounded-r-md border border-l-0 border-gray-300 bg-gray-50 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-100"
                            >
                                保存
                            </button>
                        </div>
                    </div>
                </form>
            </div>

            {/* Security Settings Card */}
            <div className="bg-white p-6 rounded-lg shadow-md">
                <h2 className="text-xl font-semibold mb-4">安全设置</h2>
                <div className="space-y-6">
                    {/* Change Password Form */}
                    <form onSubmit={handleChangePassword}>
                        <h3 className="text-lg font-medium">修改密码</h3>
                        <div className="mt-4 grid grid-cols-1 gap-y-6 sm:grid-cols-2 sm:gap-x-4">
                            <div>
                                <label htmlFor="current-password" className="block text-sm font-medium text-slate-700">
                                    当前密码
                                </label>
                                <input
                                    type="password"
                                    id="current-password"
                                    value={currentPassword}
                                    onChange={(e) => setCurrentPassword(e.target.value)}
                                    className="mt-1 block w-full px-3 py-2 bg-white border border-slate-300 rounded-md shadow-sm placeholder-slate-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                                />
                            </div>
                            <div />
                            <div>
                                <label htmlFor="new-password" className="block text-sm font-medium text-slate-700">
                                    新密码
                                </label>
                                <input
                                    type="password"
                                    id="new-password"
                                    value={newPassword}
                                    onChange={(e) => setNewPassword(e.target.value)}
                                    className="mt-1 block w-full px-3 py-2 bg-white border border-slate-300 rounded-md shadow-sm placeholder-slate-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                                />
                            </div>
                            <div>
                                <label htmlFor="confirm-password" className="block text-sm font-medium text-slate-700">
                                    确认新密码
                                </label>
                                <input
                                    type="password"
                                    id="confirm-password"
                                    value={confirmPassword}
                                    onChange={(e) => setConfirmPassword(e.target.value)}
                                    className="mt-1 block w-full px-3 py-2 bg-white border border-slate-300 rounded-md shadow-sm placeholder-slate-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                                />
                            </div>
                        </div>
                        <div className="mt-6">
                            <button
                                type="submit"
                                className="px-4 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
                            >
                                修改密码
                            </button>
                        </div>
                    </form>

                    <hr />

                    {/* Connected Accounts */}
                    <div>
                        <h3 className="text-lg font-medium">关联账户</h3>
                        <div className="mt-4 space-y-4">
                            {loading ? (
                                <p>正在加载关联账户...</p>
                            ) : connectedAccounts.length > 0 ? (
                                connectedAccounts.map((account) => (
                                    <div key={account.provider_name} className="flex items-center justify-between p-3 bg-slate-50 rounded-md">
                                        <div className="flex items-center gap-4">
                                            <ProviderIcon providerName={account.provider_name} />
                                            <div>
                                                <p className="font-semibold capitalize">{account.provider_name}</p>
                                                <p className="text-sm text-slate-500">已关联为 {account.provider_user_id}</p>
                                            </div>
                                        </div>
                                        <button
                                            onClick={() => handleUnlinkAccount(account.provider_name)}
                                            className="px-3 py-1 text-sm font-medium text-red-600 border border-red-300 rounded-md hover:bg-red-50"
                                        >
                                            解除关联
                                        </button>
                                    </div>
                                ))
                            ) : (
                                <p className="text-sm text-slate-500">没有已关联的第三方账户。</p>
                            )}
                            {unlinkedProviders.length > 0 && (
                                <div className="pt-4">
                                    <h4 className="text-md font-medium">关联新账户</h4>
                                    <div className="mt-2 space-y-2">
                                        {unlinkedProviders.map((provider) => (
                                            <div key={provider.name} className="flex items-center justify-between p-3 bg-slate-50 rounded-md">
                                                <div className="flex items-center gap-4">
                                                    <ProviderIcon providerName={provider.name} />
                                                    <p className="font-semibold capitalize">{provider.name}</p>
                                                </div>
                                                <button
                                                    onClick={() => handleLinkAccount(provider.name)}
                                                    className="px-3 py-1 text-sm font-medium text-indigo-600 border border-indigo-300 rounded-md hover:bg-indigo-50"
                                                >
                                                    关联
                                                </button>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default AccountSettingsPage;