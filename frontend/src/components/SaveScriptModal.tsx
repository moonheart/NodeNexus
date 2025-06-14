import React, { useState } from 'react';

interface Props {
    isOpen: boolean;
    onClose: () => void;
    onSave: (name: string, description: string) => void;
    initialCommand: string;
}

const SaveScriptModal: React.FC<Props> = ({ isOpen, onClose, onSave, initialCommand }) => {
    const [name, setName] = useState('');
    const [description, setDescription] = useState('');

    if (!isOpen) return null;

    const handleSave = () => {
        if (name.trim()) {
            onSave(name, description);
            onClose();
        } else {
            alert('Script name is required.');
        }
    };

    return (
        <div className="fixed inset-0 bg-black/50 flex justify-center items-center z-50">
            <div className="bg-white p-6 rounded-lg shadow-xl w-full max-w-md">
                <h2 className="text-xl font-bold mb-4">Save Command as Script</h2>
                <div className="mb-4">
                    <label htmlFor="script-name" className="block text-sm font-medium text-gray-700">Script Name</label>
                    <input
                        type="text"
                        id="script-name"
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                        required
                    />
                </div>
                <div className="mb-4">
                    <label htmlFor="script-description" className="block text-sm font-medium text-gray-700">Description (Optional)</label>
                    <textarea
                        id="script-description"
                        value={description}
                        onChange={(e) => setDescription(e.target.value)}
                        rows={3}
                        className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                    />
                </div>
                <div className="mb-4">
                    <p className="block text-sm font-medium text-gray-700">Command</p>
                    <p className="mt-1 p-2 bg-gray-100 rounded-md text-sm text-gray-800 font-mono break-all">{initialCommand}</p>
                </div>
                <div className="flex justify-end space-x-2">
                    <button onClick={onClose} className="px-4 py-2 bg-gray-300 text-gray-800 rounded-md hover:bg-gray-400">
                        Cancel
                    </button>
                    <button onClick={handleSave} className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700">
                        Save Script
                    </button>
                </div>
            </div>
        </div>
    );
};

export default SaveScriptModal;