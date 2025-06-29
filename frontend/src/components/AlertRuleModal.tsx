import React, { useEffect, useState } from 'react';
import { useForm, Controller } from 'react-hook-form';
import type { SubmitHandler } from 'react-hook-form';
import * as alertService from '../services/alertService';
import { getAllChannels as getAllNotificationChannels } from '../services/notificationService';
import type { AlertRule, CreateAlertRulePayload, UpdateAlertRulePayload, VpsListItemResponse, ChannelResponse } from '../types';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Checkbox } from "@/components/ui/checkbox";
import { RefreshCwIcon as SpinnerIcon } from '@/components/Icons';

type AlertRuleFormInputs = {
  name: string;
  vpsId: string;
  metricType: string;
  threshold: number;
  comparisonOperator: string;
  durationSeconds: number;
  notificationChannelIds: number[];
  cooldownSeconds: number;
};

interface AlertRuleModalProps {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  onRuleSaved: () => void;
  rule: AlertRule | null;
  vpsList: VpsListItemResponse[];
}

const AlertRuleModal: React.FC<AlertRuleModalProps> = ({ isOpen, onOpenChange, onRuleSaved, rule, vpsList }) => {
  const {
    control,
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<AlertRuleFormInputs>();

  const [notificationChannels, setNotificationChannels] = useState<ChannelResponse[]>([]);

  useEffect(() => {
    if (isOpen) {
      getAllNotificationChannels()
        .then(setNotificationChannels)
        .catch(err => console.error("Failed to fetch notification channels", err));

      if (rule) {
        reset({
          name: rule.name || '',
          vpsId: rule.vpsId?.toString() || 'global',
          metricType: rule.metricType,
          threshold: rule.threshold,
          comparisonOperator: rule.comparisonOperator,
          durationSeconds: rule.durationSeconds,
          notificationChannelIds: rule.notificationChannelIds || [],
          cooldownSeconds: rule.cooldownSeconds || 300,
        });
      } else {
        reset({
          name: '',
          vpsId: 'global',
          metricType: 'cpu_usage_percent',
          threshold: 80,
          comparisonOperator: '>',
          durationSeconds: 300,
          notificationChannelIds: [],
          cooldownSeconds: 300,
        });
      }
    }
  }, [isOpen, rule, reset]);

  const onSubmit: SubmitHandler<AlertRuleFormInputs> = async (data) => {
    try {
      const payload = {
        ...data,
        vpsId: data.vpsId === 'global' ? null : parseInt(data.vpsId, 10),
        threshold: Number(data.threshold),
        durationSeconds: Number(data.durationSeconds),
        cooldownSeconds: Number(data.cooldownSeconds),
        notificationChannelIds: data.notificationChannelIds.map(id => Number(id)),
      };

      if (rule) {
        await alertService.updateAlertRule(rule.id, payload as UpdateAlertRulePayload);
      } else {
        await alertService.createAlertRule(payload as CreateAlertRulePayload);
      }
      onRuleSaved();
      onOpenChange(false);
    } catch (err) {
      console.error('Failed to save alert rule:', err);
    }
  };

  const metricTypes = ["cpu_usage_percent", "memory_usage_percent", "network_rx_instant_bps", "network_tx_instant_bps"];
  const comparisonOperators = [">", "<", "=", ">=", "<="];

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>{rule ? 'Edit Alert Rule' : 'Create New Alert Rule'}</DialogTitle>
          <DialogDescription>
            Configure the details of your alert rule. Click save when you're done.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="name">Rule Name</Label>
            <Input id="name" {...register('name', { required: 'Rule name is required' })} />
            {errors.name && <p className="text-sm text-destructive">{errors.name.message}</p>}
          </div>

          <div className="space-y-2">
            <Label htmlFor="vpsId">Target VPS (Optional)</Label>
            <Controller
              name="vpsId"
              control={control}
              render={({ field }) => (
                <Select onValueChange={field.onChange} defaultValue={field.value} value={field.value}>
                  <SelectTrigger>
                    <SelectValue placeholder="Select a VPS or Global" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="global">Global (All VPS)</SelectItem>
                    {vpsList.map(vps => <SelectItem key={vps.id} value={vps.id.toString()}>{vps.name}</SelectItem>)}
                  </SelectContent>
                </Select>
              )}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="metricType">Metric Type</Label>
            <Controller
              name="metricType"
              control={control}
              render={({ field }) => (
                <Select onValueChange={field.onChange} defaultValue={field.value} value={field.value}>
                  <SelectTrigger>
                    <SelectValue placeholder="Select a metric type" />
                  </SelectTrigger>
                  <SelectContent>
                    {metricTypes.map(mt => <SelectItem key={mt} value={mt}>{mt.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}</SelectItem>)}
                  </SelectContent>
                </Select>
              )}
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="threshold">Threshold</Label>
              <Input id="threshold" type="number" {...register('threshold', { required: 'Threshold is required', valueAsNumber: true })} />
              {errors.threshold && <p className="text-sm text-destructive">{errors.threshold.message}</p>}
            </div>
            <div className="space-y-2">
              <Label htmlFor="comparisonOperator">Operator</Label>
              <Controller
                name="comparisonOperator"
                control={control}
                render={({ field }) => (
                  <Select onValueChange={field.onChange} defaultValue={field.value} value={field.value}>
                    <SelectTrigger>
                      <SelectValue placeholder="Select an operator" />
                    </SelectTrigger>
                    <SelectContent>
                      {comparisonOperators.map(op => <SelectItem key={op} value={op}>{op}</SelectItem>)}
                    </SelectContent>
                  </Select>
                )}
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="durationSeconds">Duration (seconds)</Label>
              <Input id="durationSeconds" type="number" {...register('durationSeconds', { required: 'Duration is required', valueAsNumber: true, min: { value: 1, message: "Duration must be at least 1 second"} })} />
              {errors.durationSeconds && <p className="text-sm text-destructive">{errors.durationSeconds.message}</p>}
            </div>
            <div className="space-y-2">
              <Label htmlFor="cooldownSeconds">Cooldown (seconds)</Label>
              <Input id="cooldownSeconds" type="number" {...register('cooldownSeconds', { required: 'Cooldown is required', valueAsNumber: true, min: { value: 0, message: "Cooldown must be non-negative"} })} />
              {errors.cooldownSeconds && <p className="text-sm text-destructive">{errors.cooldownSeconds.message}</p>}
            </div>
          </div>

          <div className="space-y-3">
            <Label>Notification Channels</Label>
            <Controller
              name="notificationChannelIds"
              control={control}
              render={({ field }) => (
                <div className="space-y-2 rounded-md border p-4 max-h-40 overflow-y-auto">
                  {notificationChannels.map((channel) => (
                    <div key={channel.id} className="flex flex-row items-start space-x-3 space-y-0">
                      <Checkbox
                        id={channel.id.toString()}
                        checked={field.value?.includes(channel.id)}
                        onCheckedChange={(checked) => {
                          return checked
                            ? field.onChange([...(field.value || []), channel.id])
                            : field.onChange(field.value?.filter((id) => id !== channel.id));
                        }}
                      />
                      <Label htmlFor={channel.id.toString()} className="font-normal">
                        {channel.name} <span className="text-muted-foreground">({channel.channelType})</span>
                      </Label>
                    </div>
                  ))}
                </div>
              )}
            />
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting && <SpinnerIcon className="mr-2 h-4 w-4 animate-spin" />}
              {isSubmitting ? 'Saving...' : (rule ? 'Update Rule' : 'Create Rule')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default AlertRuleModal;