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

// Updated types to match the new system
export interface UserThemeSettings {
  theme_mode: 'light' | 'dark' | 'system';
  active_theme_id: string | null;
}

const ThemeSettingsPage = () => {
  const [userThemes, setUserThemes] = useState<Theme[]>([]);
  const [settings, setSettings] = useState<UserThemeSettings | null>(null);
  const [themeToDelete, setThemeToDelete] = useState<Theme | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingTheme, setEditingTheme] = useState<Theme | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { token } = useAuthStore();
  const { reloadTheme, activeThemeId, setActiveThemeId, themeMode, setThemeMode } = useTheme();

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
          throw new Error('Failed to fetch theme data.');
        }

        const themesData: Theme[] = await themesResponse.json();
        const settingsData: UserThemeSettings = await settingsResponse.json();

        setUserThemes(themesData);
        setSettings({
          ...settingsData,
          active_theme_id: settingsData.active_theme_id || 'default',
        });
      } catch (err: unknown) {
        setError(err instanceof Error ? err.message : "An unknown error occurred.");
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [token]);

  const handleSaveSettings = async () => {
    if (!settings) return;
    try {
      await fetch('/api/user/theme-settings', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` },
        body: JSON.stringify(settings),
      });
      // Apply changes immediately via context
      setThemeMode(settings.theme_mode);
      setActiveThemeId(settings.active_theme_id || 'default');
      alert('Settings saved successfully!');
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save settings.");
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
        throw new Error(errorData.message || `Failed to save theme.`);
      }

      const savedTheme: Theme = await response.json();
      
      if (isUpdating) {
        setUserThemes(userThemes.map(t => t.id === savedTheme.id ? savedTheme : t));
      } else {
        setUserThemes([...userThemes, savedTheme]);
      }

      setIsModalOpen(false);
      setEditingTheme(null);
      alert(`Theme ${isUpdating ? 'updated' : 'created'} successfully!`);
      reloadTheme();
    } catch (err) {
      setError(err instanceof Error ? err.message : `An unknown error occurred.`);
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
      alert('Theme deleted successfully!');
      // If the deleted theme was active, switch to default
      if (activeThemeId === themeToDelete.id) {
        setActiveThemeId('default');
      }
      reloadTheme();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete theme.");
    }
  };

  if (loading) return <div>Loading theme settings...</div>;
  if (error) return <div className="text-destructive">Error: {error}</div>;

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader><CardTitle>Appearance Settings</CardTitle></CardHeader>
        <CardContent>
          {settings && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">Appearance Mode</label>
                <Select
                  value={themeMode}
                  onValueChange={(value) => setThemeMode(value as UserThemeSettings['theme_mode'])}
                >
                  <SelectTrigger className="w-[180px]"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="light">Light</SelectItem>
                    <SelectItem value="dark">Dark</SelectItem>
                    <SelectItem value="system">System</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">Theme</label>
                <Select
                  value={activeThemeId}
                  onValueChange={(value) => setActiveThemeId(value)}
                >
                  <SelectTrigger className="w-[180px]"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    {allThemes.map(theme => (
                      <SelectItem key={theme.id} value={theme.id}>{theme.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <Button onClick={handleSaveSettings}>Save Settings</Button>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Theme Library</CardTitle>
          <Button onClick={() => { setEditingTheme(null); setIsModalOpen(true); }}>Create New Theme</Button>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {allThemes.map((theme) => (
                <TableRow key={theme.id}>
                  <TableCell>{theme.name}</TableCell>
                  <TableCell className="text-right">
                    <Button variant="ghost" size="sm" onClick={() => { setEditingTheme(theme); setIsModalOpen(true); }}>Edit</Button>
                    {theme.id !== 'default' && (
                      <Button variant="ghost" size="sm" className="text-destructive" onClick={() => setThemeToDelete(theme)}>Delete</Button>
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
            <AlertDialogTitle>Are you sure?</AlertDialogTitle>
            <AlertDialogDescription>
              This action cannot be undone. This will permanently delete the theme "{themeToDelete?.name}".
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={handleDeleteTheme}>Continue</AlertDialogAction>
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