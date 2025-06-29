import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";
import { useAuthStore } from "@/store/authStore";
import ThemeEditorModal from "@/components/ThemeEditorModal";
import { useTheme } from "@/components/ThemeProvider";
import { builtInThemes } from "@/lib/themes";

// Define types based on backend entities
export interface ThemeConfig {
  [key: string]: string; // e.g., '--background': 'oklch(1 0 0)'
}

export interface Theme {
  id: string;
  name: string;
  type: 'light' | 'dark';
  config: ThemeConfig;
  is_official: boolean;
}

export interface UserThemeSettings {
  theme_mode: 'light' | 'dark' | 'system';
  active_light_theme_id: string | null;
  active_dark_theme_id: string | null;
}

const ThemeSettingsPage = () => {
  const [themes, setThemes] = useState<Theme[]>([]);
  const [settings, setSettings] = useState<UserThemeSettings | null>(null);
  const [themeToDelete, setThemeToDelete] = useState<Theme | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingTheme, setEditingTheme] = useState<Theme | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { token } = useAuthStore();
  const { reloadTheme } = useTheme();

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

        const themesData = await themesResponse.json();
        const settingsData = await settingsResponse.json();

        // Combine built-in themes with user-fetched themes
        const allThemes = [...builtInThemes, ...themesData];
        setThemes(allThemes);
        // Ensure settings have default fallbacks for active themes
        const processedSettings = {
          ...settingsData,
          active_light_theme_id: settingsData.active_light_theme_id || 'builtin-light',
          active_dark_theme_id: settingsData.active_dark_theme_id || 'builtin-dark',
        };
        setSettings(processedSettings);
      } catch (err: unknown) {
        if (err instanceof Error) {
          setError(err.message);
        } else {
          setError("An unknown error occurred.");
        }
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [token]);

  if (loading) {
    return <div>Loading theme settings...</div>;
  }

  if (error) {
    return <div className="text-red-500">Error: {error}</div>;
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Appearance Settings</CardTitle>
        </CardHeader>
        <CardContent>
          {settings && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <label htmlFor="theme-mode" className="text-sm font-medium">Theme Mode</label>
                <Select
                  value={settings.theme_mode}
                  onValueChange={(value) => setSettings({ ...settings, theme_mode: value as UserThemeSettings['theme_mode'] })}
                >
                  <SelectTrigger id="theme-mode" className="w-[180px]">
                    <SelectValue placeholder="Select mode" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="light">Light</SelectItem>
                    <SelectItem value="dark">Dark</SelectItem>
                    <SelectItem value="system">System</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="flex items-center justify-between">
                <label htmlFor="light-theme" className="text-sm font-medium">Light Theme</label>
                <Select
                  value={settings.active_light_theme_id || ''}
                  onValueChange={(value) => setSettings({ ...settings, active_light_theme_id: value })}
                >
                  <SelectTrigger id="light-theme" className="w-[180px]">
                    <SelectValue placeholder="Select light theme" />
                  </SelectTrigger>
                  <SelectContent>
                    {themes.filter(t => t.type === 'light').map(theme => (
                      <SelectItem key={theme.id} value={theme.id}>{theme.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="flex items-center justify-between">
                <label htmlFor="dark-theme" className="text-sm font-medium">Dark Theme</label>
                <Select
                  value={settings.active_dark_theme_id || ''}
                  onValueChange={(value) => setSettings({ ...settings, active_dark_theme_id: value })}
                >
                  <SelectTrigger id="dark-theme" className="w-[180px]">
                    <SelectValue placeholder="Select dark theme" />
                  </SelectTrigger>
                  <SelectContent>
                    {themes.filter(t => t.type === 'dark').map(theme => (
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
                <TableHead>Type</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {themes.map((theme) => (
                <TableRow key={theme.id}>
                  <TableCell>{theme.name}</TableCell>
                  <TableCell>{theme.type}</TableCell>
                  <TableCell className="text-right">
                    <Button variant="ghost" size="sm" onClick={() => { setEditingTheme(theme); setIsModalOpen(true); }}>Edit</Button>
                    {!theme.is_official && (
                      <Button variant="ghost" size="sm" className="text-red-500" onClick={() => setThemeToDelete(theme)}>Delete</Button>
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
        isOfficial={editingTheme?.is_official}
      />
    </div>
  );

  async function handleSaveTheme(themeToSave: Partial<Theme> & { id?: string }) {
    const isUpdating = !!themeToSave.id;
    const url = isUpdating ? `/api/themes/${themeToSave.id}` : '/api/themes';
    const method = isUpdating ? 'PUT' : 'POST';

    try {
      const response = await fetch(url, {
        method,
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify(themeToSave),
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || `Failed to ${isUpdating ? 'update' : 'create'} theme.`);
      }

      const savedTheme: Theme = await response.json();

      if (isUpdating) {
        setThemes(themes.map(t => t.id === savedTheme.id ? savedTheme : t));
      } else {
        setThemes([...themes, savedTheme]);
      }

      setIsModalOpen(false);
      setEditingTheme(null);
      alert(`Theme ${isUpdating ? 'updated' : 'created'} successfully!`);
      reloadTheme(); // Reload theme to apply changes if it's active

    } catch (err) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError(`An unknown error occurred while saving the theme.`);
      }
    }
  }

  async function handleDeleteTheme() {
    if (!themeToDelete) return;
    try {
      const response = await fetch(`/api/themes/${themeToDelete.id}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${token}` },
      });
      if (!response.ok) {
        throw new Error('Failed to delete theme.');
      }
      setThemes(themes.filter(t => t.id !== themeToDelete.id));
      setThemeToDelete(null);
      // Optionally, show a success toast
      alert('Theme deleted successfully!');
      reloadTheme(); // Reload theme in case the active one was deleted
    } catch (err) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("An unknown error occurred while deleting the theme.");
      }
    }
  }

  async function handleSaveSettings() {
    if (!settings) return;
    try {
      const response = await fetch('/api/user/theme-settings', {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify(settings),
      });
      if (!response.ok) {
        throw new Error('Failed to save settings.');
      }
      // Optionally, show a success toast
      alert('Settings saved successfully!');
      reloadTheme(); // Reload theme to apply new settings
    } catch (err) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("An unknown error occurred while saving settings.");
      }
    }
  }
};

export default ThemeSettingsPage;