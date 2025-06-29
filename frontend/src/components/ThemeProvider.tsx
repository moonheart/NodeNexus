import { createContext, useContext, useEffect, useState, useCallback } from "react";
import { useAuthStore } from "@/store/authStore";
import type { UserThemeSettings, Theme, ThemeConfig } from "@/pages/ThemeSettingsPage";
import { defaultLightTheme, defaultDarkTheme } from "@/lib/themes";

// --- Helper Functions ---

const applyThemeToDOM = (config: ThemeConfig, type: 'light' | 'dark') => {
  const styleId = 'dynamic-theme-styles';
  let styleTag = document.getElementById(styleId) as HTMLStyleElement | null;

  if (!styleTag) {
    styleTag = document.createElement('style');
    styleTag.id = styleId;
    document.head.appendChild(styleTag);
  }

  const cssVariables = Object.entries(config)
    .map(([key, value]) => `  ${key}: ${value};`)
    .join('\n');

  styleTag.innerHTML = `
:root {
${cssVariables}
}
  `;

  const root = document.documentElement;
  root.classList.remove('light', 'dark');
  root.classList.add(type);
};

// --- Types and Context ---

export type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeProviderState {
  themeType: 'light' | 'dark';
  themeMode: ThemeMode;
  setThemeMode: (mode: ThemeMode) => void;
  reloadTheme: () => void;
}

const ThemeProviderContext = createContext<ThemeProviderState | undefined>(undefined);

const GUEST_THEME_STORAGE_KEY = 'vite-ui-theme-mode';

// --- Provider Component ---

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { token } = useAuthStore();
  const [themeType, setThemeType] = useState<'light' | 'dark'>('light');
  const [themeMode, setThemeModeState] = useState<ThemeMode>(
    () => (localStorage.getItem(GUEST_THEME_STORAGE_KEY) as ThemeMode) || 'system'
  );
  const [triggerReload, setTriggerReload] = useState(0);

  const reloadTheme = useCallback(() => {
    setTriggerReload(v => v + 1);
  }, []);

  useEffect(() => {
    let isMounted = true;

    const handleSystemThemeChange = () => {
      // Re-run the entire effect to determine the correct theme
      if (isMounted) {
        reloadTheme();
      }
    };

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', handleSystemThemeChange);

    // Unauthenticated User Logic
    if (!token) {
      const systemIsDark = mediaQuery.matches;
      if (themeMode === 'system') {
        applyThemeToDOM(systemIsDark ? defaultDarkTheme.config : defaultLightTheme.config, systemIsDark ? 'dark' : 'light');
        if (isMounted) setThemeType(systemIsDark ? 'dark' : 'light');
      } else {
        applyThemeToDOM(themeMode === 'dark' ? defaultDarkTheme.config : defaultLightTheme.config, themeMode);
        if (isMounted) setThemeType(themeMode);
      }
    }
    // Authenticated User Logic
    else {
      const fetchAndApplyUserTheme = async () => {
        try {
          const headers = { 'Authorization': `Bearer ${token}` };
          const settingsRes = await fetch('/api/user/theme-settings', { headers });
          if (!settingsRes.ok) throw new Error('Failed to fetch user theme settings.');
          const settings: UserThemeSettings = await settingsRes.json();
          
          if (!isMounted) return;
          setThemeModeState(settings.theme_mode);

          const systemIsDark = mediaQuery.matches;
          const currentMode = settings.theme_mode === 'system' ? (systemIsDark ? 'dark' : 'light') : settings.theme_mode;
          const activeThemeId = currentMode === 'dark' ? settings.active_dark_theme_id : settings.active_light_theme_id;
          
          setThemeType(currentMode);

          if (activeThemeId) {
            const themeRes = await fetch(`/api/themes/${activeThemeId}`, { headers });
            if (!themeRes.ok) throw new Error(`Failed to fetch theme: ${activeThemeId}`);
            const theme: Theme = await themeRes.json();
            if (isMounted) applyThemeToDOM(theme.config, theme.type);
          } else {
            // Fallback to built-in themes if no custom theme is selected
            applyThemeToDOM(currentMode === 'dark' ? defaultDarkTheme.config : defaultLightTheme.config, currentMode);
          }
        } catch (error) {
          console.error("Error applying user theme, falling back to default:", error);
          // Fallback on error
          const systemIsDark = mediaQuery.matches;
          applyThemeToDOM(systemIsDark ? defaultDarkTheme.config : defaultLightTheme.config, systemIsDark ? 'dark' : 'light');
        }
      };
      fetchAndApplyUserTheme();
    }

    return () => {
      isMounted = false;
      mediaQuery.removeEventListener('change', handleSystemThemeChange);
    };
  }, [token, themeMode, triggerReload]);

  const setThemeMode = useCallback(async (mode: ThemeMode) => {
    if (!token) {
      localStorage.setItem(GUEST_THEME_STORAGE_KEY, mode);
      setThemeModeState(mode);
      return;
    }

    // For logged-in users, update backend
    try {
      const headers = { 'Authorization': `Bearer ${token}` };
      const settingsRes = await fetch('/api/user/theme-settings', { headers });
      if (!settingsRes.ok) throw new Error('Failed to fetch settings before update.');
      
      const currentSettings = await settingsRes.json();
      const payload = { ...currentSettings, theme_mode: mode };

      const updateRes = await fetch('/api/user/theme-settings', {
        method: 'PUT',
        headers: { ...headers, 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });

      if (!updateRes.ok) throw new Error('Failed to save theme mode.');
      
      setThemeModeState(mode); // Update state after successful API call
      reloadTheme(); // Trigger effect to apply new theme
    } catch (error) {
      console.error("Failed to set theme mode:", error);
    }
  }, [token, reloadTheme]);

  const value = { themeType, themeMode, setThemeMode, reloadTheme };

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