import React, { useState, useEffect } from 'react';
import type { ChannelTemplate, ChannelResponse, CreateChannelRequest, UpdateChannelRequest } from '../types';
import DynamicForm from './DynamicForm';

interface NotificationChannelModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: CreateChannelRequest | UpdateChannelRequest) => Promise<void>;
  templates: ChannelTemplate[];
  editingChannel?: ChannelResponse | null; // For pre-filling form when editing
  // We might need to fetch full config for editing, or adjust backend ChannelResponse
}

const NotificationChannelModal: React.FC<NotificationChannelModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  templates,
  editingChannel,
}) => {
  const [channelName, setChannelName] = useState('');
  const [selectedTemplate, setSelectedTemplate] = useState<ChannelTemplate | null>(null);
  const [formData, setFormData] = useState<Record<string, unknown>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      if (editingChannel && templates.length > 0) {
        const template = templates.find(t => t.channelType === editingChannel.channelType);
        setSelectedTemplate(template || null);
        setChannelName(editingChannel.name);
        const initialFormData: Record<string, unknown> = {};
        // Populate formData based on the editing channel's config if available,
        // otherwise, use template fields. This needs backend to send full config.
        // For now, assuming editingChannel.config is available or we initialize from template.
        if (template) {
            template.fields.forEach(field => {
                initialFormData[field.name] = editingChannel?.config?.[field.name] || '';
            });
        }
        setFormData(initialFormData);
      } else if (!editingChannel) {
        // Reset form for new channel creation when modal opens
        setChannelName('');
        setSelectedTemplate(null); // Explicitly set to null so user must select
        setFormData({});
      }
    } else {
      // Optionally reset when modal closes, if desired
      // setChannelName('');
      // setSelectedTemplate(null);
      // setFormData({});
    }
  }, [isOpen, editingChannel, templates]); // Removed reset from dependencies


  const handleFormChange = (fieldName: string, value: unknown) => {
    setFormData(prev => ({ ...prev, [fieldName]: value }));
  };

  const handleTemplateChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const templateType = e.target.value;
    console.log("Selected templateType:", templateType);
    const template = templates.find(t => t.channelType === templateType) || null;
    console.log("Found template:", template);
    setSelectedTemplate(template);
    
    const newFormData: Record<string, unknown> = {};
    if (template) {
      template.fields.forEach(field => {
        newFormData[field.name] = ''; // Initialize with empty string or default value
      });
    }
    setFormData(newFormData);
    console.log("New formData after template change:", newFormData);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!selectedTemplate) {
      setError("Please select a channel type.");
      return;
    }
    if (!channelName.trim()) {
        setError("Channel name is required.");
        return;
    }

    setIsSubmitting(true);
    try {
      // Prepare config according to backend expectations for ChannelConfig enum
      // The `type` field is crucial for Serde to deserialize into the correct enum variant.
      // The fields within the config (like botToken, chatId) must match the camelCase renaming
      // if `#[serde(rename_all = "camelCase")]` is applied to the enum variants' fields,
      // or match the original snake_case names if not.
      // Given ChannelConfig has #[serde(tag = "type", rename_all = "camelCase")]
      // and its variants like Telegram { bot_token: String, chat_id: String }
      // Serde will expect "botToken" and "chatId" in the JSON for the Telegram variant.
      
      const preparedConfig: Record<string, unknown> = { type: selectedTemplate.channelType.toLowerCase() };
      for (const key in formData) {
        // Convert snake_case keys from formData (if any) to camelCase for backend
        // Or ensure formData keys are already camelCase if DynamicForm produces them that way.
        // For now, let's assume formData keys are what backend expects for the variant fields (e.g. botToken)
        // If DynamicForm uses 'bot_token', we need to convert here.
        // Let's assume DynamicForm field names match the expected JSON (camelCase)
        preparedConfig[key] = formData[key];
      }
      
      // Correction: The `channelType` from `selectedTemplate.channelType` (e.g., "Telegram")
      // needs to be transformed to the value expected by `#[serde(tag = "type")]` which is likely lowercase.
      // And the fields within `formData` (e.g. `bot_token`) need to be transformed to camelCase `botToken`.

      const finalConfig: Record<string, unknown> = { type: selectedTemplate.channelType.toLowerCase() };
      selectedTemplate.fields.forEach(field => {
        // field.name is likely 'bot_token', 'chat_id', 'url' etc.
        // We need to convert these to camelCase for the final config if the backend expects camelCase variant fields.
        // The ChannelConfig enum itself has rename_all = "camelCase" for its variants (Telegram, Webhook)
        // but the fields *within* those variants (bot_token, chat_id) do not have rename_all.
        // So, the backend will expect "bot_token", "chat_id" as keys within the config, after the "type" tag.
        // The `formData` keys should directly match these (e.g. `bot_token`).
        if (formData[field.name] !== undefined) {
            finalConfig[field.name] = formData[field.name];
        }
      });


      const submissionData: CreateChannelRequest | UpdateChannelRequest = {
        name: channelName,
        channelType: selectedTemplate.channelType, // This is "Telegram" or "Webhook"
        config: finalConfig, // Pass the structured config
      };
      // If editing, we might need to send only updated fields or handle it differently
      // For now, this structure matches CreateChannelRequest.
      // UpdateChannelRequest might need an ID.
      await onSubmit(submissionData);
      onClose(); // Close modal on success
    } catch (err: unknown) {
      if (err instanceof Error) {
        setError(err.message || 'Failed to save channel.');
      } else {
        setError('An unknown error occurred.');
      }
      console.error(err);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) {
    return null;
  }

  return (
    <div className="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex justify-center items-center">
      <div className="relative mx-auto p-5 border w-full max-w-lg shadow-lg rounded-md bg-white">
        <div className="mt-3 text-center">
          <h3 className="text-lg leading-6 font-medium text-gray-900">
            {editingChannel ? 'Edit' : 'Add New'} Notification Channel
          </h3>
          <button
            onClick={onClose}
            className="absolute top-0 right-0 mt-4 mr-4 text-gray-400 hover:text-gray-600"
          >
            <span className="sr-only">Close</span>
            &times;
          </button>
          <div className="mt-2 px-7 py-3">
            {error && <p className="text-red-500 text-sm mb-3">{error}</p>}
            <div className="mb-4">
              <label htmlFor="channelName" className="block text-sm font-medium text-gray-700 text-left">
                Channel Name
                <span className="text-red-500 ml-1">*</span>
              </label>
              <input
                type="text"
                id="channelName"
                name="channelName"
                value={channelName}
                onChange={(e) => setChannelName(e.target.value)}
                required
                className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
              />
            </div>
            {!editingChannel && (
                 <div className="mb-4">
                    <label htmlFor="channelType" className="block text-sm font-medium text-gray-700 text-left">
                        Channel Type
                        <span className="text-red-500 ml-1">*</span>
                    </label>
                    <select
                        id="channelType"
                        name="channelType"
                        value={selectedTemplate?.channelType || ''}
                        onChange={handleTemplateChange}
                        required
                        className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                    >
                        <option value="" disabled>Select a type</option>
                        {templates.map(template => (
                        <option key={template.channelType} value={template.channelType}>
                            {template.name}
                        </option>
                        ))}
                    </select>
                </div>
            )}
           
            {selectedTemplate && (
              <DynamicForm
                fields={selectedTemplate.fields}
                formData={formData}
                onFormChange={handleFormChange}
                onSubmit={handleSubmit}
                isSubmitting={isSubmitting}
                submitButtonText={editingChannel ? 'Save Changes' : 'Create Channel'}
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default NotificationChannelModal;