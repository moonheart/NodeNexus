export interface Theme {
  id: string;
  name: string;
  css?: string; // Used for custom themes injected via a <style> tag
}

export const defaultTheme: Theme = {
  id: 'default',
  name: 'Default',
};

export const builtInThemes: Theme[] = [defaultTheme];