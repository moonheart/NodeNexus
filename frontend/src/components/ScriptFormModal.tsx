import React, { useState, useEffect } from 'react';
import { Editor } from '@monaco-editor/react';
import { useTheme } from "@/components/ThemeProvider";
import type { ScriptPayload, CommandScript } from '../services/scriptService';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

interface ScriptFormModalProps {
    isOpen: boolean;
    onClose: () => void;
    onSave: (data: ScriptPayload) => void;
    initialData?: CommandScript;
}

const ScriptFormModal: React.FC<ScriptFormModalProps> = ({ isOpen, onClose, onSave, initialData }) => {
    const { theme } = useTheme();
    const [formData, setFormData] = useState<ScriptPayload>({
        name: '',
        description: '',
        language: 'shell',
        script_content: '',
        working_directory: '.',
    });

    useEffect(() => {
        const resetForm = (): ScriptPayload => ({
            name: '',
            description: '',
            language: 'shell',
            script_content: '',
            working_directory: '.',
        });

        if (initialData) {
            setFormData({
                name: initialData.name,
                description: initialData.description || '',
                language: initialData.language,
                script_content: initialData.script_content,
                working_directory: initialData.working_directory,
            });
        } else {
            setFormData(resetForm());
        }
    }, [initialData, isOpen]);

    const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
        const { name, value } = e.target;
        setFormData(prev => ({ ...prev, [name]: value }));
    };
    
    const handleSelectChange = (name: keyof ScriptPayload, value: string) => {
        setFormData(prev => ({ ...prev, [name]: value }));
    };

    const handleEditorChange = (value: string | undefined) => {
        setFormData(prev => ({ ...prev, script_content: value || '' }));
    };

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        onSave(formData);
    };

    return (
        <Dialog open={isOpen} onOpenChange={onClose}>
            <DialogContent className="sm:max-w-3xl max-h-[90vh] flex flex-col">
                <DialogHeader>
                    <DialogTitle>{initialData ? 'Edit Script' : 'Create New Script'}</DialogTitle>
                    <DialogDescription>
                        Fill in the details for your command script below.
                    </DialogDescription>
                </DialogHeader>
                <form onSubmit={handleSubmit} className="flex-grow flex flex-col space-y-4 overflow-y-auto min-h-0 p-1">
                    <div className="grid gap-2">
                        <Label htmlFor="name">Name</Label>
                        <Input id="name" name="name" value={formData.name} onChange={handleChange} required />
                    </div>
                    <div className="grid gap-2">
                        <Label htmlFor="description">Description</Label>
                        <Textarea id="description" name="description" value={formData.description || ''} onChange={handleChange} rows={2} />
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div className="grid gap-2">
                            <Label htmlFor="language">Language</Label>
                            <Select name="language" value={formData.language} onValueChange={(value) => handleSelectChange('language', value)}>
                                <SelectTrigger>
                                    <SelectValue placeholder="Select language" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="shell">Shell</SelectItem>
                                    <SelectItem value="powershell">PowerShell</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="working_directory">Working Directory</Label>
                            <Input id="working_directory" name="working_directory" value={formData.working_directory} onChange={handleChange} required />
                        </div>
                    </div>
                    <div className="flex-grow flex flex-col min-h-0">
                        <Label htmlFor="script_content" className="mb-2">Script Content</Label>
                        <div className="border rounded-md overflow-hidden flex-grow h-48">
                            <Editor
                                height="100%"
                                language={formData.language}
                                value={formData.script_content}
                                onChange={handleEditorChange}
                                theme={theme === 'light' ? 'vs-light' : 'vs-dark'}
                                options={{ minimap: { enabled: false }, scrollbar: { vertical: 'auto' } }}
                            />
                        </div>
                    </div>
                </form>
                 <DialogFooter className="pt-4">
                    <Button type="button" variant="outline" onClick={onClose}>Cancel</Button>
                    <Button type="submit" onClick={handleSubmit}>Save</Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};

export default ScriptFormModal;