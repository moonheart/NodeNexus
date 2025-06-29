import React, { useState } from 'react';
import type { ChannelTemplateField } from '../types';
import { Eye, EyeOff } from 'lucide-react';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Button } from '@/components/ui/button';

interface DynamicFormProps {
  fields: ChannelTemplateField[];
  formData: Record<string, unknown>;
  onFormChange: (fieldName: string, value: unknown) => void;
}

const DynamicForm: React.FC<DynamicFormProps> = ({
  fields,
  formData,
  onFormChange,
}) => {
  const [passwordVisibility, setPasswordVisibility] = useState<Record<string, boolean>>({});

  const togglePasswordVisibility = (fieldName: string) => {
    setPasswordVisibility(prev => ({ ...prev, [fieldName]: !prev[fieldName] }));
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { name, value, type } = e.target;
    let processedValue: string | number | boolean = value;

    if (type === 'number') {
      processedValue = Number(value);
    } else if (type === 'checkbox') {
      processedValue = (e.target as HTMLInputElement).checked;
    }
    
    onFormChange(name, processedValue);
  };

  const renderField = (field: ChannelTemplateField) => {
    const value = formData[field.name];

    switch (field.type) {
      case 'textarea':
        return (
          <Textarea
            id={field.name}
            name={field.name}
            value={(value as string) || ''}
            onChange={handleChange}
            required={field.required}
            rows={3}
          />
        );
      case 'password':
        return (
          <div className="relative">
            <Input
              type={passwordVisibility[field.name] ? 'text' : 'password'}
              id={field.name}
              name={field.name}
              value={(value as string) || ''}
              onChange={handleChange}
              required={field.required}
              className="pr-10"
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={() => togglePasswordVisibility(field.name)}
              className="absolute inset-y-0 right-0 h-full px-3"
              aria-label={passwordVisibility[field.name] ? 'Hide password' : 'Show password'}
            >
              {passwordVisibility[field.name] ? <EyeOff className="h-5 w-5" /> : <Eye className="h-5 w-5" />}
            </Button>
          </div>
        );
      case 'number':
        return (
          <Input
            type="number"
            id={field.name}
            name={field.name}
            value={(value as number) || ''}
            onChange={handleChange}
            required={field.required}
          />
        );
      default: // 'text' and any other type
        return (
          <Input
            type="text"
            id={field.name}
            name={field.name}
            value={(value as string) || ''}
            onChange={handleChange}
            required={field.required}
          />
        );
    }
  };

  return (
    <div className="space-y-4">
      {fields.map((field) => (
        <div key={field.name} className="space-y-2">
          <Label htmlFor={field.name}>
            {field.label}
            {field.required && <span className="text-destructive ml-1">*</span>}
          </Label>
          {renderField(field)}
          {field.helpText && <p className="text-sm text-muted-foreground">{field.helpText}</p>}
        </div>
      ))}
    </div>
  );
};

export default DynamicForm;