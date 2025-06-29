import React, { useEffect } from 'react';
import { useForm, Controller, type ControllerRenderProps } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import type { ServiceMonitor, ServiceMonitorInput, Tag, VpsListItemResponse } from '../types';
import { getAllVpsListItems } from '../services/vpsService';
import { getTags } from '../services/tagService';
import toast from 'react-hot-toast';

import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Popover, PopoverTrigger, PopoverContent } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ChevronDown } from 'lucide-react';
import { Badge } from './ui/badge';
import { Switch } from './ui/switch';

const httpMonitorConfigSchema = z.object({
  expected_status_codes: z.array(z.number()).optional(),
  response_body_match: z.string().optional(),
}).optional();

const formSchema = z.object({
  name: z.string().min(1, "Name is required"),
  monitorType: z.enum(['http', 'ping', 'tcp']),
  target: z.string().min(1, "Target is required"),
  frequencySeconds: z.number().min(10),
  timeoutSeconds: z.number().min(1),
  isActive: z.boolean(),
  monitorConfig: httpMonitorConfigSchema,
  assignments: z.object({
    agentIds: z.array(z.number()),
    tagIds: z.array(z.number()),
    assignmentType: z.enum(['INCLUSIVE', 'EXCLUSIVE']),
  }),
});

type FormValues = z.infer<typeof formSchema>;

interface ServiceMonitorModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (monitor: ServiceMonitorInput, id?: number) => void;
  monitorToEdit?: ServiceMonitor | null;
}

const ServiceMonitorModal: React.FC<ServiceMonitorModalProps> = ({ isOpen, onClose, onSave, monitorToEdit }) => {
  const [allAgents, setAllAgents] = React.useState<VpsListItemResponse[]>([]);
  const [allTags, setAllTags] = React.useState<Tag[]>([]);

  const form = useForm<FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {
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
    },
  });

  const { handleSubmit, control, reset, watch } = form;

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [agentsResponse, tags] = await Promise.all([getAllVpsListItems(), getTags()]);
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
    if (isOpen) {
      if (monitorToEdit) {
        const initialMonitorConfig =
          monitorToEdit.monitorType === 'http' &&
          monitorToEdit.monitorConfig &&
          'expected_status_codes' in monitorToEdit.monitorConfig
            ? {
                expected_status_codes: monitorToEdit.monitorConfig.expected_status_codes,
                response_body_match: 'response_body_match' in monitorToEdit.monitorConfig ? monitorToEdit.monitorConfig.response_body_match : undefined,
              }
            : {};
        
        reset({
          name: monitorToEdit.name,
          monitorType: monitorToEdit.monitorType,
          target: monitorToEdit.target,
          frequencySeconds: monitorToEdit.frequencySeconds,
          timeoutSeconds: monitorToEdit.timeoutSeconds,
          isActive: monitorToEdit.isActive,
          monitorConfig: initialMonitorConfig,
          assignments: {
            agentIds: monitorToEdit.agentIds || [],
            tagIds: monitorToEdit.tagIds || [],
            assignmentType: monitorToEdit.assignmentType || 'INCLUSIVE',
          },
        });
      } else {
        // Reset to default for creation
        form.reset();
      }
    }
  }, [monitorToEdit, isOpen, reset, form]);

  const onSubmit = (data: FormValues) => {
    const monitorInput: ServiceMonitorInput = {
        ...data,
        // Handle any transformations if necessary, e.g., for monitorConfig
        monitorConfig: data.monitorType === 'http' ? data.monitorConfig : {},
    };
    onSave(monitorInput, monitorToEdit?.id);
  };
  
  const MultiSelectPopover = ({ field, options, placeholder }: { field: ControllerRenderProps<FormValues, 'assignments.agentIds' | 'assignments.tagIds'>, options: (VpsListItemResponse | Tag)[], placeholder: string }) => (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="outline" className="w-full justify-between">
          <span className="truncate">
            {(field.value || []).length > 0 ? `${(field.value || []).length} selected` : placeholder}
          </span>
          <ChevronDown className="h-4 w-4 ml-2" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[--radix-popover-trigger-width] p-0">
        <ScrollArea className="h-48">
          <div className="p-4 space-y-2">
            {options.map(option => (
              <div key={option.id} className="flex items-center space-x-2">
                <Checkbox
                  id={`option-${option.id}`}
                  checked={field.value?.includes(option.id)}
                  onCheckedChange={(checked) => {
                    const newValue = checked
                      ? [...(field.value || []), option.id]
                      : (field.value || []).filter((id: number) => id !== option.id);
                    field.onChange(newValue);
                  }}
                />
                <Label htmlFor={`option-${option.id}`} className="flex-grow">
                  {'color' in option && option.color ? <Badge style={{ backgroundColor: option.color, color: '#fff' }}>{option.name}</Badge> : option.name}
                </Label>
              </div>
            ))}
          </div>
        </ScrollArea>
      </PopoverContent>
    </Popover>
  );

  if (!isOpen) return null;

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>{monitorToEdit ? 'Edit' : 'Create'} Service Monitor</DialogTitle>
          <DialogDescription>Configure the details for your service monitor.</DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit(onSubmit)}>
          <ScrollArea className="h-[70vh] p-1">
            <div className="p-4 space-y-6">
              {/* Basic Info */}
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                    <Label>Is Active</Label>
                    <Controller name="isActive" control={control} render={({ field }) => <Switch checked={field.value} onCheckedChange={field.onChange} />} />
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <Label htmlFor="name">Monitor Name</Label>
                    <Controller name="name" control={control} render={({ field }) => <Input id="name" {...field} />} />
                  </div>
                  <div>
                    <Label>Monitor Type</Label>
                    <Controller name="monitorType" control={control} render={({ field }) => (
                      <Select onValueChange={field.onChange} value={field.value}>
                        <SelectTrigger><SelectValue /></SelectTrigger>
                        <SelectContent>
                          <SelectItem value="http">HTTP(s)</SelectItem>
                          <SelectItem value="ping">Ping</SelectItem>
                          <SelectItem value="tcp">TCP Port</SelectItem>
                        </SelectContent>
                      </Select>
                    )} />
                  </div>
                </div>
                <div>
                  <Label htmlFor="target">Target</Label>
                  <Controller name="target" control={control} render={({ field }) => <Input id="target" placeholder="e.g., https://example.com or 8.8.8.8:53" {...field} />} />
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <Label htmlFor="frequencySeconds">Frequency (seconds)</Label>
                    <Controller name="frequencySeconds" control={control} render={({ field }) => <Input id="frequencySeconds" type="number" {...field} onChange={e => field.onChange(parseInt(e.target.value, 10))} />} />
                  </div>
                  <div>
                    <Label htmlFor="timeoutSeconds">Timeout (seconds)</Label>
                    <Controller name="timeoutSeconds" control={control} render={({ field }) => <Input id="timeoutSeconds" type="number" {...field} onChange={e => field.onChange(parseInt(e.target.value, 10))} />} />
                  </div>
                </div>
              </div>

              {/* Dynamic Config Section */}
              {watch('monitorType') === 'http' && (
                <div className="space-y-4 p-4 border rounded-md bg-slate-50">
                    <h3 className="text-lg font-medium text-slate-900">HTTP Options</h3>
                    <div>
                        <Label>Expected Status Codes</Label>
                        <Controller name="monitorConfig.expected_status_codes" control={control} render={({ field }) => <Input placeholder="e.g., 200, 201" {...field} value={Array.isArray(field.value) ? field.value.join(', ') : ''} onChange={e => field.onChange(e.target.value.split(',').map(s => parseInt(s.trim(), 10)).filter(n => !isNaN(n)))} />} />
                    </div>
                    <div>
                        <Label>Response Body Match</Label>
                        <Controller name="monitorConfig.response_body_match" control={control} render={({ field }) => <Input placeholder="Text to find in response body" {...field} value={field.value || ''} />} />
                    </div>
                </div>
              )}

              {/* Assignments */}
              <div className="p-4 border rounded-md bg-slate-50 space-y-4">
                <h3 className="text-lg font-medium">Assignments</h3>
                <Controller name="assignments.assignmentType" control={control} render={({ field }) => (
                  <RadioGroup onValueChange={field.onChange} value={field.value} className="flex items-center space-x-4">
                    <div className="flex items-center space-x-2">
                      <RadioGroupItem value="INCLUSIVE" id="inclusive" />
                      <Label htmlFor="inclusive">Inclusive (Apply to selected)</Label>
                    </div>
                    <div className="flex items-center space-x-2">
                      <RadioGroupItem value="EXCLUSIVE" id="exclusive" />
                      <Label htmlFor="exclusive">Exclusive (Apply to all except selected)</Label>
                    </div>
                  </RadioGroup>
                )} />
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <Label>{watch('assignments.assignmentType') === 'EXCLUSIVE' ? 'Exclude Agents' : 'Assign to Agents'}</Label>
                    <Controller name="assignments.agentIds" control={control} render={({ field }) => <MultiSelectPopover field={field} options={allAgents} placeholder="Select agents..." />} />
                  </div>
                  <div>
                    <Label>{watch('assignments.assignmentType') === 'EXCLUSIVE' ? 'Exclude Tags' : 'Assign to Tags'}</Label>
                    <Controller name="assignments.tagIds" control={control} render={({ field }) => <MultiSelectPopover field={field} options={allTags} placeholder="Select tags..." />} />
                  </div>
                </div>
              </div>
            </div>
          </ScrollArea>
          <DialogFooter className="pt-4">
            <Button type="button" variant="outline" onClick={onClose}>Cancel</Button>
            <Button type="submit" disabled={form.formState.isSubmitting}>
              {form.formState.isSubmitting ? 'Saving...' : 'Save'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default ServiceMonitorModal;