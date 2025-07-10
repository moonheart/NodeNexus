import React, { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useForm, Controller, type ControllerRenderProps } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import { updateVps } from '../services/vpsService';
import type { VpsListItemResponse } from '../types';
import axios from 'axios';
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
import { DateTimePicker } from './date-time-picker';

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

// 计算下次续费日期的工具函数
const calculateNextRenewalDate = (
  startDate: Date,
  cycle: string,
  customDays?: string | null
): Date | null => {
  if (!startDate || !cycle) return null;
  
  const nextDate = new Date(startDate);
  
  switch (cycle) {
    case 'monthly':
      nextDate.setMonth(nextDate.getMonth() + 1);
      return nextDate;
    case 'quarterly':
      nextDate.setMonth(nextDate.getMonth() + 3);
      return nextDate;
    case 'semi_annually':
      nextDate.setMonth(nextDate.getMonth() + 6);
      return nextDate;
    case 'annually':
      nextDate.setFullYear(nextDate.getFullYear() + 1);
      return nextDate;
    case 'biennially':
      nextDate.setFullYear(nextDate.getFullYear() + 2);
      return nextDate;
    case 'triennially':
      nextDate.setFullYear(nextDate.getFullYear() + 3);
      return nextDate;
    case 'custom_days':
      if (customDays && !isNaN(parseInt(customDays, 10))) {
        nextDate.setDate(nextDate.getDate() + parseInt(customDays, 10));
        return nextDate;
      }
      return null;
    default:
      return null;
  }
};

const getFormSchema = (t: (key: string) => string) => z.object({
  name: z.string().min(1, t('common.errors.validation.nameRequired')),
  group: z.string().optional().nullable(),
  tagIds: z.array(z.number()).optional(),
  
  trafficLimitInput: z.string().optional(),
  trafficLimitUnit: z.string().optional(),
  trafficBillingRule: z.string().optional().nullable(),
  trafficResetConfigType: z.string().optional().nullable(),
  trafficResetConfigValue: z.string().optional().nullable(),
  nextTrafficResetAt: z.date().optional().nullable(),

  renewalCycle: z.string().optional().nullable(),
  renewalCycleCustomDays: z.string().optional().nullable(),
  renewalPrice: z.string().optional().nullable(),
  renewalCurrency: z.string().optional().nullable(),
  nextRenewalDate: z.date().optional().nullable(),
  lastRenewalDate: z.date().optional().nullable(),
  serviceStartDate: z.date().optional().nullable(),
  paymentMethod: z.string().optional().nullable(),
  autoRenewEnabled: z.boolean().optional(),
  renewalNotes: z.string().optional().nullable(),
}).refine(data => {
    if (data.trafficLimitInput && data.trafficLimitInput.trim() !== '') {
        return !!data.trafficBillingRule && !!data.trafficResetConfigType;
    }
    return true;
}, {
    message: "When traffic limit is set, billing rule and reset type are required.", // This message is not shown to user, so no need to translate
    path: ["trafficBillingRule"],
});


type FormValues = z.infer<ReturnType<typeof getFormSchema>>;
type TagOption = { id: number; name: string; color: string };

interface EditVpsModalProps {
  isOpen: boolean;
  onClose: () => void;
  vps: VpsListItemResponse | null;
  groupOptions: { value: string; label: string }[];
  tagOptions: TagOption[];
  onVpsUpdated: () => void;
}

const EditVpsModal: React.FC<EditVpsModalProps> = ({ isOpen, onClose, vps, groupOptions, tagOptions, onVpsUpdated }) => {
  const { t } = useTranslation();
  const formSchema = useMemo(() => getFormSchema(t), [t]);

  const form = useForm<FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {},
  });

  const { handleSubmit, control, reset, watch, setValue } = form;
  
  // 追踪用户是否手动修改了日期字段
  const userModifiedLastRenewalDate = React.useRef(false);
  const userModifiedNextRenewalDate = React.useRef(false);
  
  // 高亮状态管理
  const [highlightedFields, setHighlightedFields] = useState<{
    lastRenewalDate: boolean;
    nextRenewalDate: boolean;
  }>({
    lastRenewalDate: false,
    nextRenewalDate: false
  });
  
  // 高亮辅助函数
  const highlightField = (fieldName: 'lastRenewalDate' | 'nextRenewalDate') => {
    setHighlightedFields(prev => ({
      ...prev,
      [fieldName]: true
    }));
    
    // 2秒后取消高亮
    setTimeout(() => {
      setHighlightedFields(prev => ({
        ...prev,
        [fieldName]: false
      }));
    }, 2000);
  };

  useEffect(() => {
    if (isOpen && vps) {
      // 重置用户修改状态
      userModifiedLastRenewalDate.current = false;
      userModifiedNextRenewalDate.current = false;
      
      // 重置高亮状态
      setHighlightedFields({
        lastRenewalDate: false,
        nextRenewalDate: false
      });
      
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
        nextTrafficResetAt: vps.nextTrafficResetAt ? new Date(vps.nextTrafficResetAt) : null,
        renewalCycle: vps.renewalCycle || null,
        renewalCycleCustomDays: vps.renewalCycleCustomDays?.toString() || null,
        renewalPrice: vps.renewalPrice?.toString() || null,
        renewalCurrency: vps.renewalCurrency || null,
        nextRenewalDate: vps.nextRenewalDate ? new Date(vps.nextRenewalDate) : null,
        lastRenewalDate: vps.lastRenewalDate ? new Date(vps.lastRenewalDate) : null,
        serviceStartDate: vps.serviceStartDate ? new Date(vps.serviceStartDate) : null,
        paymentMethod: vps.paymentMethod || 'PayPal',
        autoRenewEnabled: vps.autoRenewEnabled || false,
        renewalNotes: vps.renewalNotes || null,
      });
    }
  }, [isOpen, vps, reset]);
  
  // Watch for changes that trigger automatic date calculations
  const watchedServiceStartDate = watch('serviceStartDate');
  const watchedLastRenewalDate = watch('lastRenewalDate');
  const watchedRenewalCycle = watch('renewalCycle');
  const watchedRenewalCycleCustomDays = watch('renewalCycleCustomDays');

  // Effect 1: Auto-fill Last Renewal Date from Service Start Date
  useEffect(() => {
    // Only fill if service start date is set and last renewal was not manually changed.
    if (watchedServiceStartDate && !userModifiedLastRenewalDate.current) {
      setValue('lastRenewalDate', new Date(watchedServiceStartDate));
      highlightField('lastRenewalDate');
    }
  }, [watchedServiceStartDate, setValue]);

  // Effect 2: Auto-fill Next Renewal Date from Last Renewal Date and Cycle
  useEffect(() => {
    // Only fill if we have a base date and a cycle, and next renewal was not manually changed.
    if (watchedLastRenewalDate && watchedRenewalCycle && !userModifiedNextRenewalDate.current) {
      const nextDate = calculateNextRenewalDate(
        new Date(watchedLastRenewalDate),
        watchedRenewalCycle,
        watchedRenewalCycleCustomDays
      );
      
      if (nextDate) {
        const currentNextDateVal = watch('nextRenewalDate');
        // Set value only if it's different to avoid re-renders/loops
        if (!currentNextDateVal || new Date(currentNextDateVal).getTime() !== nextDate.getTime()) {
          setValue('nextRenewalDate', nextDate);
          highlightField('nextRenewalDate');
        }
      }
    }
  }, [watchedLastRenewalDate, watchedRenewalCycle, watchedRenewalCycleCustomDays, setValue]);

  const trafficResetConfigType = watch('trafficResetConfigType');
  const nextTrafficResetAt = watch('nextTrafficResetAt');

  useEffect(() => {
    if (trafficResetConfigType === 'monthly_day_of_month' && nextTrafficResetAt) {
      if (nextTrafficResetAt) {
        try {
          const date = new Date(nextTrafficResetAt);
          const day = date.getDate();
          const timeOffsetSeconds = date.getHours() * 3600 + date.getMinutes() * 60 + date.getSeconds();
          setValue('trafficResetConfigValue', `day:${day},time_offset_seconds:${timeOffsetSeconds}`);
        } catch (e) {
          console.error("Error parsing nextTrafficResetAt for config value:", e);
        }
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
      nextTrafficResetAt: data.nextTrafficResetAt ? data.nextTrafficResetAt.toISOString() : null,
      renewalCycle: data.renewalCycle || undefined,
      renewalCycleCustomDays: data.renewalCycleCustomDays ? parseInt(data.renewalCycleCustomDays, 10) : null,
      renewalPrice: data.renewalPrice ? parseFloat(data.renewalPrice) : null,
      renewalCurrency: data.renewalCurrency || undefined,
      nextRenewalDate: data.nextRenewalDate ? data.nextRenewalDate.toISOString() : null,
      lastRenewalDate: data.lastRenewalDate ? data.lastRenewalDate.toISOString() : null,
      serviceStartDate: data.serviceStartDate ? data.serviceStartDate.toISOString() : null,
      paymentMethod: data.paymentMethod || undefined,
      autoRenewEnabled: data.autoRenewEnabled,
      renewalNotes: data.renewalNotes || undefined,
    };

    try {
      await updateVps(vps.id, payload);
      toast.success(t('serverManagement.modals.edit.updateSuccess'));
      onVpsUpdated();
      onClose();
    } catch (err: unknown) {
      console.error('Failed to update VPS:', err);
      let errorMessage = t('serverManagement.modals.edit.updateFailed');
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
                        placeholder={t('serverManagement.modals.edit.creatable.searchPlaceholder')}
                        value={inputValue}
                        onValueChange={setInputValue}
                    />
                    <CommandEmpty>
                        <Button variant="ghost" className="w-full" onClick={handleCreate}>
                            {t('serverManagement.modals.edit.creatable.createLabel', { value: inputValue })}
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

  const MultiSelectPopover = ({ field, options, placeholder }: { field: ControllerRenderProps<FormValues, 'tagIds'>, options: TagOption[], placeholder: string }) => {
    return (
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="outline" className="w-full justify-between">
            <span className="truncate">
              {(field.value || []).length > 0 ? t('serverManagement.modals.bulkEditTags.selected', { count: (field.value || []).length }) : placeholder}
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
          <DialogTitle>{t('serverManagement.modals.edit.title')}</DialogTitle>
          <DialogDescription>{t('serverManagement.modals.edit.description')}</DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit(onSubmit)}>
          <Tabs defaultValue="basic" className="w-full">
            <TabsList className="grid w-full grid-cols-3">
              <TabsTrigger value="basic">{t('serverManagement.modals.edit.tabs.basic')}</TabsTrigger>
              <TabsTrigger value="traffic">{t('serverManagement.modals.edit.tabs.traffic')}</TabsTrigger>
              <TabsTrigger value="renewal">{t('serverManagement.modals.edit.tabs.renewal')}</TabsTrigger>
            </TabsList>
            
            <ScrollArea className="h-[500px] p-1">
            <div className="p-4">
            <TabsContent value="basic">
              <div className="space-y-4">
                <div>
                  <Label htmlFor="name" className="mb-2 block">{t('serverManagement.modals.edit.basicInfo.name')}</Label>
                  <Controller name="name" control={control} render={({ field }) => <Input id="name" {...field} />} />
                </div>
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.basicInfo.group')}</Label>
                  <Controller name="group" control={control} render={({ field }) => <CreatableCombobox field={field} options={groupOptions} placeholder={t('serverManagement.modals.edit.basicInfo.groupPlaceholder')} />} />
                </div>
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.basicInfo.tags')}</Label>
                  <Controller name="tagIds" control={control} render={({ field }) => <MultiSelectPopover field={field} options={tagOptions} placeholder={t('serverManagement.modals.edit.basicInfo.tagsPlaceholder')} />} />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="traffic">
              <div className="space-y-4">
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.trafficMonitoring.limit')}</Label>
                  <div className="flex space-x-2">
                    <Controller name="trafficLimitInput" control={control} render={({ field }) => <Input type="number" placeholder={t('serverManagement.modals.edit.trafficMonitoring.limitPlaceholder')} {...field} value={field.value || ''} />} />
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
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.trafficMonitoring.billingRule')}</Label>
                  <Controller name="trafficBillingRule" control={control} render={({ field }) => (
                    <Select onValueChange={field.onChange} value={field.value || ''}>
                      <SelectTrigger><SelectValue placeholder={t('serverManagement.modals.edit.trafficMonitoring.billingRulePlaceholder')} /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="sum_in_out">{t('serverManagement.modals.edit.trafficMonitoring.billingRules.sum')}</SelectItem>
                        <SelectItem value="out_only">{t('serverManagement.modals.edit.trafficMonitoring.billingRules.out')}</SelectItem>
                        <SelectItem value="max_in_out">{t('serverManagement.modals.edit.trafficMonitoring.billingRules.max')}</SelectItem>
                      </SelectContent>
                    </Select>
                  )} />
                </div>
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.trafficMonitoring.resetRuleType')}</Label>
                  <Controller name="trafficResetConfigType" control={control} render={({ field }) => (
                     <Select onValueChange={field.onChange} value={field.value || ''}>
                      <SelectTrigger><SelectValue placeholder={t('serverManagement.modals.edit.trafficMonitoring.resetRuleTypePlaceholder')} /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="monthly_day_of_month">{t('serverManagement.modals.edit.trafficMonitoring.resetRuleTypes.monthly')}</SelectItem>
                        <SelectItem value="fixed_days">{t('serverManagement.modals.edit.trafficMonitoring.resetRuleTypes.fixed')}</SelectItem>
                      </SelectContent>
                    </Select>
                  )} />
                </div>
                {watch('trafficResetConfigType') && (
                  <div>
                    <Label className="mb-2 block">{t('serverManagement.modals.edit.trafficMonitoring.resetRuleValue')}</Label>
                    <Controller name="trafficResetConfigValue" control={control} render={({ field }) => <Input {...field} value={field.value || ''} readOnly={watch('trafficResetConfigType') === 'monthly_day_of_month'} />} />
                  </div>
                )}
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.trafficMonitoring.nextResetDate')}</Label>
                  <Controller name="nextTrafficResetAt" control={control} render={({ field }) => <DateTimePicker value={field.value ? new Date(field.value) : null} onChange={field.onChange} />} />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="renewal">
              <div className="space-y-4">
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.renewalSettings.cycle')}</Label>
                  <div className="flex items-center gap-4">
                    <div>
                      <Controller name="renewalCycle" control={control} render={({ field }) => (
                        <Select onValueChange={field.onChange} value={field.value || ''}>
                          <SelectTrigger><SelectValue placeholder={t('serverManagement.modals.edit.renewalSettings.cyclePlaceholder')} /></SelectTrigger>
                          <SelectContent>
                            <SelectItem value="monthly">{t('serverManagement.modals.edit.renewalSettings.cycles.monthly')}</SelectItem>
                            <SelectItem value="quarterly">{t('serverManagement.modals.edit.renewalSettings.cycles.quarterly')}</SelectItem>
                            <SelectItem value="semi_annually">{t('serverManagement.modals.edit.renewalSettings.cycles.semi_annually')}</SelectItem>
                            <SelectItem value="annually">{t('serverManagement.modals.edit.renewalSettings.cycles.annually')}</SelectItem>
                            <SelectItem value="biennially">{t('serverManagement.modals.edit.renewalSettings.cycles.biennially')}</SelectItem>
                            <SelectItem value="triennially">{t('serverManagement.modals.edit.renewalSettings.cycles.triennially')}</SelectItem>
                            <SelectItem value="custom_days">{t('serverManagement.modals.edit.renewalSettings.cycles.custom')}</SelectItem>
                          </SelectContent>
                        </Select>
                      )} />
                    </div>
                    <div className="flex items-center space-x-2">
                      <Controller name="autoRenewEnabled" control={control} render={({ field }) => <Checkbox id="autoRenewEnabled" checked={field.value} onCheckedChange={field.onChange} />} />
                      <Label htmlFor="autoRenewEnabled" className="whitespace-nowrap">{t('serverManagement.modals.edit.renewalSettings.autoRenew')}</Label>
                    </div>
                  </div>
                </div>
                {watch('renewalCycle') === 'custom_days' && (
                  <div>
                    <Label className="mb-2 block">{t('serverManagement.modals.edit.renewalSettings.customCycleDays')}</Label>
                    <Controller name="renewalCycleCustomDays" control={control} render={({ field }) => <Input type="number" {...field} value={field.value || ''} />} />
                  </div>
                )}
                <div className="grid grid-cols-2 gap-4">
                    <div>
                        <Label className="mb-2 block">{t('serverManagement.modals.edit.renewalSettings.price')}</Label>
                        <Controller name="renewalPrice" control={control} render={({ field }) => <Input type="number" step="0.01" {...field} value={field.value || ''} />} />
                    </div>
                    <div>
                        <Label className="mb-2 block">{t('serverManagement.modals.edit.renewalSettings.currency')}</Label>
                        <Controller name="renewalCurrency" control={control} render={({ field }) => <CreatableCombobox field={field} options={[
                           {value: 'USD', label: t('serverManagement.modals.edit.renewalSettings.currencies.usd')},
                           {value: 'CNY', label: t('serverManagement.modals.edit.renewalSettings.currencies.cny')},
                           {value: 'EUR', label: t('serverManagement.modals.edit.renewalSettings.currencies.eur')},
                           {value: 'JPY', label: t('serverManagement.modals.edit.renewalSettings.currencies.jpy')},
                           {value: 'GBP', label: t('serverManagement.modals.edit.renewalSettings.currencies.gbp')},
                           {value: 'AUD', label: t('serverManagement.modals.edit.renewalSettings.currencies.aud')},
                           {value: 'CAD', label: t('serverManagement.modals.edit.renewalSettings.currencies.cad')},
                           {value: 'HKD', label: t('serverManagement.modals.edit.renewalSettings.currencies.hkd')}
                       ]} placeholder={t('serverManagement.modals.edit.renewalSettings.currencyPlaceholder')} />} />
                    </div>
                </div>
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.renewalSettings.startDate')}</Label>
                  <Controller
                    name="serviceStartDate"
                    control={control}
                    render={({ field }) => (
                      <DateTimePicker
                        value={field.value ? new Date(field.value) : null}
                        onChange={field.onChange}
                      />
                    )}
                  />
                </div>
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.renewalSettings.lastDate')}</Label>
                  <div className={`transition-all duration-300 ${
                    highlightedFields.lastRenewalDate
                      ? 'bg-green-50 border-green-300 rounded shadow-sm p-1'
                      : ''
                  }`}>
                    <Controller
                      name="lastRenewalDate"
                      control={control}
                      render={({ field }) => (
                        <DateTimePicker
                          value={field.value ? new Date(field.value) : null}
                          onChange={(date) => {
                            userModifiedLastRenewalDate.current = true;
                            field.onChange(date);
                          }}
                        />
                      )}
                    />
                  </div>
                </div>
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.renewalSettings.nextDate')}</Label>
                  <div className={`transition-all duration-300 ${
                    highlightedFields.nextRenewalDate
                      ? 'bg-green-50 border-green-300 rounded shadow-sm p-1'
                      : ''
                  }`}>
                    <Controller
                      name="nextRenewalDate"
                      control={control}
                      render={({ field }) => (
                        <DateTimePicker
                          value={field.value ? new Date(field.value) : null}
                          onChange={(date) => {
                            userModifiedNextRenewalDate.current = true;
                            field.onChange(date);
                          }}
                        />
                      )}
                    />
                  </div>
                </div>
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.renewalSettings.paymentMethod')}</Label>
                  <Controller name="paymentMethod" control={control} render={({ field }) => <CreatableCombobox field={field} options={[
                    { value: 'PayPal', label: t('serverManagement.modals.edit.renewalSettings.paymentMethods.paypal') },
                    { value: 'Alipay', label: t('serverManagement.modals.edit.renewalSettings.paymentMethods.alipay') },
                    { value: 'Credit Card', label: t('serverManagement.modals.edit.renewalSettings.paymentMethods.creditCard') },
                    { value: 'Stripe', label: t('serverManagement.modals.edit.renewalSettings.paymentMethods.stripe') },
                    { value: 'Bank Transfer', label: t('serverManagement.modals.edit.renewalSettings.paymentMethods.bankTransfer') },
                    { value: 'WeChat Pay', label: t('serverManagement.modals.edit.renewalSettings.paymentMethods.wechatPay') },
                    { value: 'UnionPay', label: t('serverManagement.modals.edit.renewalSettings.paymentMethods.unionPay') },
                    { value: 'Apple Pay', label: t('serverManagement.modals.edit.renewalSettings.paymentMethods.applePay') },
                    { value: 'Google Pay', label: t('serverManagement.modals.edit.renewalSettings.paymentMethods.googlePay') }
                    ]} placeholder={t('serverManagement.modals.edit.renewalSettings.paymentMethodPlaceholder')} />} />
                </div>
                <div>
                  <Label className="mb-2 block">{t('serverManagement.modals.edit.renewalSettings.notes')}</Label>
                  <Controller name="renewalNotes" control={control} render={({ field }) => <Textarea {...field} value={field.value || ''} />} />
                </div>
              </div>
            </TabsContent>
            </div>
            </ScrollArea>
          </Tabs>
          <DialogFooter className="pt-4">
            <Button type="button" variant="outline" onClick={onClose}>{t('common.actions.cancel')}</Button>
            <Button type="submit" disabled={form.formState.isSubmitting}>
              {form.formState.isSubmitting ? t('serverManagement.modals.edit.saving') : t('serverManagement.modals.edit.saveButton')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default React.memo(EditVpsModal);
