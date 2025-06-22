import React, { useState, useEffect } from 'react';
import type { ServiceMonitor, ServiceMonitorInput, Tag, HttpMonitorConfig, VpsListItemResponse } from '../types';
import { getAllVpsListItems } from '../services/vpsService';
import { getTags } from '../services/tagService';
import toast from 'react-hot-toast';

interface ServiceMonitorModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (monitor: ServiceMonitorInput, id?: number) => void;
  monitorToEdit?: ServiceMonitor | null;
}

const ServiceMonitorModal: React.FC<ServiceMonitorModalProps> = ({ isOpen, onClose, onSave, monitorToEdit }) => {
  const [formData, setFormData] = useState<ServiceMonitorInput>({
    name: '',
    monitorType: 'http',
    target: '',
    frequencySeconds: 60,
    timeoutSeconds: 10,
    isActive: true,
    monitorConfig: {},
    assignments: {
      agentIds: [],
      tagIds: [],
      assignmentType: 'INCLUSIVE',
    },
  });
  const [allAgents, setAllAgents] = useState<VpsListItemResponse[]>([]);
  const [allTags, setAllTags] = useState<Tag[]>([]);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [agentsResponse, tags] = await Promise.all([getAllVpsListItems(), getTags()]);
        // The Vps type is compatible with what we need for the list.
        setAllAgents(agentsResponse as VpsListItemResponse[]);
        setAllTags(tags);
      } catch (error) {
        console.error("Failed to fetch agents and tags:", error);
        toast.error('Failed to fetch agents and tags.');
      }
    };
    if (isOpen) {
      fetchData();
    }
  }, [isOpen]);

  useEffect(() => {
    if (monitorToEdit) {
      setFormData({
        name: monitorToEdit.name,
        monitorType: monitorToEdit.monitorType,
        target: monitorToEdit.target,
        frequencySeconds: monitorToEdit.frequencySeconds,
        timeoutSeconds: monitorToEdit.timeoutSeconds,
        isActive: monitorToEdit.isActive,
        monitorConfig: monitorToEdit.monitorConfig || {},
        assignments: {
          agentIds: monitorToEdit.agentIds || [],
          tagIds: monitorToEdit.tagIds || [],
          assignmentType: monitorToEdit.assignmentType || 'INCLUSIVE',
        }
      });
    } else {
      // Reset to default when opening for creation
      setFormData({
        name: '',
        monitorType: 'http',
        target: '',
        frequencySeconds: 60,
        timeoutSeconds: 10,
        isActive: true,
        monitorConfig: {},
        assignments: {
          agentIds: [],
          tagIds: [],
          assignmentType: 'INCLUSIVE',
        },
      });
    }
  }, [monitorToEdit, isOpen]);


  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value, type } = e.target;
    const isNumber = type === 'number';
    setFormData(prev => ({ ...prev, [name]: isNumber ? parseInt(value, 10) : value }));
  };

  const handleAssignmentChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData(prev => ({
        ...prev,
        assignments: {
            ...prev.assignments,
            [name]: value
        }
    }));
  };
  
  const handleHttpConfigChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setFormData(prev => ({
      ...prev,
      monitorConfig: {
        ...prev.monitorConfig,
        [name]: value,
      } as HttpMonitorConfig,
    }));
  };

  const handleMultiSelectChange = (e: React.ChangeEvent<HTMLSelectElement>, field: 'agentIds' | 'tagIds') => {
    const selectedIds = Array.from(e.target.selectedOptions, option => parseInt(option.value, 10));
    setFormData(prev => ({
        ...prev,
        assignments: {
            ...prev.assignments,
            [field]: selectedIds
        }
    }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSave(formData, monitorToEdit?.id);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex justify-center items-center">
      <div className="bg-white rounded-lg shadow-xl p-6 w-full max-w-3xl max-h-[90vh] overflow-y-auto">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-xl font-bold">{monitorToEdit ? 'Edit' : 'Create'} Service Monitor</h2>
          <button onClick={onClose} className="text-gray-500 hover:text-gray-800">&times;</button>
        </div>
        
        <form onSubmit={handleSubmit}>
          <div className="space-y-4">
            {/* Basic Info */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label htmlFor="name" className="block text-sm font-medium text-gray-700">Monitor Name</label>
                <input type="text" id="name" name="name" value={formData.name} onChange={handleChange} required className="mt-1 block w-full border-gray-300 rounded-md shadow-sm" />
              </div>
              <div>
                <label htmlFor="monitorType" className="block text-sm font-medium text-gray-700">Monitor Type</label>
                <select id="monitorType" name="monitorType" value={formData.monitorType} onChange={handleChange} className="mt-1 block w-full border-gray-300 rounded-md shadow-sm">
                  <option value="http">HTTP(s)</option>
                  <option value="ping">Ping</option>
                  <option value="tcp">TCP Port</option>
                </select>
              </div>
            </div>
            <div>
              <label htmlFor="target" className="block text-sm font-medium text-gray-700">Target</label>
              <input type="text" id="target" name="target" value={formData.target} onChange={handleChange} placeholder="e.g., https://example.com or 8.8.8.8:53" required className="mt-1 block w-full border-gray-300 rounded-md shadow-sm" />
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label htmlFor="frequencySeconds" className="block text-sm font-medium text-gray-700">Frequency (seconds)</label>
                <input type="number" id="frequencySeconds" name="frequencySeconds" value={formData.frequencySeconds} onChange={handleChange} min="10" className="mt-1 block w-full border-gray-300 rounded-md shadow-sm" />
              </div>
              <div>
                <label htmlFor="timeoutSeconds" className="block text-sm font-medium text-gray-700">Timeout (seconds)</label>
                <input type="number" id="timeoutSeconds" name="timeoutSeconds" value={formData.timeoutSeconds} onChange={handleChange} min="1" className="mt-1 block w-full border-gray-300 rounded-md shadow-sm" />
              </div>
            </div>

            {/* Dynamic Config Section */}
            {formData.monitorType === 'http' && (
              <div className="p-4 border rounded-md bg-gray-50">
                <h3 className="text-lg font-medium mb-2">HTTP Options</h3>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label htmlFor="expected_status_codes" className="block text-sm font-medium text-gray-700">Expected Status Codes</label>
                        <input type="text" id="expected_status_codes" name="expected_status_codes"
                               value={(formData.monitorConfig as HttpMonitorConfig)?.expected_status_codes?.join(', ') || '200'}
                               onChange={handleHttpConfigChange}
                               placeholder="e.g., 200, 201"
                               className="mt-1 block w-full border-gray-300 rounded-md shadow-sm" />
                    </div>
                    <div>
                        <label htmlFor="response_body_match" className="block text-sm font-medium text-gray-700">Response Body Match</label>
                        <input type="text" id="response_body_match" name="response_body_match"
                               value={(formData.monitorConfig as HttpMonitorConfig)?.response_body_match || ''}
                               onChange={handleHttpConfigChange}
                               placeholder="Text to find in response body"
                               className="mt-1 block w-full border-gray-300 rounded-md shadow-sm" />
                    </div>
                </div>
              </div>
            )}

            {/* Assignments */}
            <div className="p-4 border rounded-md bg-gray-50">
                <h3 className="text-lg font-medium mb-2">Assignments</h3>
                <div className="mb-4">
                    <label className="block text-sm font-medium text-gray-700">Assignment Mode</label>
                    <div className="flex items-center space-x-4 mt-1">
                        <label className="flex items-center">
                            <input type="radio" name="assignmentType" value="INCLUSIVE" checked={formData.assignments?.assignmentType === 'INCLUSIVE'} onChange={handleAssignmentChange} className="form-radio"/>
                            <span className="ml-2">Inclusive (Apply to selected)</span>
                        </label>
                        <label className="flex items-center">
                            <input type="radio" name="assignmentType" value="EXCLUSIVE" checked={formData.assignments?.assignmentType === 'EXCLUSIVE'} onChange={handleAssignmentChange} className="form-radio"/>
                            <span className="ml-2">Exclusive (Apply to all except selected)</span>
                        </label>
                    </div>
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label htmlFor="agentIds" className="block text-sm font-medium text-gray-700">
                      {formData.assignments?.assignmentType === 'EXCLUSIVE' ? 'Exclude Agents' : 'Assign to Agents'}
                    </label>
                    <select id="agentIds" name="agentIds" multiple value={formData.assignments?.agentIds?.map(String)} onChange={(e) => handleMultiSelectChange(e, 'agentIds')} className="mt-1 block w-full h-32 border-gray-300 rounded-md shadow-sm">
                      {allAgents.map(agent => (
                        <option key={agent.id} value={agent.id}>{agent.name}</option>
                      ))}
                    </select>
                  </div>
                  <div>
                    <label htmlFor="tagIds" className="block text-sm font-medium text-gray-700">
                      {formData.assignments?.assignmentType === 'EXCLUSIVE' ? 'Exclude Tags' : 'Assign to Tags'}
                    </label>
                    <select id="tagIds" name="tagIds" multiple value={formData.assignments?.tagIds?.map(String)} onChange={(e) => handleMultiSelectChange(e, 'tagIds')} className="mt-1 block w-full h-32 border-gray-300 rounded-md shadow-sm">
                      {allTags.map(tag => (
                        <option key={tag.id} value={tag.id} style={{ color: tag.color }}>{tag.name}</option>
                      ))}
                    </select>
                  </div>
                </div>
            </div>
          </div>

          <div className="mt-6 flex justify-end space-x-2">
            <button type="button" onClick={onClose} className="bg-gray-200 text-gray-800 px-4 py-2 rounded-md hover:bg-gray-300">Cancel</button>
            <button type="submit" className="bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700">Save</button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default ServiceMonitorModal;