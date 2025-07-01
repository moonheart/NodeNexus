import { createContext, useContext, useEffect, useState, useCallback } from "react";
import { useAuthStore } from "@/store/authStore";
import type { UserThemeSettings } from "@/pages/ThemeSettingsPage";
import type { Theme } from "@/lib/themes";

// --- Constants ---
const DYNAMIC_STYLE_ID = 'dynamic-theme-styles';
const GUEST_THEME_STORAGE_KEY = 'vite-ui-theme-mode';
const ACTIVE_THEME_CSS_STORAGE_KEY = 'active-theme-css';

// --- Helper Functions ---
const getSystemTheme = () => (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');

const updateThemeClass = (mode: 'light' | 'dark') => {
  const root = document.documentElement;
  root.classList.remove('light', 'dark');
  root.classList.add(mode);
};

const applyCustomTheme = (css: string) => {
  // Phase 2: Transition from preloaded inline styles to a style tag.
  // 1. Clear any inline styles set by the flicker-prevention script.
  if (document.documentElement.dataset.preloadedTheme) {
    const style = document.documentElement.style;
    for (let i = style.length - 1; i >= 0; i--) {
      const propName = style[i];
      if (propName.startsWith('--')) {
        style.removeProperty(propName);
      }
    }
    delete document.documentElement.dataset.preloadedTheme;
  }

  // 2. Use the robust <style> tag method for the running app.
  let styleTag = document.getElementById(DYNAMIC_STYLE_ID) as HTMLStyleElement | null;
  if (!styleTag) {
    styleTag = document.createElement('style');
    styleTag.id = DYNAMIC_STYLE_ID;
  }
  
  styleTag.innerHTML = css;
  // Append to ensure it's last and has precedence over other stylesheets.
  document.head.appendChild(styleTag);

  // 3. Save to localStorage for the next initial load.
  try {
    localStorage.setItem(ACTIVE_THEME_CSS_STORAGE_KEY, css);
  } catch (e) {
    console.error("Failed to save theme CSS to localStorage", e);
  }
};

const clearCustomTheme = () => {
  // Clear both potential theme application methods.
  // 1. Clear inline styles.
  if (document.documentElement.dataset.preloadedTheme) {
    const style = document.documentElement.style;
    for (let i = style.length - 1; i >= 0; i--) {
      const propName = style[i];
      if (propName.startsWith('--')) {
        style.removeProperty(propName);
      }
    }
    delete document.documentElement.dataset.preloadedTheme;
  }

  // 2. Clear style tag.
  const styleTag = document.getElementById(DYNAMIC_STYLE_ID);
  if (styleTag) {
    styleTag.innerHTML = '';
  }

  // 3. Clear from storage.
  try {
    localStorage.removeItem(ACTIVE_THEME_CSS_STORAGE_KEY);
  } catch (e) {
    console.error("Failed to remove theme CSS from localStorage", e);
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
  resolvedTheme: 'light' | 'dark';
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
  const [resolvedTheme, setResolvedTheme] = useState<'light' | 'dark'>(getSystemTheme());

  const reloadTheme = useCallback(() => setTriggerReload(v => v + 1), []);

  // Effect to handle theme application
  useEffect(() => {
    let isMounted = true;

    const handleSystemThemeChange = () => {
      if (themeMode === 'system') {
        const newResolvedTheme = getSystemTheme();
        updateThemeClass(newResolvedTheme);
        setResolvedTheme(newResolvedTheme);
      }
    };

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', handleSystemThemeChange);

    // Set initial resolved theme based on the class set by the inline script
    const initialResolvedTheme = document.documentElement.classList.contains('dark') ? 'dark' : 'light';
    setResolvedTheme(initialResolvedTheme);

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

        // Sync with backend state on load
        if (settings.theme_mode && settings.theme_mode !== themeMode) {
          setThemeModeState(settings.theme_mode);
          // Also update localStorage to be in sync
          localStorage.setItem(GUEST_THEME_STORAGE_KEY, settings.theme_mode);
        }

        const themeIdToApply = settings.active_theme_id || 'default';
        setActiveThemeIdState(themeIdToApply);

        if (themeIdToApply === 'default') {
          clearCustomTheme();
        } else {
          const themeRes = await fetch(`/api/themes/${themeIdToApply}`, { headers });
          if (!themeRes.ok) throw new Error(`Failed to fetch theme: ${themeIdToApply}`);
          const theme: Theme = await themeRes.json();
          if (isMounted && theme.css) {
            applyCustomTheme(theme.css); // This will also save it to localStorage
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
  }, [token, triggerReload, themeMode]);

  const setThemeMode = useCallback(async (mode: ThemeMode) => {
    setThemeModeState(mode);
    localStorage.setItem(GUEST_THEME_STORAGE_KEY, mode);

    // Update class immediately for responsiveness
    const newResolvedTheme = mode === 'system' ? getSystemTheme() : mode;
    updateThemeClass(newResolvedTheme);
    setResolvedTheme(newResolvedTheme);

    if (token) {
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

  const value = { themeMode, setThemeMode, activeThemeId, setActiveThemeId, reloadTheme, resolvedTheme };

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
