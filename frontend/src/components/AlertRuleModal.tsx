import React, { useEffect, useState } from 'react';
import { useForm, Controller } from 'react-hook-form';
import type { SubmitHandler } from 'react-hook-form';
import { X } from 'lucide-react';
import * as alertService from '../services/alertService'; // To be created
import { getAllChannels as getAllNotificationChannels } from '../services/notificationService'; // To fetch channels
import type { AlertRule, CreateAlertRulePayload, UpdateAlertRulePayload, VpsListItemResponse, ChannelResponse } from '../types'; // Assuming these types exist or will be created

// TODO: Define these types in src/types/index.ts
// export interface AlertRule {
//   id: number;
//   userId: number;
//   vpsId?: number | null;
//   metricType: string;
//   threshold: number;
//   comparisonOperator: string;
//   durationSeconds: number;
//   notificationChannelIds?: number[]; // Changed from single string to array of IDs
//   createdAt: string;
//   updatedAt: string;
// }
// export interface CreateAlertRulePayload {
//   name: string; // Assuming AlertRule has a name, add if not present in backend
//   vpsId?: number | null;
//   metricType: string;
//   threshold: number;
//   comparisonOperator: string;
//   durationSeconds: number;
//   notificationChannelIds: number[];
// }
// export interface UpdateAlertRulePayload extends Partial<CreateAlertRulePayload> {}


type AlertRuleFormInputs = {
  name: string;
  vpsId: string; // Store as string for form, convert to number or null on submit
  metricType: string;
  threshold: number;
  comparisonOperator: string;
  durationSeconds: number;
  notificationChannelIds: number[];
  cooldownSeconds: number; // Added
};

interface AlertRuleModalProps {
  isOpen: boolean;
  onClose: () => void;
  onRuleSaved: (data: CreateAlertRulePayload | UpdateAlertRulePayload) => Promise<void>; // Updated signature
  rule: AlertRule | null;
  vpsList: VpsListItemResponse[]; // To populate VPS dropdown
}

const AlertRuleModal: React.FC<AlertRuleModalProps> = ({ isOpen, onClose, onRuleSaved, rule, vpsList }) => {
  const {
    register,
    handleSubmit,
    reset,
    control,
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
          vpsId: rule.vpsId?.toString() || '',
          metricType: rule.metricType,
          threshold: rule.threshold,
          comparisonOperator: rule.comparisonOperator,
          durationSeconds: rule.durationSeconds,
          notificationChannelIds: rule.notificationChannelIds || [],
          cooldownSeconds: rule.cooldownSeconds || 300, // Added
        });
      } else {
        reset({
          name: '',
          vpsId: '',
          metricType: 'cpu_usage_percent', // Default value
          threshold: 80,
          comparisonOperator: '>',
          durationSeconds: 300,
          notificationChannelIds: [],
          cooldownSeconds: 300, // Default cooldown
        });
      }
    }
  }, [isOpen, rule, reset]);

  const onSubmit: SubmitHandler<AlertRuleFormInputs> = async (data) => {
    try {
      const payload = {
        name: data.name,
        vpsId: data.vpsId ? parseInt(data.vpsId, 10) : null,
        metricType: data.metricType,
        threshold: Number(data.threshold),
        comparisonOperator: data.comparisonOperator,
        durationSeconds: Number(data.durationSeconds),
        notificationChannelIds: data.notificationChannelIds.map(id => Number(id)),
        cooldownSeconds: Number(data.cooldownSeconds), // Added
      };

      if (rule) {
        await alertService.updateAlertRule(rule.id, payload as UpdateAlertRulePayload);
        console.log("Update Alert Rule:", rule.id, payload);
      } else {
        await alertService.createAlertRule(payload as CreateAlertRulePayload);
        console.log("Create Alert Rule:", payload);
      }
      await onRuleSaved(payload); // Pass payload to onRuleSaved
      onClose();
    } catch (err) {
      console.error('Failed to save alert rule:', err);
      // Consider adding toast notifications for errors
    }
  };

  if (!isOpen) {
    return null;
  }

  const metricTypes = ["cpu_usage_percent", "memory_usage_percent", "network_rx_instant_bps", "network_tx_instant_bps"]; // Example, fetch from backend or define globally
  const comparisonOperators = [">", "<", "=", ">=", "<="];


  return (
    <div className="fixed inset-0 bg-slate-900/50 flex items-center justify-center z-50 transition-opacity duration-300">
      <div className="bg-white rounded-lg shadow-xl p-6 w-full max-w-lg m-4 transform transition-all duration-300">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-xl font-bold text-slate-800">{rule ? 'Edit Alert Rule' : 'Create New Alert Rule'}</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600 transition-colors">
            <X className="w-6 h-6" />
          </button>
        </div>

        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          <div>
            <label htmlFor="ruleName" className="block text-sm font-medium text-slate-700 mb-1">Rule Name</label>
            <input
              type="text"
              id="ruleName"
              {...register('name', { required: 'Rule name is required' })}
              className={`w-full px-3 py-2 border rounded-md ${errors.name ? 'border-red-500' : 'border-slate-300'}`}
            />
            {errors.name && <p className="text-red-500 text-xs mt-1">{errors.name.message}</p>}
          </div>

          <div>
            <label htmlFor="vpsId" className="block text-sm font-medium text-slate-700 mb-1">Target VPS (Optional)</label>
            <select
              id="vpsId"
              {...register('vpsId')}
              className="w-full px-3 py-2 border border-slate-300 rounded-md"
            >
              <option value="">Global (All VPS)</option>
              {vpsList.map(vps => <option key={vps.id} value={vps.id}>{vps.name}</option>)}
            </select>
          </div>

          <div>
            <label htmlFor="metricType" className="block text-sm font-medium text-slate-700 mb-1">Metric Type</label>
            <select
              id="metricType"
              {...register('metricType', { required: 'Metric type is required' })}
              className={`w-full px-3 py-2 border rounded-md ${errors.metricType ? 'border-red-500' : 'border-slate-300'}`}
            >
              {metricTypes.map(mt => <option key={mt} value={mt}>{mt.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}</option>)}
            </select>
            {errors.metricType && <p className="text-red-500 text-xs mt-1">{errors.metricType.message}</p>}
          </div>
          
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label htmlFor="threshold" className="block text-sm font-medium text-slate-700 mb-1">Threshold</label>
              <input
                type="number"
                id="threshold"
                {...register('threshold', { required: 'Threshold is required', valueAsNumber: true })}
                className={`w-full px-3 py-2 border rounded-md ${errors.threshold ? 'border-red-500' : 'border-slate-300'}`}
              />
              {errors.threshold && <p className="text-red-500 text-xs mt-1">{errors.threshold.message}</p>}
            </div>
            <div>
              <label htmlFor="comparisonOperator" className="block text-sm font-medium text-slate-700 mb-1">Operator</label>
              <select
                id="comparisonOperator"
                {...register('comparisonOperator', { required: 'Operator is required' })}
                className={`w-full px-3 py-2 border rounded-md ${errors.comparisonOperator ? 'border-red-500' : 'border-slate-300'}`}
              >
                {comparisonOperators.map(op => <option key={op} value={op}>{op}</option>)}
              </select>
              {errors.comparisonOperator && <p className="text-red-500 text-xs mt-1">{errors.comparisonOperator.message}</p>}
            </div>
          </div>

          <div>
            <label htmlFor="durationSeconds" className="block text-sm font-medium text-slate-700 mb-1">Duration (seconds)</label>
            <input
              type="number"
              id="durationSeconds"
              {...register('durationSeconds', { required: 'Duration is required', valueAsNumber: true, min: { value: 1, message: "Duration must be at least 1 second"} })}
              className={`w-full px-3 py-2 border rounded-md ${errors.durationSeconds ? 'border-red-500' : 'border-slate-300'}`}
            />
            {errors.durationSeconds && <p className="text-red-500 text-xs mt-1">{errors.durationSeconds.message}</p>}
          </div>

          <div>
            <label htmlFor="cooldownSeconds" className="block text-sm font-medium text-slate-700 mb-1">Cooldown (seconds)</label>
            <input
              type="number"
              id="cooldownSeconds"
              {...register('cooldownSeconds', { required: 'Cooldown is required', valueAsNumber: true, min: { value: 0, message: "Cooldown must be non-negative"} })}
              className={`w-full px-3 py-2 border rounded-md ${errors.cooldownSeconds ? 'border-red-500' : 'border-slate-300'}`}
            />
            {errors.cooldownSeconds && <p className="text-red-500 text-xs mt-1">{errors.cooldownSeconds.message}</p>}
          </div>

          <div>
            <label htmlFor="notificationChannelIds" className="block text-sm font-medium text-slate-700 mb-1">Notification Channels</label>
            <Controller
                name="notificationChannelIds"
                control={control}
                defaultValue={[]}
                render={({ field }) => (
                    <select
                        multiple
                        id="notificationChannelIds"
                        className="w-full px-3 py-2 border border-slate-300 rounded-md h-32"
                        // react-hook-form's field.value for multiple select is expected to be an array of values.
                        // HTML select element's value property behaves differently for multiple.
                        // We ensure the value passed to select is an array of strings (matching option values).
                        value={field.value ? field.value.map(String) : []}
                        onBlur={field.onBlur}
                        ref={field.ref}
                        onChange={(e) => {
                            const selectedOptions = Array.from(e.target.selectedOptions, option => Number(option.value));
                            field.onChange(selectedOptions);
                        }}
                    >
                        {notificationChannels.map(channel => (
                            <option key={channel.id} value={channel.id}>{channel.name} ({channel.channelType})</option>
                        ))}
                    </select>
                )}
            />
             {errors.notificationChannelIds && <p className="text-red-500 text-xs mt-1">{errors.notificationChannelIds.message}</p>}
          </div>


          <div className="mt-6 flex justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="bg-slate-200 hover:bg-slate-300 text-slate-800 font-semibold py-2 px-4 rounded-lg shadow-sm transition-colors duration-150"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="bg-indigo-600 hover:bg-indigo-700 text-white font-semibold py-2 px-4 rounded-lg shadow-sm transition-colors duration-150 disabled:bg-indigo-400 disabled:cursor-not-allowed"
            >
              {isSubmitting ? 'Saving...' : (rule ? 'Update Rule' : 'Create Rule')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default AlertRuleModal;