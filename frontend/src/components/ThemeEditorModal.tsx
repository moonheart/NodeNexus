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

const ThemeEditorModal = ({ theme, isOpen, onClose, onSave }: ThemeEditorModalProps) => {
  const { themeMode } = useTheme();
  const [name, setName] = useState("");
  const [css, setCss] = useState("");
  const isReadOnly = theme?.id === 'default';

  useEffect(() => {
    if (isOpen) {
      if (theme) {
        setName(theme.name);
        setCss(theme.css || '');
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
          <DialogTitle>{theme ? (isReadOnly ? "View Theme" : "Edit Theme") : "Create New Theme"}</DialogTitle>
          <DialogDescription>
            {isReadOnly
              ? "The default theme cannot be edited. You can duplicate its configuration to create a new custom theme."
              : "Define your theme by providing a name and the full CSS for both light (:root) and dark (.dark) modes."}
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4 py-4 flex-grow overflow-y-auto pr-6">
          <div className="flex items-center gap-4">
            <Label htmlFor="name" className="w-16 text-right">
              Name
            </Label>
            <Input id="name" value={name} onChange={(e) => setName(e.target.value)} className="flex-1" disabled={isReadOnly} />
          </div>
          <div className="flex flex-col gap-2 flex-grow min-h-0">
            <Label htmlFor="config">
              Theme CSS
            </Label>
            <div className="border rounded-md overflow-hidden flex-grow h-[400px]">
              <Editor
                height="100%"
                language="css"
                value={css}
                onChange={handleEditorChange}
                theme={themeMode === 'light' ? 'vs-light' : 'vs-dark'}
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
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button onClick={handleSave} disabled={isReadOnly}>Save Theme</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default ThemeEditorModal;