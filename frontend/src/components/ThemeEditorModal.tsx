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
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { useEffect, useState } from "react";
import { Editor } from '@monaco-editor/react';
import { useTheme } from "@/components/ThemeProvider";
import type { Theme } from "@/pages/ThemeSettingsPage"; // Import the unified Theme type

interface ThemeEditorModalProps {
  theme: Theme | null;
  isOpen: boolean;
  onClose: () => void;
 onSave: (data: Partial<Theme> & { id?: string }) => void;
  isOfficial?: boolean;
}

const ThemeEditorModal = ({ theme, isOpen, onClose, onSave, isOfficial }: ThemeEditorModalProps) => {
  const { themeType } = useTheme();
  const [name, setName] = useState("");
  const [type, setType] = useState<'light' | 'dark'>("light");
  const [config, setConfig] = useState("");
  const isReadOnly = isOfficial || theme?.is_official;

  useEffect(() => {
    if (theme) {
      setName(theme.name);
      setType(theme.type);
      setConfig(JSON.stringify(theme.config, null, 2));
    } else {
      // Reset for new theme creation
      setName("");
      setType("light");
      setConfig("{\n  \"--background\": \"oklch(1 0 0)\"\n}");
    }
  }, [theme, isOpen]);

  const handleEditorChange = (value: string | undefined) => {
    if (isReadOnly) return;
    setConfig(value || '');
  };

  const handleSave = () => {
    if (isReadOnly) return;
    try {
      const parsedConfig = JSON.parse(config);
      const themeData = {
        name,
        type,
        config: parsedConfig,
      };

      if (theme) {
        onSave({ id: theme.id, ...themeData });
      } else {
        onSave(themeData);
      }
    } catch {
      alert("Invalid JSON in config.");
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[800px] flex flex-col max-h-[90vh]">
        <DialogHeader>
          <DialogTitle>{theme ? (isReadOnly ? "View Theme" : "Edit Theme") : "Create New Theme"}</DialogTitle>
          <DialogDescription>
            {isReadOnly
              ? "This is an official theme and cannot be edited. You can duplicate its configuration to create a new custom theme."
              : "Define your theme properties below. Use valid JSON for the configuration."}
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4 py-4 flex-grow overflow-y-auto pr-6">
          <div className="flex gap-4">
            <div className="flex-1 flex items-center gap-4">
              <Label htmlFor="name" className="w-16 text-right">
                Name
              </Label>
              <Input id="name" value={name} onChange={(e) => setName(e.target.value)} className="flex-1" disabled={isReadOnly} />
            </div>
            <div className="flex-1 flex items-center gap-4">
              <Label className="w-16 text-right">Type</Label>
              <RadioGroup value={type} onValueChange={(value) => setType(value as 'light' | 'dark')} className="flex-1 flex gap-4" disabled={isReadOnly}>
                <div className="flex items-center space-x-2">
                  <RadioGroupItem value="light" id="r1" disabled={isReadOnly} />
                  <Label htmlFor="r1">Light</Label>
                </div>
                <div className="flex items-center space-x-2">
                  <RadioGroupItem value="dark" id="r2" disabled={isReadOnly} />
                  <Label htmlFor="r2">Dark</Label>
                </div>
              </RadioGroup>
            </div>
          </div>
          <div className="flex flex-col gap-2 flex-grow min-h-0">
            <Label htmlFor="config">
              Config (JSON)
            </Label>
            <div className="border rounded-md overflow-hidden flex-grow h-[400px]">
              <Editor
                height="100%"
                language="json"
                value={config}
                onChange={handleEditorChange}
                theme={themeType === 'light' ? 'vs-light' : 'vs-dark'}
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