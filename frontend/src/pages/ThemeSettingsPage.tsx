import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";
import { useAuthStore } from "@/store/authStore";
import ThemeEditorModal from "@/components/ThemeEditorModal";
import { useTheme } from "@/components/ThemeProvider";
import { builtInThemes, type Theme } from "@/lib/themes";
import { useTranslation } from "react-i18next";

// Updated types to match the new system
export interface UserThemeSettings {
  theme_mode: 'light' | 'dark' | 'system';
  active_theme_id: string | null;
}

const ThemeSettingsPage = () => {
  const { t } = useTranslation();
  const [userThemes, setUserThemes] = useState<Theme[]>([]);
  const [settings, setSettings] = useState<UserThemeSettings | null>(null);
  const [themeToDelete, setThemeToDelete] = useState<Theme | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingTheme, setEditingTheme] = useState<Theme | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { token } = useAuthStore();
  const { reloadTheme, activeThemeId, setActiveThemeId } = useTheme();

  const allThemes = [...builtInThemes, ...userThemes];

  useEffect(() => {
    const fetchData = async () => {
      setLoading(true);
      setError(null);
      try {
        const headers = { 'Authorization': `Bearer ${token}` };
        
        const [themesResponse, settingsResponse] = await Promise.all([
          fetch('/api/themes', { headers }),
          fetch('/api/user/theme-settings', { headers }),
        ]);

        if (!themesResponse.ok || !settingsResponse.ok) {
          throw new Error(t('common.notifications.fetchFailed'));
        }

        const themesData: Theme[] = await themesResponse.json();
        const settingsData: UserThemeSettings = await settingsResponse.json();

        setUserThemes(themesData);
        setSettings({
          ...settingsData,
          active_theme_id: settingsData.active_theme_id || 'default',
        });
      } catch (err: unknown) {
        setError(err instanceof Error ? err.message : t('common.errors.unknown'));
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [token, t]);

  const handleSaveSettings = async () => {
    if (!settings) return;
    try {
      await fetch('/api/user/theme-settings', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` },
        body: JSON.stringify(settings),
      });
      // The theme provider will fetch the latest settings and apply them
      reloadTheme();
      alert(t('common.notifications.saved'));
    } catch (err) {
      setError(err instanceof Error ? err.message : t('common.notifications.saveFailed'));
    }
  };

  const handleSaveTheme = async (themeToSave: Partial<Theme>) => {
    const isUpdating = !!themeToSave.id;
    const url = isUpdating ? `/api/themes/${themeToSave.id}` : '/api/themes';
    const method = isUpdating ? 'PUT' : 'POST';

    try {
      const response = await fetch(url, {
        method,
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` },
        body: JSON.stringify(themeToSave),
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || t('common.notifications.saveFailed'));
      }

      const savedTheme: Theme = await response.json();
      
      if (isUpdating) {
        setUserThemes(userThemes.map(t => t.id === savedTheme.id ? savedTheme : t));
      } else {
        setUserThemes([...userThemes, savedTheme]);
      }

      setIsModalOpen(false);
      setEditingTheme(null);
      alert(t('themeSettings.notifications.themeSaved', { status: isUpdating ? t('themeSettings.status.updated') : t('themeSettings.status.created') }));
      reloadTheme();
    } catch (err) {
      setError(err instanceof Error ? err.message : t('common.errors.unknown'));
    }
  };

  const handleDeleteTheme = async () => {
    if (!themeToDelete) return;
    try {
      await fetch(`/api/themes/${themeToDelete.id}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${token}` },
      });
      setUserThemes(userThemes.filter(t => t.id !== themeToDelete.id));
      setThemeToDelete(null);
      alert(t('common.notifications.deleted'));
      // If the deleted theme was active, switch to default
      if (activeThemeId === themeToDelete.id) {
        setActiveThemeId('default');
      }
      reloadTheme();
    } catch (err) {
      setError(err instanceof Error ? err.message : t('common.notifications.deleteFailed'));
    }
  };

  if (loading) return <div>{t('common.status.loading')}</div>;
  if (error) return <div className="text-destructive">{t('common.notifications.error', { error: error })}</div>;

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader><CardTitle>{t('themeSettings.title')}</CardTitle></CardHeader>
        <CardContent>
          {settings && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">{t('themeSettings.mode')}</label>
                <Select
                  value={settings.theme_mode}
                  onValueChange={(value) => {
                    setSettings(s => s ? { ...s, theme_mode: value as UserThemeSettings['theme_mode'] } : null);
                  }}
                >
                  <SelectTrigger className="w-[180px]"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="light">{t('themeSettings.modes.light')}</SelectItem>
                    <SelectItem value="dark">{t('themeSettings.modes.dark')}</SelectItem>
                    <SelectItem value="system">{t('themeSettings.modes.system')}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">{t('themeSettings.theme')}</label>
                <Select
                  value={settings.active_theme_id || ''}
                  onValueChange={(value) => {
                    setSettings(s => s ? { ...s, active_theme_id: value } : null);
                  }}
                >
                  <SelectTrigger className="w-[180px]"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    {allThemes.map(theme => (
                      <SelectItem key={theme.id} value={theme.id}>{theme.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <Button onClick={handleSaveSettings}>{t('common.actions.save')}</Button>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>{t('themeSettings.libraryTitle')}</CardTitle>
          <Button onClick={() => { setEditingTheme(null); setIsModalOpen(true); }}>{t('common.actions.create')}</Button>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t('common.table.name')}</TableHead>
                <TableHead className="text-right">{t('common.table.actions')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {allThemes.map((theme) => (
                <TableRow key={theme.id}>
                  <TableCell>{theme.name}</TableCell>
                  <TableCell className="text-right">
                    <Button variant="ghost" size="sm" onClick={() => { setEditingTheme(theme); setIsModalOpen(true); }}>{t('common.actions.edit')}</Button>
                    {theme.id !== 'default' && (
                      <Button variant="ghost" size="sm" className="text-destructive" onClick={() => setThemeToDelete(theme)}>{t('common.actions.delete')}</Button>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <AlertDialog open={!!themeToDelete} onOpenChange={(open) => !open && setThemeToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('common.dialogs.delete.title')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('themeSettings.deleteDialog.description', { themeName: themeToDelete?.name })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.actions.cancel')}</AlertDialogCancel>
            <AlertDialogAction onClick={handleDeleteTheme}>{t('common.actions.continue')}</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <ThemeEditorModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        theme={editingTheme}
        onSave={handleSaveTheme}
      />
    </div>
  );
};

export default ThemeSettingsPage;