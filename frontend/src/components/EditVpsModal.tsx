import React, { useEffect, useMemo } from 'react';
import { useForm, Controller, type ControllerRenderProps } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import { updateVps } from '../services/vpsService';
import type { VpsListItemResponse } from '../types';
import axios from 'axios';
import { useServerListStore } from '../store/serverListStore';
import { toast } from 'react-hot-toast';

import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Popover, PopoverTrigger, PopoverContent } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem } from "@/components/ui/command";
import { Check, ChevronsUpDown, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { Badge } from './ui/badge';

const BYTES_IN_KB = 1024;
const BYTES_IN_MB = BYTES_IN_KB * 1024;
const BYTES_IN_GB = BYTES_IN_MB * 1024;
const BYTES_IN_TB = BYTES_IN_GB * 1024;

const bytesToOptimalUnit = (bytes: number): { value: number, unit: string } => {
  if (bytes >= BYTES_IN_TB) return { value: parseFloat((bytes / BYTES_IN_TB).toFixed(2)), unit: 'TB' };
  if (bytes >= BYTES_IN_GB) return { value: parseFloat((bytes / BYTES_IN_GB).toFixed(2)), unit: 'GB' };
  if (bytes >= BYTES_IN_MB) return { value: parseFloat((bytes / BYTES_IN_MB).toFixed(2)), unit: 'MB' };
  return { value: parseFloat((bytes / BYTES_IN_GB).toFixed(2)), unit: 'GB' };
};

const unitToBytes = (value: number, unit: string): number => {
  switch (unit) {
    case 'MB': return Math.round(value * BYTES_IN_MB);
    case 'GB': return Math.round(value * BYTES_IN_GB);
    case 'TB': return Math.round(value * BYTES_IN_TB);
    default: return 0;
  }
};

// Zod Schema for validation
const formSchema = z.object({
  name: z.string().min(1, "Name is required"),
  group: z.string().optional().nullable(),
  tagIds: z.array(z.number()).optional(),
  
  trafficLimitInput: z.string().optional(),
  trafficLimitUnit: z.string().optional(),
  trafficBillingRule: z.string().optional().nullable(),
  trafficResetConfigType: z.string().optional().nullable(),
  trafficResetConfigValue: z.string().optional().nullable(),
  nextTrafficResetAt: z.string().optional().nullable(),

  renewalCycle: z.string().optional().nullable(),
  renewalCycleCustomDays: z.string().optional().nullable(),
  renewalPrice: z.string().optional().nullable(),
  renewalCurrency: z.string().optional().nullable(),
  nextRenewalDate: z.string().optional().nullable(),
  lastRenewalDate: z.string().optional().nullable(),
  serviceStartDate: z.string().optional().nullable(),
  paymentMethod: z.string().optional().nullable(),
  autoRenewEnabled: z.boolean().optional(),
  renewalNotes: z.string().optional().nullable(),
}).refine(data => {
    if (data.trafficLimitInput && data.trafficLimitInput.trim() !== '') {
        return !!data.trafficBillingRule && !!data.trafficResetConfigType;
    }
    return true;
}, {
    message: "When traffic limit is set, billing rule and reset type are required.",
    path: ["trafficBillingRule"], // you can point to a specific field
});


type FormValues = z.infer<typeof formSchema>;

interface EditVpsModalProps {
  isOpen: boolean;
  onClose: () => void;
  vps: VpsListItemResponse | null;
  allVps: VpsListItemResponse[];
  onVpsUpdated: () => void;
}

const EditVpsModal: React.FC<EditVpsModalProps> = ({ isOpen, onClose, vps, allVps, onVpsUpdated }) => {
  const allTags = useServerListStore((state) => state.allTags);
  const fetchAllTags = useServerListStore((state) => state.fetchAllTags);

  const form = useForm<FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {},
  });

  const { handleSubmit, control, reset, watch, setValue } = form;

  const groupOptions = useMemo(() => {
    const allGroups = new Set(allVps.map(v => v.group).filter((g): g is string => !!g));
    return [...allGroups].map(g => ({ value: g, label: g }));
  }, [allVps]);

  const tagOptions = useMemo(() => {
    return allTags.map(tag => ({ id: tag.id, name: tag.name, color: tag.color }));
  }, [allTags]);

  useEffect(() => {
    if (isOpen) {
      fetchAllTags();
      if (vps) {
        const { value: trafficValue, unit: trafficUnit } = vps.trafficLimitBytes ? bytesToOptimalUnit(vps.trafficLimitBytes) : { value: '', unit: 'GB' };
        reset({
          name: vps.name || '',
          group: vps.group || null,
          tagIds: vps.tags ? vps.tags.map(t => t.id) : [],
          trafficLimitInput: trafficValue.toString(),
          trafficLimitUnit: trafficUnit,
          trafficBillingRule: vps.trafficBillingRule || null,
          trafficResetConfigType: vps.trafficResetConfigType || null,
          trafficResetConfigValue: vps.trafficResetConfigValue || null,
          nextTrafficResetAt: vps.nextTrafficResetAt ? vps.nextTrafficResetAt.substring(0, 16) : null,
          renewalCycle: vps.renewalCycle || null,
          renewalCycleCustomDays: vps.renewalCycleCustomDays?.toString() || null,
          renewalPrice: vps.renewalPrice?.toString() || null,
          renewalCurrency: vps.renewalCurrency || null,
          nextRenewalDate: vps.nextRenewalDate ? vps.nextRenewalDate.substring(0, 16) : null,
          lastRenewalDate: vps.lastRenewalDate ? vps.lastRenewalDate.substring(0, 16) : null,
          serviceStartDate: vps.serviceStartDate ? vps.serviceStartDate.substring(0, 16) : null,
          paymentMethod: vps.paymentMethod || null,
          autoRenewEnabled: vps.autoRenewEnabled || false,
          renewalNotes: vps.renewalNotes || null,
        });
      }
    }
  }, [isOpen, vps, reset, fetchAllTags]);
  
  const trafficResetConfigType = watch('trafficResetConfigType');
  const nextTrafficResetAt = watch('nextTrafficResetAt');

  useEffect(() => {
    if (trafficResetConfigType === 'monthly_day_of_month' && nextTrafficResetAt) {
      try {
        const date = new Date(nextTrafficResetAt);
        const day = date.getDate();
        const timeOffsetSeconds = date.getHours() * 3600 + date.getMinutes() * 60 + date.getSeconds();
        setValue('trafficResetConfigValue', `day:${day},time_offset_seconds:${timeOffsetSeconds}`);
      } catch (e) {
        console.error("Error parsing nextTrafficResetAt for config value:", e);
      }
    }
  }, [nextTrafficResetAt, trafficResetConfigType, setValue]);


  const onSubmit = async (data: FormValues) => {
    if (!vps) return;

    const payload = {
      name: data.name.trim(),
      group: data.group || undefined,
      tagIds: data.tagIds,
      trafficLimitBytes: data.trafficLimitInput ? unitToBytes(parseFloat(data.trafficLimitInput), data.trafficLimitUnit || 'GB') : null,
      trafficBillingRule: data.trafficBillingRule || undefined,
      trafficResetConfigType: data.trafficResetConfigType || undefined,
      trafficResetConfigValue: data.trafficResetConfigValue || undefined,
      nextTrafficResetAt: data.nextTrafficResetAt ? new Date(data.nextTrafficResetAt).toISOString() : null,
      renewalCycle: data.renewalCycle || undefined,
      renewalCycleCustomDays: data.renewalCycleCustomDays ? parseInt(data.renewalCycleCustomDays, 10) : null,
      renewalPrice: data.renewalPrice ? parseFloat(data.renewalPrice) : null,
      renewalCurrency: data.renewalCurrency || undefined,
      nextRenewalDate: data.nextRenewalDate ? new Date(data.nextRenewalDate).toISOString() : null,
      lastRenewalDate: data.lastRenewalDate ? new Date(data.lastRenewalDate).toISOString() : null,
      serviceStartDate: data.serviceStartDate ? new Date(data.serviceStartDate).toISOString() : null,
      paymentMethod: data.paymentMethod || undefined,
      autoRenewEnabled: data.autoRenewEnabled,
      renewalNotes: data.renewalNotes || undefined,
    };

    try {
      await updateVps(vps.id, payload);
      toast.success('VPS updated successfully!');
      onVpsUpdated();
      onClose();
    } catch (err: unknown) {
      console.error('Failed to update VPS:', err);
      let errorMessage = 'Failed to update VPS. Please try again.';
      if (axios.isAxiosError(err) && err.response?.data?.error) {
        errorMessage = err.response.data.error;
      } else if (err instanceof Error) {
        errorMessage = err.message;
      }
      toast.error(errorMessage);
    }
  };

  const CreatableCombobox = ({ field, options, placeholder }: { field: ControllerRenderProps<FormValues, 'group' | 'renewalCurrency' | 'paymentMethod'>, options: {value: string, label: string}[], placeholder: string }) => {
    const [open, setOpen] = React.useState(false);
    const [localOptions, setLocalOptions] = React.useState(options);
    const [inputValue, setInputValue] = React.useState("");

    const handleSelect = (currentValue: string) => {
        field.onChange(currentValue === field.value ? "" : currentValue);
        setOpen(false);
    };
    
    const handleCreate = () => {
        if (inputValue && !localOptions.some(opt => opt.value.toLowerCase() === inputValue.toLowerCase())) {
            const newOption = { value: inputValue, label: inputValue };
            setLocalOptions(prev => [...prev, newOption]);
            field.onChange(inputValue);
            setOpen(false);
        }
    };

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
                <Button variant="outline" role="combobox" aria-expanded={open} className="w-full justify-between">
                    {field.value ? localOptions.find(option => option.value === field.value)?.label : placeholder}
                    <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
                </Button>
            </PopoverTrigger>
            <PopoverContent className="w-[--radix-popover-trigger-width] p-0">
                <Command>
                    <CommandInput 
                        placeholder="Search or create..." 
                        value={inputValue}
                        onValueChange={setInputValue}
                    />
                    <CommandEmpty>
                        <Button variant="ghost" className="w-full" onClick={handleCreate}>
                            Create "{inputValue}"
                        </Button>
                    </CommandEmpty>
                    <CommandGroup>
                        {localOptions.map((option) => (
                            <CommandItem key={option.value} onSelect={() => handleSelect(option.value)}>
                                <Check className={cn("mr-2 h-4 w-4", field.value === option.value ? "opacity-100" : "opacity-0")} />
                                {option.label}
                            </CommandItem>
                        ))}
                    </CommandGroup>
                </Command>
            </PopoverContent>
        </Popover>
    );
  };

  const MultiSelectPopover = ({ field, options, placeholder }: { field: ControllerRenderProps<FormValues, 'tagIds'>, options: {id: number, name: string, color: string}[], placeholder: string }) => {
    return (
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="outline" className="w-full justify-between">
            <span className="truncate">
              {(field.value || []).length > 0 ? `${(field.value || []).length} tag(s) selected` : placeholder}
            </span>
            <ChevronDown className="h-4 w-4 ml-2" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-[--radix-popover-trigger-width] p-0">
          <ScrollArea className="h-48">
            <div className="p-4 space-y-2">
              {options.map(tag => (
                <div key={tag.id} className="flex items-center space-x-2">
                  <Checkbox
                    id={`tag-${tag.id}`}
                    checked={field.value?.includes(tag.id)}
                    onCheckedChange={(checked) => {
                      const newValue = checked
                        ? [...(field.value || []), tag.id]
                        : (field.value || []).filter((id: number) => id !== tag.id);
                      field.onChange(newValue);
                    }}
                  />
                  <Label htmlFor={`tag-${tag.id}`} className="flex-grow">
                    <Badge style={{ backgroundColor: tag.color, color: '#fff' }}>{tag.name}</Badge>
                  </Label>
                </div>
              ))}
            </div>
          </ScrollArea>
        </PopoverContent>
      </Popover>
    );
  };

  if (!isOpen || !vps) return null;

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Edit Server Information</DialogTitle>
          <DialogDescription>Make changes to your VPS configuration. Click save when you're done.</DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit(onSubmit)}>
          <Tabs defaultValue="basic" className="w-full">
            <TabsList className="grid w-full grid-cols-3">
              <TabsTrigger value="basic">Basic Info</TabsTrigger>
              <TabsTrigger value="traffic">Traffic Monitoring</TabsTrigger>
              <TabsTrigger value="renewal">Renewal Settings</TabsTrigger>
            </TabsList>
            
            <ScrollArea className="h-[500px] p-1">
            <div className="p-4">
            <TabsContent value="basic">
              <div className="space-y-4">
                <div>
                  <Label htmlFor="name">Name</Label>
                  <Controller name="name" control={control} render={({ field }) => <Input id="name" {...field} />} />
                </div>
                <div>
                  <Label>Group</Label>
                  <Controller name="group" control={control} render={({ field }) => <CreatableCombobox field={field} options={groupOptions} placeholder="Select or create a group..." />} />
                </div>
                <div>
                  <Label>Tags</Label>
                  <Controller name="tagIds" control={control} render={({ field }) => <MultiSelectPopover field={field} options={tagOptions} placeholder="Select tags..." />} />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="traffic">
              <div className="space-y-4">
                <div>
                  <Label>Traffic Limit</Label>
                  <div className="flex space-x-2">
                    <Controller name="trafficLimitInput" control={control} render={({ field }) => <Input type="number" placeholder="e.g., 100" {...field} value={field.value || ''} />} />
                    <Controller name="trafficLimitUnit" control={control} render={({ field }) => (
                      <Select onValueChange={field.onChange} defaultValue={field.value || 'GB'}>
                        <SelectTrigger><SelectValue /></SelectTrigger>
                        <SelectContent>
                          <SelectItem value="MB">MB</SelectItem>
                          <SelectItem value="GB">GB</SelectItem>
                          <SelectItem value="TB">TB</SelectItem>
                        </SelectContent>
                      </Select>
                    )} />
                  </div>
                </div>
                <div>
                  <Label>Traffic Billing Rule</Label>
                  <Controller name="trafficBillingRule" control={control} render={({ field }) => (
                    <Select onValueChange={field.onChange} value={field.value || ''}>
                      <SelectTrigger><SelectValue placeholder="Select a rule..." /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="sum_in_out">Sum IN+OUT</SelectItem>
                        <SelectItem value="out_only">OUT only</SelectItem>
                        <SelectItem value="max_in_out">Max(IN, OUT)</SelectItem>
                      </SelectContent>
                    </Select>
                  )} />
                </div>
                <div>
                  <Label>Traffic Reset Rule Type</Label>
                  <Controller name="trafficResetConfigType" control={control} render={({ field }) => (
                     <Select onValueChange={field.onChange} value={field.value || ''}>
                      <SelectTrigger><SelectValue placeholder="Select a type..." /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="monthly_day_of_month">Monthly by day</SelectItem>
                        <SelectItem value="fixed_days">Fixed day interval</SelectItem>
                      </SelectContent>
                    </Select>
                  )} />
                </div>
                {watch('trafficResetConfigType') && (
                  <div>
                    <Label>Reset Rule Value</Label>
                    <Controller name="trafficResetConfigValue" control={control} render={({ field }) => <Input {...field} value={field.value || ''} readOnly={watch('trafficResetConfigType') === 'monthly_day_of_month'} />} />
                  </div>
                )}
                <div>
                  <Label>Next Reset Date</Label>
                  <Controller name="nextTrafficResetAt" control={control} render={({ field }) => <Input type="datetime-local" {...field} value={field.value || ''} />} />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="renewal">
              <div className="space-y-4">
                <div>
                  <Label>Renewal Cycle</Label>
                  <Controller name="renewalCycle" control={control} render={({ field }) => (
                    <Select onValueChange={field.onChange} value={field.value || ''}>
                      <SelectTrigger><SelectValue placeholder="Select a cycle..." /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="monthly">Monthly</SelectItem>
                        <SelectItem value="quarterly">Quarterly</SelectItem>
                        <SelectItem value="semi_annually">Semi-Annually</SelectItem>
                        <SelectItem value="annually">Annually</SelectItem>
                        <SelectItem value="biennially">Biennially</SelectItem>
                        <SelectItem value="triennially">Triennially</SelectItem>
                        <SelectItem value="custom_days">Custom Days</SelectItem>
                      </SelectContent>
                    </Select>
                  )} />
                </div>
                {watch('renewalCycle') === 'custom_days' && (
                  <div>
                    <Label>Custom Cycle Days</Label>
                    <Controller name="renewalCycleCustomDays" control={control} render={({ field }) => <Input type="number" {...field} value={field.value || ''} />} />
                  </div>
                )}
                <div className="grid grid-cols-2 gap-4">
                    <div>
                        <Label>Renewal Price</Label>
                        <Controller name="renewalPrice" control={control} render={({ field }) => <Input type="number" step="0.01" {...field} value={field.value || ''} />} />
                    </div>
                    <div>
                        <Label>Currency</Label>
                        <Controller name="renewalCurrency" control={control} render={({ field }) => <CreatableCombobox field={field} options={[{value: 'USD', label: 'USD'}, {value: 'CNY', label: 'CNY'}]} placeholder="Select currency..." />} />
                    </div>
                </div>
                <div>
                  <Label>Service Start Date</Label>
                  <Controller name="serviceStartDate" control={control} render={({ field }) => <Input type="datetime-local" {...field} value={field.value || ''} />} />
                </div>
                <div>
                  <Label>Last Renewal Date</Label>
                  <Controller name="lastRenewalDate" control={control} render={({ field }) => <Input type="datetime-local" {...field} value={field.value || ''} />} />
                </div>
                <div>
                  <Label>Next Renewal Date</Label>
                  <Controller name="nextRenewalDate" control={control} render={({ field }) => <Input type="datetime-local" {...field} value={field.value || ''} />} />
                </div>
                <div>
                  <Label>Payment Method</Label>
                  <Controller name="paymentMethod" control={control} render={({ field }) => <CreatableCombobox field={field} options={[
                    { value: 'PayPal', label: 'PayPal' },
                    { value: 'Alipay', label: 'Alipay (支付宝)' },
                    { value: 'Credit Card', label: 'Credit Card (信用卡)' },
                    { value: 'Stripe', label: 'Stripe' },
                    { value: 'Bank Transfer', label: 'Bank Transfer (银行转账)' },
                    { value: 'WeChat Pay', label: 'WeChat Pay (微信支付)' },
                    { value: 'UnionPay', label: 'UnionPay (银联)' },
                    { value: 'Apple Pay', label: 'Apple Pay' },
                    { value: 'Google Pay', label: 'Google Pay' }
                    ]} placeholder="Select payment method..." />} />
                </div>
                <div>
                  <Label>Renewal Notes</Label>
                  <Controller name="renewalNotes" control={control} render={({ field }) => <Textarea {...field} value={field.value || ''} />} />
                </div>
                <div className="flex items-center space-x-2">
                  <Controller name="autoRenewEnabled" control={control} render={({ field }) => <Checkbox id="autoRenewEnabled" checked={field.value} onCheckedChange={field.onChange} />} />
                  <Label htmlFor="autoRenewEnabled">Enable Auto-Renewal</Label>
                </div>
              </div>
            </TabsContent>
            </div>
            </ScrollArea>
          </Tabs>
          <DialogFooter className="pt-4">
            <Button type="button" variant="outline" onClick={onClose}>Cancel</Button>
            <Button type="submit" disabled={form.formState.isSubmitting}>
              {form.formState.isSubmitting ? 'Saving...' : 'Save Changes'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default EditVpsModal;
