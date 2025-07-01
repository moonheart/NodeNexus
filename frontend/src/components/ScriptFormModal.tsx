import React, { useState, useEffect } from 'react';
import { Editor } from '@monaco-editor/react';
import { useTheme } from "@/components/ThemeProvider";
import { useTranslation } from 'react-i18next';
import type { ScriptPayload } from '../services/scriptService';
import type { CommandScript } from '../types';
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
    const { t } = useTranslation();
    const { resolvedTheme } = useTheme();
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
                    <DialogTitle>{initialData ? t('common.form.editTitle') : t('common.form.createTitle')}</DialogTitle>
                    <DialogDescription>
                        {t('scriptManagement.form.description')}
                    </DialogDescription>
                </DialogHeader>
                <form onSubmit={handleSubmit} className="flex-grow flex flex-col space-y-4 overflow-y-auto min-h-0 p-1">
                    <div className="grid gap-2">
                        <Label htmlFor="name">{t('common.labels.name')}</Label>
                        <Input id="name" name="name" value={formData.name} onChange={handleChange} required />
                    </div>
                    <div className="grid gap-2">
                        <Label htmlFor="description">{t('common.labels.description')}</Label>
                        <Textarea id="description" name="description" value={formData.description || ''} onChange={handleChange} rows={2} />
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div className="grid gap-2">
                            <Label htmlFor="language">{t('common.labels.language')}</Label>
                            <Select name="language" value={formData.language} onValueChange={(value) => handleSelectChange('language', value)}>
                                <SelectTrigger>
                                    <SelectValue placeholder={t('common.placeholders.selectLanguage')} />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="shell">{t('scriptManagement.form.languages.shell')}</SelectItem>
                                    <SelectItem value="powershell">{t('scriptManagement.form.languages.powershell')}</SelectItem>
                                </SelectContent>
                            </Select>
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="working_directory">{t('common.labels.workingDirectory')}</Label>
                            <Input id="working_directory" name="working_directory" value={formData.working_directory} onChange={handleChange} required />
                        </div>
                    </div>
                    <div className="flex-grow flex flex-col min-h-0">
                        <Label htmlFor="script_content" className="mb-2">{t('common.labels.content')}</Label>
                        <div className="border rounded-md overflow-hidden flex-grow h-48">
                            <Editor
                                height="100%"
                                language={formData.language}
                                value={formData.script_content}
                                onChange={handleEditorChange}
                                theme={resolvedTheme === 'light' ? 'vs-light' : 'vs-dark'}
                                options={{ minimap: { enabled: false }, scrollbar: { vertical: 'auto' } }}
                            />
                        </div>
                    </div>
                </form>
                 <DialogFooter className="pt-4">
                    <Button type="button" variant="outline" onClick={onClose}>{t('common.actions.cancel')}</Button>
                    <Button type="submit" onClick={handleSubmit}>{t('common.actions.save')}</Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};

export default ScriptFormModal;
