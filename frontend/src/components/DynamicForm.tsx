import React from 'react';
import type { ChannelTemplateField } from '../types';

interface DynamicFormProps {
  fields: ChannelTemplateField[];
  formData: Record<string, unknown>;
  onFormChange: (fieldName: string, value: unknown) => void;
  onSubmit: (e: React.FormEvent) => void;
  isSubmitting: boolean;
  submitButtonText?: string;
}

const DynamicForm: React.FC<DynamicFormProps> = ({
  fields,
  formData,
  onFormChange,
  onSubmit,
  isSubmitting,
  submitButtonText = 'Submit',
}) => {
  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) => {
    const { name, value, type } = e.target;
    let processedValue: string | number | boolean = value;

    if (type === 'number') {
      processedValue = Number(value);
    } else if (type === 'checkbox') {
      // Assuming checkbox is an HTMLInputElement
      processedValue = (e.target as HTMLInputElement).checked;
    }
    // Add other type coercions if necessary, e.g., for select multiple

    onFormChange(name, processedValue);
  };

  return (
    <form onSubmit={onSubmit} className="space-y-6">
      {fields.map((field) => (
        <div key={field.name}>
          <label htmlFor={field.name} className="block text-sm font-medium text-gray-700">
            {field.label}
            {field.required && <span className="text-red-500 ml-1">*</span>}
          </label>
          {field.type === 'textarea' ? (
            <textarea
              id={field.name}
              name={field.name}
              value={(formData[field.name] as string) || ''}
              onChange={handleChange}
              required={field.required}
              className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
              rows={3}
            />
          ) : field.type === 'password' ? (
            <input
              type="password"
              id={field.name}
              name={field.name}
              value={(formData[field.name] as string) || ''}
              onChange={handleChange}
              required={field.required}
              className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
            />
          ) : field.type === 'number' ? (
             <input
              type="number"
              id={field.name}
              name={field.name}
              value={(formData[field.name] as number) || ''}
              onChange={handleChange}
              required={field.required}
              className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
            />
          ) : ( // Default to text input
            <input
              type="text"
              id={field.name}
              name={field.name}
              value={(formData[field.name] as string) || ''}
              onChange={handleChange}
              required={field.required}
              className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
            />
          )}
          {field.helpText && <p className="mt-2 text-sm text-gray-500">{field.helpText}</p>}
        </div>
      ))}
      <button
        type="submit"
        disabled={isSubmitting}
        className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:bg-indigo-300"
      >
        {isSubmitting ? 'Submitting...' : submitButtonText}
      </button>
    </form>
  );
};

export default DynamicForm;