import { createContext, useContext, useEffect, useState, useCallback } from "react";
import { useAuthStore } from "@/store/authStore";
import type { UserThemeSettings } from "@/pages/ThemeSettingsPage";
import type { Theme } from "@/lib/themes";

// --- Constants ---
const DYNAMIC_STYLE_ID = 'dynamic-theme-styles';
const GUEST_THEME_STORAGE_KEY = 'vite-ui-theme-mode';

// --- Helper Functions ---
const getSystemTheme = () => (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');

const updateThemeClass = (mode: 'light' | 'dark') => {
  const root = document.documentElement;
  root.classList.remove('light', 'dark');
  root.classList.add(mode);
};

const applyCustomTheme = (css: string) => {
  let styleTag = document.getElementById(DYNAMIC_STYLE_ID) as HTMLStyleElement | null;
  if (!styleTag) {
    styleTag = document.createElement('style');
    styleTag.id = DYNAMIC_STYLE_ID;
    document.head.appendChild(styleTag);
  }
  styleTag.innerHTML = css;
};

const clearCustomTheme = () => {
  const styleTag = document.getElementById(DYNAMIC_STYLE_ID);
  if (styleTag) {
    styleTag.innerHTML = '';
  }
};

// --- Types and Context ---
export type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeProviderState {
  themeMode: ThemeMode;
  setThemeMode: (mode: ThemeMode) => void;
  activeThemeId: string;
  setActiveThemeId: (themeId: string) => void;
  reloadTheme: () => void;
}

const ThemeProviderContext = createContext<ThemeProviderState | undefined>(undefined);

// --- Provider Component ---
export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { token } = useAuthStore();
  const [themeMode, setThemeModeState] = useState<ThemeMode>(
    () => (localStorage.getItem(GUEST_THEME_STORAGE_KEY) as ThemeMode) || 'system'
  );
  const [activeThemeId, setActiveThemeIdState] = useState<string>('default');
  const [triggerReload, setTriggerReload] = useState(0);

  const reloadTheme = useCallback(() => setTriggerReload(v => v + 1), []);

  // Effect to handle theme application
  useEffect(() => {
    let isMounted = true;

    const handleSystemThemeChange = () => {
      if (themeMode === 'system') {
        updateThemeClass(getSystemTheme());
      }
    };

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', handleSystemThemeChange);

    // Determine and apply light/dark mode
    const currentMode = themeMode === 'system' ? getSystemTheme() : themeMode;
    updateThemeClass(currentMode);

    // Fetch and apply user's selected theme if logged in
    const applyUserTheme = async () => {
      if (!token) {
        setActiveThemeIdState('default');
        clearCustomTheme();
        return;
      }

      try {
        const headers = { 'Authorization': `Bearer ${token}` };
        const settingsRes = await fetch('/api/user/theme-settings', { headers });
        if (!settingsRes.ok) throw new Error('Failed to fetch user theme settings.');
        
        const settings: UserThemeSettings = await settingsRes.json();
        if (!isMounted) return;

        const themeIdToApply = settings.active_theme_id || 'default';
        setActiveThemeIdState(themeIdToApply);

        if (themeIdToApply === 'default') {
          clearCustomTheme();
        } else {
          const themeRes = await fetch(`/api/themes/${themeIdToApply}`, { headers });
          if (!themeRes.ok) throw new Error(`Failed to fetch theme: ${themeIdToApply}`);
          const theme: Theme = await themeRes.json();
          if (isMounted && theme.css) {
            applyCustomTheme(theme.css);
          }
        }
      } catch (error) {
        console.error("Error applying user theme, falling back to default:", error);
        if (isMounted) {
          setActiveThemeIdState('default');
          clearCustomTheme();
        }
      }
    };

    applyUserTheme();

    return () => {
      isMounted = false;
      mediaQuery.removeEventListener('change', handleSystemThemeChange);
    };
  }, [token, themeMode, triggerReload]);

  const setThemeMode = useCallback(async (mode: ThemeMode) => {
    setThemeModeState(mode);
    if (!token) {
      localStorage.setItem(GUEST_THEME_STORAGE_KEY, mode);
    } else {
      try {
        await fetch('/api/user/theme-settings', {
          method: 'PUT',
          headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
          body: JSON.stringify({ theme_mode: mode }),
        });
      } catch (error) {
        console.error("Failed to save theme mode:", error);
      }
    }
  }, [token]);

  const setActiveThemeId = useCallback(async (themeId: string) => {
    setActiveThemeIdState(themeId);
    if (token) {
      try {
        await fetch('/api/user/theme-settings', {
          method: 'PUT',
          headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
          body: JSON.stringify({ active_theme_id: themeId }),
        });
        reloadTheme(); // Reload to apply the new theme from backend
      } catch (error) {
        console.error("Failed to save active theme:", error);
      }
    }
  }, [token, reloadTheme]);

  const value = { themeMode, setThemeMode, activeThemeId, setActiveThemeId, reloadTheme };

  return (
    <ThemeProviderContext.Provider value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}

export const useTheme = () => {
  const context = useContext(ThemeProviderContext);
  if (context === undefined) {
    throw new Error("useTheme must be used within a ThemeProvider");
  }
  return context;
};