import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useEffect, useState } from "react";
import { Editor } from '@monaco-editor/react';
import { useTheme } from "@/components/ThemeProvider";
import type { Theme } from "@/lib/themes";
import { Trans, useTranslation } from "react-i18next";

interface ThemeEditorModalProps {
  theme: Theme | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (data: Partial<Theme>) => void;
}

const NEW_THEME_PLACEHOLDER = `:root {
  /* Paste your light mode theme here */
  /* ... all your other variables */
  --success: oklch(0.65 0.2 150);
  --warning: oklch(0.8 0.2 90);
}

.dark {
  /* Paste your dark mode theme here */
  /* ... all your other variables */
  --success: oklch(0.7 0.2 150);
  --warning: oklch(0.85 0.2 90);
}
`;

const DEFAULT_THEME_CSS = `/*
  This is the base CSS for the default theme.
  You can copy this content to create a new theme.
*/
:root {
  --background: oklch(0.9821 0 0);
  --foreground: oklch(0.2435 0 0);
  --card: oklch(0.9911 0 0);
  --card-foreground: oklch(0.2435 0 0);
  --popover: oklch(0.9911 0 0);
  --popover-foreground: oklch(0.2435 0 0);
  --primary: oklch(0.4341 0.0392 41.9938);
  --primary-foreground: oklch(1.0000 0 0);
  --secondary: oklch(0.9200 0.0651 74.3695);
  --secondary-foreground: oklch(0.3499 0.0685 40.8288);
  --muted: oklch(0.9521 0 0);
  --muted-foreground: oklch(0.5032 0 0);
  --accent: oklch(0.9310 0 0);
  --accent-foreground: oklch(0.2435 0 0);
  --destructive: oklch(0.6271 0.1936 33.3390);
  --destructive-foreground: oklch(1.0000 0 0);
  --success: oklch(0.65 0.2 150);
  --warning: oklch(0.8 0.2 90);
  --border: oklch(0.8822 0 0);
  --input: oklch(0.8822 0 0);
  --ring: oklch(0.4341 0.0392 41.9938);
  --chart-1: oklch(0.4341 0.0392 41.9938);
  --chart-2: oklch(0.9200 0.0651 74.3695);
  --chart-3: oklch(0.9310 0 0);
  --chart-4: oklch(0.9367 0.0523 75.5009);
  --chart-5: oklch(0.4338 0.0437 41.6746);
  --sidebar: oklch(0.9881 0 0);
  --sidebar-foreground: oklch(0.2645 0 0);
  --sidebar-primary: oklch(0.3250 0 0);
  --sidebar-primary-foreground: oklch(0.9881 0 0);
  --sidebar-accent: oklch(0.9761 0 0);
  --sidebar-accent-foreground: oklch(0.3250 0 0);
  --sidebar-border: oklch(0.9401 0 0);
  --sidebar-ring: oklch(0.7731 0 0);
  --font-sans: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, 'Noto Sans', sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  --font-serif: ui-serif, Georgia, Cambria, "Times New Roman", Times, serif;
  --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  --radius: 0.5rem;
  --shadow-2xs: 0 1px 3px 0px hsl(0 0% 0% / 0.05);
  --shadow-xs: 0 1px 3px 0px hsl(0 0% 0% / 0.05);
  --shadow-sm: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 1px 2px -1px hsl(0 0% 0% / 0.10);
  --shadow: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 1px 2px -1px hsl(0 0% 0% / 0.10);
  --shadow-md: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 2px 4px -1px hsl(0 0% 0% / 0.10);
  --shadow-lg: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 4px 6px -1px hsl(0 0% 0% / 0.10);
  --shadow-xl: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 8px 10px -1px hsl(0 0% 0% / 0.10);
  --shadow-2xl: 0 1px 3px 0px hsl(0 0% 0% / 0.25);
}

.dark {
  --background: oklch(0.1776 0 0);
  --foreground: oklch(0.9491 0 0);
  --card: oklch(0.2134 0 0);
  --card-foreground: oklch(0.9491 0 0);
  --popover: oklch(0.2134 0 0);
  --popover-foreground: oklch(0.9491 0 0);
  --primary: oklch(0.9247 0.0524 66.1732);
  --primary-foreground: oklch(0.2029 0.0240 200.1962);
  --secondary: oklch(0.3163 0.0190 63.6992);
  --secondary-foreground: oklch(0.9247 0.0524 66.1732);
  --muted: oklch(0.2520 0 0);
  --muted-foreground: oklch(0.7699 0 0);
  --accent: oklch(0.2850 0 0);
  --accent-foreground: oklch(0.9491 0 0);
  --destructive: oklch(0.6271 0.1936 33.3390);
  --destructive-foreground: oklch(1.0000 0 0);
  --success: oklch(0.7 0.2 150);
  --warning: oklch(0.85 0.2 90);
  --border: oklch(0.2351 0.0115 91.7467);
  --input: oklch(0.4017 0 0);
  --ring: oklch(0.9247 0.0524 66.1732);
  --chart-1: oklch(0.9247 0.0524 66.1732);
  --chart-2: oklch(0.3163 0.0190 63.6992);
  --chart-3: oklch(0.2850 0 0);
  --chart-4: oklch(0.3481 0.0219 67.0001);
  --chart-5: oklch(0.9245 0.0533 67.0855);
  --sidebar: oklch(0.2103 0.0059 285.8852);
  --sidebar-foreground: oklch(0.9674 0.0013 286.3752);
  --sidebar-primary: oklch(0.4882 0.2172 264.3763);
  --sidebar-primary-foreground: oklch(1.0000 0 0);
  --sidebar-accent: oklch(0.2739 0.0055 286.0326);
  --sidebar-accent-foreground: oklch(0.9674 0.0013 286.3752);
  --sidebar-border: oklch(0.2739 0.0055 286.0326);
  --sidebar-ring: oklch(0.8711 0.0055 286.2860);
  --font-sans: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, 'Noto Sans', sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
  --font-serif: ui-serif, Georgia, Cambria, "Times New Roman", Times, serif;
  --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  --radius: 0.5rem;
  --shadow-2xs: 0 1px 3px 0px hsl(0 0% 0% / 0.05);
  --shadow-xs: 0 1px 3px 0px hsl(0 0% 0% / 0.05);
  --shadow-sm: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 1px 2px -1px hsl(0 0% 0% / 0.10);
  --shadow: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 1px 2px -1px hsl(0 0% 0% / 0.10);
  --shadow-md: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 2px 4px -1px hsl(0 0% 0% / 0.10);
  --shadow-lg: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 4px 6px -1px hsl(0 0% 0% / 0.10);
  --shadow-xl: 0 1px 3px 0px hsl(0 0% 0% / 0.10), 0 8px 10px -1px hsl(0 0% 0% / 0.10);
  --shadow-2xl: 0 1px 3px 0px hsl(0 0% 0% / 0.25);
}
`;

const ThemeEditorModal = ({ theme, isOpen, onClose, onSave }: ThemeEditorModalProps) => {
  const { t } = useTranslation();
  const { resolvedTheme } = useTheme();
  const [name, setName] = useState("");
  const [css, setCss] = useState("");
  const isReadOnly = theme?.id === 'default';

  useEffect(() => {
    if (isOpen) {
      if (theme) {
        setName(theme.name);
        if (theme.id === 'default') {
          setCss(DEFAULT_THEME_CSS);
        } else {
          setCss(theme.css || '');
        }
      } else {
        // Reset for new theme creation
        setName("");
        setCss(NEW_THEME_PLACEHOLDER);
      }
    }
  }, [theme, isOpen]);

  const handleEditorChange = (value: string | undefined) => {
    if (isReadOnly) return;
    setCss(value || '');
  };

  const handleSave = () => {
    if (isReadOnly) return;
    
    const themeData: Partial<Theme> = {
      name,
      css,
    };

    if (theme) {
      onSave({ id: theme.id, ...themeData });
    } else {
      onSave(themeData);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[800px] flex flex-col max-h-[90vh]">
        <DialogHeader>
          <DialogTitle>{theme ? (isReadOnly ? t("themeSettings.editor.viewTitle") : t("themeSettings.editor.editTitle")) : t("themeSettings.editor.createTitle")}</DialogTitle>
          <DialogDescription>
            {isReadOnly
              ? t("themeSettings.editor.readOnlyDescription")
              : (
                <Trans
                  i18nKey="themeSettings.editor.description"
                  components={[
                    <a
                      href="https://tweakcn.com/editor/theme"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-primary underline"
                    />,
                  ]}
                />
              )}
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4 py-4 flex-grow overflow-y-auto pr-6">
          <div className="flex items-center gap-4">
            <Label htmlFor="name" className="w-16 text-right">
              {t("themeSettings.editor.nameLabel")}
            </Label>
            <Input id="name" value={name} onChange={(e) => setName(e.target.value)} className="flex-1" disabled={isReadOnly} />
          </div>
          <div className="flex flex-col gap-2 flex-grow min-h-0">
            <Label htmlFor="config">
              {t("themeSettings.editor.cssLabel")}
            </Label>
            <div className="border rounded-md overflow-hidden flex-grow h-[400px]">
              <Editor
                height="100%"
                language="css"
                value={css}
                onChange={handleEditorChange}
                theme={resolvedTheme === 'light' ? 'vs-light' : 'vs-dark'}
                options={{
                  minimap: { enabled: false },
                  scrollbar: { vertical: 'auto' },
                  readOnly: isReadOnly,
                }}
              />
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>{t("buttons.cancel")}</Button>
          <Button onClick={handleSave} disabled={isReadOnly}>{t("themeSettings.editor.saveButton")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default ThemeEditorModal;