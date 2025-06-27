import React, { useState, useEffect } from 'react';
import { Editor } from '@monaco-editor/react';
import type { ScriptPayload, CommandScript } from '../services/scriptService';

interface ScriptFormModalProps {
    isOpen: boolean;
    onClose: () => void;
    onSave: (data: ScriptPayload) => void;
    initialData?: CommandScript;
}

const ScriptFormModal: React.FC<ScriptFormModalProps> = ({ isOpen, onClose, onSave, initialData }) => {
    const [formData, setFormData] = useState<ScriptPayload>({
        name: '',
        description: '',
        language: 'shell',
        script_content: '',
        working_directory: '',
    });

    useEffect(() => {
        if (initialData) {
            setFormData({
                name: initialData.name,
                description: initialData.description || '',
                language: initialData.language,
                script_content: initialData.script_content,
                working_directory: initialData.working_directory,
            });
        } else {
            setFormData({
                name: '',
                description: '',
                language: 'shell',
                script_content: '',
                working_directory: '',
            });
        }
    }, [initialData, isOpen]);

    const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) => {
        const { name, value } = e.target;
        setFormData(prev => ({ ...prev, [name]: value }));
    };

    const handleEditorChange = (value: string | undefined) => {
        setFormData(prev => ({ ...prev, script_content: value || '' }));
    };

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        onSave(formData);
    };

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 bg-black/50  z-50 flex justify-center items-center">
            <div className="bg-white rounded-lg shadow-xl p-6 w-full max-w-3xl max-h-[90vh] flex flex-col">
                <h2 className="text-2xl font-bold mb-4">{initialData ? 'Edit Script' : 'Create New Script'}</h2>
                <form onSubmit={handleSubmit} className="flex-grow flex flex-col space-y-4 overflow-y-auto min-h-0">
                    <div>
                        <label htmlFor="name" className="block text-sm font-medium text-gray-700">Name</label>
                        <input type="text" name="name" id="name" value={formData.name} onChange={handleChange} required className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm" />
                    </div>
                    <div>
                        <label htmlFor="description" className="block text-sm font-medium text-gray-700">Description</label>
                        <textarea name="description" id="description" value={formData.description || ''} onChange={handleChange} rows={2} className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"></textarea>
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div>
                            <label htmlFor="language" className="block text-sm font-medium text-gray-700">Language</label>
                            <select name="language" id="language" value={formData.language} onChange={handleChange} className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm">
                                <option value="shell">Shell</option>
                                <option value="powershell">PowerShell</option>
                            </select>
                        </div>
                        <div>
                            <label htmlFor="working_directory" className="block text-sm font-medium text-gray-700">Working Directory</label>
                            <input type="text" name="working_directory" id="working_directory" value={formData.working_directory} onChange={handleChange} required className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm" />
                        </div>
                    </div>
                    <div className="flex-grow flex flex-col min-h-0">
                        <label htmlFor="script_content" className="block text-sm font-medium text-gray-700 mb-1">Script Content</label>
                        <div className="border rounded-md overflow-hidden flex-grow h-48">
                            <Editor
                                height="100%"
                                language={formData.language}
                                value={formData.script_content}
                                onChange={handleEditorChange}
                                theme="vs-dark"
                                options={{ minimap: { enabled: false } }}
                            />
                        </div>
                    </div>
                    <div className="pt-4 flex justify-end space-x-2">
                        <button type="button" onClick={onClose} className="bg-gray-200 text-gray-800 px-4 py-2 rounded-md hover:bg-gray-300">Cancel</button>
                        <button type="submit" className="bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700">Save</button>
                    </div>
                </form>
            </div>
        </div>
    );
};

export default ScriptFormModal;