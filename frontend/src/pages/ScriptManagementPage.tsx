import React, { useState, useEffect, useMemo } from 'react';
import toast from 'react-hot-toast';
import { scriptService, type CommandScript, type ScriptPayload } from '../services/scriptService';
import ScriptFormModal from '../components/ScriptFormModal';
import { Plus, Search } from 'lucide-react';

const ScriptManagementPage: React.FC = () => {
    const [scripts, setScripts] = useState<CommandScript[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [editingScript, setEditingScript] = useState<CommandScript | undefined>(undefined);
    const [searchQuery, setSearchQuery] = useState('');

    const fetchScripts = async () => {
        try {
            setLoading(true);
            const data = await scriptService.getScripts();
            setScripts(data);
            setError(null);
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'An unknown error occurred.';
            setError(errorMessage);
            toast.error(`Failed to fetch scripts: ${errorMessage}`);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchScripts();
    }, []);

    const handleAdd = () => {
        setEditingScript(undefined);
        setIsModalOpen(true);
    };

    const handleEdit = (script: CommandScript) => {
        setEditingScript(script);
        setIsModalOpen(true);
    };

    const handleDelete = async (id: number) => {
        if (window.confirm('Are you sure you want to delete this script?')) {
            try {
                await scriptService.deleteScript(id);
                toast.success('Script deleted successfully!');
                fetchScripts(); // Refresh list
            } catch (err) {
                const errorMessage = err instanceof Error ? err.message : 'An unknown error occurred.';
                toast.error(`Failed to delete script: ${errorMessage}`);
            }
        }
    };

    const handleSave = async (data: ScriptPayload) => {
        try {
            if (editingScript) {
                await scriptService.updateScript(editingScript.id, data);
                toast.success('Script updated successfully!');
            } else {
                await scriptService.createScript(data);
                toast.success('Script created successfully!');
            }
            setIsModalOpen(false);
            fetchScripts(); // Refresh list
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'An unknown error occurred.';
            toast.error(`Failed to save script: ${errorMessage}`);
        }
    };

    const filteredScripts = useMemo(() => {
        if (!searchQuery) {
            return scripts;
        }
        return scripts.filter(script =>
            script.name.toLowerCase().includes(searchQuery.toLowerCase())
        );
    }, [scripts, searchQuery]);

    const renderContent = () => {
        if (loading) {
            // Simple text loader, can be replaced with a skeleton loader component
            return <p>Loading scripts...</p>;
        }

        if (error) {
            return <p className="text-red-500">Error: {error}</p>;
        }

        if (scripts.length === 0) {
            return (
                <div className="text-center py-10">
                    <h3 className="text-lg font-medium text-gray-900">No scripts found</h3>
                    <p className="mt-1 text-sm text-gray-500">Get started by creating a new script.</p>
                    <div className="mt-6">
                        <button
                            onClick={handleAdd}
                            type="button"
                            className="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
                        >
                            <Plus className="-ml-1 mr-2 h-5 w-5" aria-hidden="true" />
                            New Script
                        </button>
                    </div>
                </div>
            );
        }

        return (
            <div className="bg-white shadow overflow-hidden sm:rounded-lg">
                <div className="overflow-x-auto">
                    <table className="min-w-full divide-y divide-gray-200">
                        <thead className="bg-gray-50">
                            <tr>
                                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Name</th>
                                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Description</th>
                                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Language</th>
                                <th scope="col" className="relative px-6 py-3"><span className="sr-only">Actions</span></th>
                            </tr>
                        </thead>
                        <tbody className="bg-white divide-y divide-gray-200">
                            {filteredScripts.map((script) => (
                                <tr key={script.id}>
                                    <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">{script.name}</td>
                                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 max-w-xs truncate">{script.description}</td>
                                    <td className="px-6 py-4 whitespace-nowrap">
                                        <span className={`px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${script.language === 'shell' ? 'bg-blue-100 text-blue-800' : 'bg-green-100 text-green-800'}`}>
                                            {script.language}
                                        </span>
                                    </td>
                                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium space-x-4">
                                        <button onClick={() => handleEdit(script)} className="text-indigo-600 hover:text-indigo-900">Edit</button>
                                        <button onClick={() => handleDelete(script.id)} className="text-red-600 hover:text-red-900">Delete</button>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            </div>
        );
    };

    return (
        <div className="space-y-6">
            <div className="flex justify-between items-center">
                <h1 className="text-2xl font-bold">Script Management</h1>
                {scripts.length > 0 && (
                     <button
                        onClick={handleAdd}
                        className="bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
                    >
                        Add New Script
                    </button>
                )}
            </div>

            {scripts.length > 0 && (
                <div className="relative">
                    <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                        <Search className="h-5 w-5 text-gray-400" aria-hidden="true" />
                    </div>
                    <input
                        type="text"
                        name="search"
                        id="search"
                        className="block w-full pl-10 pr-3 py-2 border border-gray-300 rounded-md leading-5 bg-white placeholder-gray-500 focus:outline-none focus:placeholder-gray-400 focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                        placeholder="Search by name..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                    />
                </div>
            )}

            {renderContent()}

            <ScriptFormModal
                isOpen={isModalOpen}
                onClose={() => setIsModalOpen(false)}
                onSave={handleSave}
                initialData={editingScript}
            />
        </div>
    );
};

export default ScriptManagementPage;