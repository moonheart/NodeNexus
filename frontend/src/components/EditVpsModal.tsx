import React, { useState, useEffect, useMemo } from 'react';
import Select from 'react-select';
import CreatableSelect from 'react-select/creatable';
import { updateVps } from '../services/vpsService';
import type { VpsListItemResponse } from '../types';
import axios from 'axios';
import { useServerListStore } from '../store/serverListStore';
import { X } from 'lucide-react'; // Removed ChevronDown, ChevronUp
const BYTES_IN_KB = 1024;
const BYTES_IN_MB = BYTES_IN_KB * 1024;
const BYTES_IN_GB = BYTES_IN_MB * 1024;
const BYTES_IN_TB = BYTES_IN_GB * 1024;

const bytesToOptimalUnit = (bytes: number): { value: number, unit: string } => {
  if (bytes >= BYTES_IN_TB) {
    return { value: parseFloat((bytes / BYTES_IN_TB).toFixed(2)), unit: 'TB' };
  } else if (bytes >= BYTES_IN_GB) {
    return { value: parseFloat((bytes / BYTES_IN_GB).toFixed(2)), unit: 'GB' };
  } else if (bytes >= BYTES_IN_MB) {
    return { value: parseFloat((bytes / BYTES_IN_MB).toFixed(2)), unit: 'MB' };
  }
  // Default to GB if less than MB for simplicity, or could add KB and Bytes
  // For input purposes, showing smaller numbers in GB might be fine.
  // If bytes is very small, e.g. less than 1MB, this will show 0.00 GB.
  // Consider adding KB or even Bytes if precision for smaller values is critical in the modal.
  // For now, let's stick to MB, GB, TB for the select options.
  // If we want to be more precise for display:
  if (bytes < BYTES_IN_MB && bytes > 0) { // If less than 1MB but not 0
      return { value: parseFloat((bytes / BYTES_IN_MB).toFixed(2)), unit: 'MB'};
  }
  return { value: parseFloat((bytes / BYTES_IN_GB).toFixed(2)), unit: 'GB' }; 
};

const unitToBytes = (value: number, unit: string): number => {
  switch (unit) {
    case 'MB':
      return Math.round(value * BYTES_IN_MB);
    case 'GB':
      return Math.round(value * BYTES_IN_GB);
    case 'TB':
      return Math.round(value * BYTES_IN_TB);
    default:
      return 0; 
  }
};

interface EditVpsModalProps {
  isOpen: boolean;
  onClose: () => void;
  vps: VpsListItemResponse | null;
  allVps: VpsListItemResponse[]; // Keep for group options for now
  onVpsUpdated: () => void; // Callback to trigger data refresh
}

const EditVpsModal: React.FC<EditVpsModalProps> = ({ isOpen, onClose, vps, allVps, onVpsUpdated }) => {
  const [name, setName] = useState('');
  const [group, setGroup] = useState<{ value: string; label: string } | null>(null);
  const [selectedTags, setSelectedTags] = useState<{ value: number; label: string }[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Traffic Monitoring States
  const [trafficLimitInput, setTrafficLimitInput] = useState<string>(''); // User input for traffic limit value
  const [trafficLimitUnit, setTrafficLimitUnit] = useState<string>('GB'); // Default unit
  const [trafficBillingRule, setTrafficBillingRule] = useState<string>('');
  const [trafficResetConfigType, setTrafficResetConfigType] = useState<string>('');
  const [trafficResetConfigValue, setTrafficResetConfigValue] = useState<string>('');
  const [nextTrafficResetAt, setNextTrafficResetAt] = useState<string>('');

  // Renewal Info States
  const [renewalCycle, setRenewalCycle] = useState<string>('');
  const [renewalCycleCustomDays, setRenewalCycleCustomDays] = useState<string>(''); // Input as string
  const [renewalPrice, setRenewalPrice] = useState<string>(''); // Input as string
  const [renewalCurrency, setRenewalCurrency] = useState<string>('');
  const [nextRenewalDate, setNextRenewalDate] = useState<string>(''); // datetime-local format
  const [lastRenewalDate, setLastRenewalDate] = useState<string>(''); // datetime-local format
  const [serviceStartDate, setServiceStartDate] = useState<string>(''); // datetime-local format
  const [paymentMethod, setPaymentMethod] = useState<string>('');
  const [autoRenewEnabled, setAutoRenewEnabled] = useState<boolean>(false);
  const [renewalNotes, setRenewalNotes] = useState<string>('');
  // const [isRenewalSectionOpen, setIsRenewalSectionOpen] = useState(true); // Removed
  const [activeTab, setActiveTab] = useState<'basic' | 'traffic' | 'renewal'>('basic');
 
  const allTags = useServerListStore((state) => state.allTags);
  const fetchAllTags = useServerListStore((state) => state.fetchAllTags);

  const groupOptions = useMemo(() => {
    const allGroups = new Set(allVps.map(v => v.group).filter((g): g is string => !!g));
    return [...allGroups].map(g => ({ value: g, label: g }));
  }, [allVps]);

  const tagOptions = useMemo(() => {
    return allTags.map(tag => ({ value: tag.id, label: tag.name }));
  }, [allTags]);

  useEffect(() => {
    if (isOpen && fetchAllTags) {
      fetchAllTags();
    }
  }, [isOpen, fetchAllTags]);

  useEffect(() => {
    // When the modal is opened or the vps prop changes, initialize the form state.
    // This prevents live data from websockets (via the `servers` store) from
    // overwriting what the user is actively editing.
    if (isOpen && vps) {
      setName(vps.name || '');
      setGroup(vps.group ? { value: vps.group, label: vps.group } : null);
      setSelectedTags(vps.tags ? vps.tags.map(t => ({ value: t.id, label: t.name })) : []);
      
      // Initialize traffic fields
      if (vps.trafficLimitBytes != null) {
        const { value, unit } = bytesToOptimalUnit(vps.trafficLimitBytes);
        setTrafficLimitInput(value.toString());
        setTrafficLimitUnit(unit);
      } else {
        setTrafficLimitInput('');
        setTrafficLimitUnit('GB'); // Default to GB if no limit is set
      }
      setTrafficBillingRule(vps.trafficBillingRule || '');
      setTrafficResetConfigType(vps.trafficResetConfigType || '');
      setTrafficResetConfigValue(vps.trafficResetConfigValue || '');
      setNextTrafficResetAt(vps.nextTrafficResetAt ? vps.nextTrafficResetAt.substring(0, 16) : ''); // Format for datetime-local

      // Initialize renewal fields
      setRenewalCycle(vps.renewalCycle || '');
      setRenewalCycleCustomDays(vps.renewalCycleCustomDays?.toString() || '');
      setRenewalPrice(vps.renewalPrice?.toString() || '');
      setRenewalCurrency(vps.renewalCurrency || '');
      setNextRenewalDate(vps.nextRenewalDate ? vps.nextRenewalDate.substring(0, 16) : '');
      setLastRenewalDate(vps.lastRenewalDate ? vps.lastRenewalDate.substring(0, 16) : '');
      setServiceStartDate(vps.serviceStartDate ? vps.serviceStartDate.substring(0, 16) : '');
      setPaymentMethod(vps.paymentMethod || '');
      setAutoRenewEnabled(vps.autoRenewEnabled || false);
      setRenewalNotes(vps.renewalNotes || '');

      setError(null);
      setIsLoading(false);
    }
  }, [vps, isOpen]);

  useEffect(() => {
    if (trafficResetConfigType === 'monthly_day_of_month' && nextTrafficResetAt) {
      try {
        const date = new Date(nextTrafficResetAt);
        const day = date.getDate();
        const hours = date.getHours();
        const minutes = date.getMinutes();
        const seconds = date.getSeconds(); // Usually 0 for datetime-local, but good to include
        const timeOffsetSeconds = hours * 3600 + minutes * 60 + seconds;
        setTrafficResetConfigValue(`day:${day},time_offset_seconds:${timeOffsetSeconds}`);
      } catch (e) {
        console.error("Error parsing nextTrafficResetAt for config value:", e);
        // Potentially clear trafficResetConfigValue or set an error if parsing fails
        // For now, we'll let it be, user might be typing an invalid date temporarily
      }
    }
    // Do not clear trafficResetConfigValue if type changes from monthly_day_of_month
    // as user might be switching back and forth or have a pre-filled value for fixed_days
  }, [nextTrafficResetAt, trafficResetConfigType]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!vps) return;

    setError(null); // Clear previous errors

    // Validation for nextTrafficResetAt
    if ((trafficResetConfigType === 'fixed_days' || trafficResetConfigType === 'monthly_day_of_month') && !nextTrafficResetAt.trim()) {
      setError('当重置规则类型为 "固定天数间隔" 或 "每月指定日期" 时，下次重置时间不能为空。');
      setIsLoading(false);
      return;
    }
    
    // Ensure trafficResetConfigValue is not empty if config type requires it (e.g. fixed_days)
    if (trafficResetConfigType === 'fixed_days' && !trafficResetConfigValue.trim()) {
        setError('当重置规则类型为 "固定天数间隔" 时，重置规则值不能为空。');
        setIsLoading(false);
        return;
    }

    // If trafficLimitInput is set, then trafficBillingRule and trafficResetConfigType must be set
    if (trafficLimitInput.trim() && (!trafficBillingRule || !trafficResetConfigType)) {
      const missingFields = [];
      if (!trafficBillingRule) missingFields.push("流量计费规则");
      if (!trafficResetConfigType) missingFields.push("流量重置规则类型");
      setError(`当设置了流量限制时，必须设置 ${missingFields.join(" 和 ")}。`);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);

    const payload = {
      name: name.trim(),
      group: group?.value,
      tagIds: selectedTags.map(t => t.value), // Renamed from tag_ids
      trafficLimitBytes: trafficLimitInput ? unitToBytes(parseFloat(trafficLimitInput), trafficLimitUnit) : null, // Renamed
      trafficBillingRule: trafficBillingRule || null, // Renamed
      trafficResetConfigType: trafficResetConfigType || null, // Renamed
      trafficResetConfigValue: trafficResetConfigValue || null, // Renamed
      nextTrafficResetAt: nextTrafficResetAt ? new Date(nextTrafficResetAt).toISOString() : null, // Renamed
 
      // Renewal Info
      renewalCycle: renewalCycle || null,
      renewalCycleCustomDays: renewalCycleCustomDays ? parseInt(renewalCycleCustomDays, 10) : null,
      renewalPrice: renewalPrice ? parseFloat(renewalPrice) : null,
      renewalCurrency: renewalCurrency || null,
      nextRenewalDate: nextRenewalDate ? new Date(nextRenewalDate).toISOString() : null,
      lastRenewalDate: lastRenewalDate ? new Date(lastRenewalDate).toISOString() : null,
      serviceStartDate: serviceStartDate ? new Date(serviceStartDate).toISOString() : null,
      paymentMethod: paymentMethod || null,
      autoRenewEnabled: autoRenewEnabled,
      renewalNotes: renewalNotes || null,
    };

    try {
      await updateVps(vps.id, payload);
      onVpsUpdated(); // Trigger refresh in parent component
      onClose(); // Close modal on success
    } catch (err: unknown) {
      console.error('Failed to update VPS:', err);
      let errorMessage = '更新VPS失败，请稍后再试。';
      if (axios.isAxiosError(err) && err.response?.data?.error) {
        errorMessage = err.response.data.error;
      } else if (err instanceof Error) {
        errorMessage = err.message;
      }
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  };

  if (!isOpen || !vps) {
    return null;
  }

  return (
    <div className="fixed inset-0 bg-slate-900/50 flex items-center justify-center z-50 transition-opacity duration-300">
      <div className="bg-white rounded-lg shadow-xl p-6 w-full max-w-md m-4 transform transition-all duration-300">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-xl font-bold text-slate-800">编辑服务器信息</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600 transition-colors">
            <X className="w-6 h-6" />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          {/* Tab Navigation */}
          <div className="mb-4 border-b border-slate-200">
            <nav className="-mb-px flex space-x-4" aria-label="Tabs">
              <button
                type="button"
                onClick={() => setActiveTab('basic')}
                className={`${
                  activeTab === 'basic'
                    ? 'border-indigo-500 text-indigo-600'
                    : 'border-transparent text-slate-500 hover:text-slate-700 hover:border-slate-300'
                } whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm focus:outline-none`}
              >
                基本信息
              </button>
              <button
                type="button"
                onClick={() => setActiveTab('traffic')}
                className={`${
                  activeTab === 'traffic'
                    ? 'border-indigo-500 text-indigo-600'
                    : 'border-transparent text-slate-500 hover:text-slate-700 hover:border-slate-300'
                } whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm focus:outline-none`}
              >
                流量监控
              </button>
              <button
                type="button"
                onClick={() => setActiveTab('renewal')}
                className={`${
                  activeTab === 'renewal'
                    ? 'border-indigo-500 text-indigo-600'
                    : 'border-transparent text-slate-500 hover:text-slate-700 hover:border-slate-300'
                } whitespace-nowrap py-3 px-1 border-b-2 font-medium text-sm focus:outline-none`}
              >
                续费设置
              </button>
            </nav>
          </div>

          <div className="space-y-4">
            {/* Basic Info Tab Content */}
            {activeTab === 'basic' && (
              <div className="space-y-4">
                <div>
                  <label htmlFor="vpsName" className="block text-sm font-medium text-slate-700 mb-1">名称</label>
              <input
                type="text"
                id="vpsName"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                required
              />
            </div>
            <div>
              <label htmlFor="vpsGroup" className="block text-sm font-medium text-slate-700 mb-1">分组</label>
              <CreatableSelect
                isClearable
                options={groupOptions}
                value={group}
                onChange={(newValue) => setGroup(newValue)}
                placeholder="选择或创建一个分组..."
              />
            </div>
            <div>
              <label htmlFor="vpsTags" className="block text-sm font-medium text-slate-700 mb-1">标签</label>
              <Select
                isMulti
                options={tagOptions}
                value={selectedTags}
                onChange={(newValue) => setSelectedTags(Array.from(newValue))}
                placeholder="选择标签..."
                closeMenuOnSelect={false}
              />
            </div>
              </div>
            )}

            {/* Traffic Monitoring Tab Content */}
            {activeTab === 'traffic' && (
              // Removed "border-t border-slate-200 pt-4 mt-4" and h3 title
              <div className="space-y-4">
                <div>
                  <label htmlFor="trafficLimitInput" className="block text-sm font-medium text-slate-700 mb-1">流量限制</label>
                <div className="flex space-x-2">
                  <input
                    type="number"
                    id="trafficLimitInput"
                    value={trafficLimitInput}
                    onChange={(e) => setTrafficLimitInput(e.target.value)}
                    placeholder="例如: 100"
                    className="w-2/3 px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                  />
                  <select
                    id="trafficLimitUnit"
                    value={trafficLimitUnit}
                    onChange={(e) => setTrafficLimitUnit(e.target.value)}
                    className="w-1/3 px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                  >
                    <option value="MB">MB</option>
                    <option value="GB">GB</option>
                    <option value="TB">TB</option>
                  </select>
                </div>
                <p className="text-xs text-slate-500 mt-1">留空表示不限制。</p>
              </div>
              <div className="mt-4">
                <label htmlFor="trafficBillingRule" className="block text-sm font-medium text-slate-700 mb-1">流量计费规则</label>
                <select
                  id="trafficBillingRule"
                  value={trafficBillingRule}
                  onChange={(e) => setTrafficBillingRule(e.target.value)}
                  className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                >
                  <option value="">不设置</option>
                  <option value="sum_in_out">双向流量 (IN + OUT)</option>
                  <option value="out_only">出站流量 (OUT Only)</option>
                  <option value="max_in_out">单向最大值 (Max(IN, OUT))</option>
                </select>
              </div>
              <div className="mt-4">
                <label htmlFor="trafficResetConfigType" className="block text-sm font-medium text-slate-700 mb-1">流量重置规则类型</label>
                <select
                  id="trafficResetConfigType"
                  value={trafficResetConfigType}
                  onChange={(e) => {
                    setTrafficResetConfigType(e.target.value);
                    setTrafficResetConfigValue(''); // Clear dependent field
                  }}
                  className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                >
                  <option value="">不设置</option>
                  <option value="monthly_day_of_month">每月指定日期</option>
                  <option value="fixed_days">固定天数间隔</option>
                </select>
              </div>
              {trafficResetConfigType && (
                <div className="mt-4">
                  <label htmlFor="trafficResetConfigValue" className="block text-sm font-medium text-slate-700 mb-1">
                    {trafficResetConfigType === 'monthly_day_of_month'
                      ? '重置规则值 (根据下次重置时间自动计算)'
                      : trafficResetConfigType === 'fixed_days'
                        ? '重置规则值 (例如: 30)'
                        : '重置规则值'}
                  </label>
                  <input
                    type="text"
                    id="trafficResetConfigValue"
                    value={trafficResetConfigValue}
                    onChange={(e) => setTrafficResetConfigValue(e.target.value)}
                    readOnly={trafficResetConfigType === 'monthly_day_of_month'}
                    placeholder={
                      trafficResetConfigType === 'monthly_day_of_month'
                        ? "根据下次重置时间自动生成"
                        : "例如: 30 (天数)"
                    }
                    className={`w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 ${trafficResetConfigType === 'monthly_day_of_month' ? 'bg-slate-100 cursor-not-allowed' : ''}`}
                  />
                   {trafficResetConfigType === 'monthly_day_of_month' && <p className="text-xs text-slate-500 mt-1">此值根据您选择的“下次重置时间”自动计算得出。</p>}
                   {trafficResetConfigType === 'fixed_days' && <p className="text-xs text-slate-500 mt-1">输入天数。从上次重置时间开始，经过指定天数后重置。</p>}
                </div>
              )}
              <div className="mt-4">
                <label htmlFor="nextTrafficResetAt" className="block text-sm font-medium text-slate-700 mb-1">
                  下次重置时间 {(trafficResetConfigType === 'fixed_days' || trafficResetConfigType === 'monthly_day_of_month') ? '(必填)' : '(可选)'}
                </label>
                <input
                  type="datetime-local"
                  id="nextTrafficResetAt"
                  value={nextTrafficResetAt}
                  onChange={(e) => setNextTrafficResetAt(e.target.value)}
                  className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                />
                <p className="text-xs text-slate-500 mt-1">
                  {(trafficResetConfigType === 'fixed_days' || trafficResetConfigType === 'monthly_day_of_month')
                    ? '此字段为必填项。'
                    : '如果留空，后端将根据重置规则自动计算。如果填写，将以此时间为准。'}
                </p>
              </div>
              </div> // Closing div for activeTab === 'traffic'
            )}
 
            {/* Renewal Information Tab Content */}
            {activeTab === 'renewal' && (
              // Removed "border-t border-slate-200 pt-4 mt-4" and collapsible button
              <div className="space-y-4">
                  <div>
                    <label htmlFor="renewalCycle" className="block text-sm font-medium text-slate-700 mb-1">续费周期</label>
                <select
                  id="renewalCycle"
                  value={renewalCycle}
                  onChange={(e) => {
                    const newCycle = e.target.value;
                    setRenewalCycle(newCycle);
                    if (newCycle !== 'custom_days') {
                      setRenewalCycleCustomDays('');
                    }
                    if (newCycle === '') { // If "不设置" is chosen, clear all other renewal fields
                      setRenewalCycleCustomDays('');
                      setRenewalPrice('');
                      setRenewalCurrency('');
                      setNextRenewalDate('');
                      setLastRenewalDate('');
                      setServiceStartDate('');
                      setPaymentMethod('');
                      setAutoRenewEnabled(false);
                      setRenewalNotes('');
                    }
                  }}
                  className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                >
                  <option value="">不设置</option>
                  <option value="monthly">每月</option>
                  <option value="quarterly">每季度</option>
                  <option value="semi_annually">每半年</option>
                  <option value="annually">每年</option>
                  <option value="biennially">每两年</option>
                  <option value="triennially">每三年</option>
                  <option value="custom_days">自定义天数</option>
                </select>
              </div>
 
              {/* Conditionally render the rest of the renewal fields only if a renewalCycle is selected */}
              {renewalCycle && (
                <>
                  {renewalCycle === 'custom_days' && (
                    <div className="mt-4">
                      <label htmlFor="renewalCycleCustomDays" className="block text-sm font-medium text-slate-700 mb-1">自定义周期天数</label>
                      <input
                        type="number"
                        id="renewalCycleCustomDays"
                        value={renewalCycleCustomDays}
                        onChange={(e) => setRenewalCycleCustomDays(e.target.value)}
                        placeholder="例如: 45"
                        className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                      />
                    </div>
                  )}

                  {/* Renewal Price and Currency on the same line */}
                  <div className="mt-4 grid grid-cols-4 gap-4 items-end">
                    <div className="col-span-2">
                      <label htmlFor="renewalPrice" className="block text-sm font-medium text-slate-700 mb-1">续费价格</label>
                      <input
                        type="number"
                        step="0.01"
                        id="renewalPrice"
                        value={renewalPrice}
                        onChange={(e) => setRenewalPrice(e.target.value)}
                        placeholder="例如: 19.99"
                        className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                      />
                    </div>
                    <div className="col-span-2">
                      <label htmlFor="renewalCurrency" className="block text-sm font-medium text-slate-700 mb-1">货币</label>
                      <CreatableSelect
                        id="renewalCurrency"
                        isClearable
                        value={renewalCurrency ? { value: renewalCurrency, label: renewalCurrency } : null}
                        onChange={(selectedOption) => setRenewalCurrency(selectedOption ? selectedOption.value : '')}
                        options={[
                          { value: 'USD', label: 'USD' },
                          { value: 'CNY', label: 'CNY' },
                          { value: 'EUR', label: 'EUR' },
                          { value: 'JPY', label: 'JPY' },
                          { value: 'GBP', label: 'GBP' },
                          { value: 'HKD', label: 'HKD' },
                          { value: 'AUD', label: 'AUD' },
                          { value: 'CAD', label: 'CAD' },
                          { value: 'SGD', label: 'SGD' },
                        ]}
                        placeholder="选择或输入..."
                        classNamePrefix="react-select"
                        styles={{
                          control: (base) => ({ ...base, minHeight: '42px', borderColor: 'rgb(203 213 225)', boxShadow: '0 1px 2px 0 rgb(0 0 0 / 0.05)', '&:hover': { borderColor: 'rgb(99 102 241)' }}),
                          input: (base) => ({ ...base, margin: '0px', paddingBottom: '0px', paddingTop: '0px' }),
                          valueContainer: (base) => ({ ...base, padding: '0px 8px' }),
                          placeholder: (base) => ({ ...base, color: 'rgb(100 116 139)' })
                        }}
                      />
                    </div>
                  </div>
                  
                  <div className="mt-4">
                <label htmlFor="serviceStartDate" className="block text-sm font-medium text-slate-700 mb-1">服务开始日期</label>
                <input
                  type="datetime-local"
                  id="serviceStartDate"
                  value={serviceStartDate}
                  onChange={(e) => setServiceStartDate(e.target.value)}
                  className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                />
              </div>

              <div className="mt-4">
                <label htmlFor="lastRenewalDate" className="block text-sm font-medium text-slate-700 mb-1">上次续费日期</label>
                <input
                  type="datetime-local"
                  id="lastRenewalDate"
                  value={lastRenewalDate}
                  onChange={(e) => setLastRenewalDate(e.target.value)}
                  className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                />
              </div>

              <div className="mt-4">
                <label htmlFor="nextRenewalDate" className="block text-sm font-medium text-slate-700 mb-1">下次续费日期</label>
                <input
                  type="datetime-local"
                  id="nextRenewalDate"
                  value={nextRenewalDate}
                  onChange={(e) => setNextRenewalDate(e.target.value)}
                  className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                />
                 <p className="text-xs text-slate-500 mt-1">如果留空，且设置了续费周期和上次续费/服务开始日期，后端将自动计算。</p>
              </div>

              <div className="mt-4">
                <label htmlFor="paymentMethod" className="block text-sm font-medium text-slate-700 mb-1">支付方式</label>
                <CreatableSelect
                  id="paymentMethod"
                  isClearable
                  value={paymentMethod ? { value: paymentMethod, label: paymentMethod } : null}
                  onChange={(selectedOption) => setPaymentMethod(selectedOption ? selectedOption.value : '')}
                  options={[
                    { value: 'PayPal', label: 'PayPal' },
                    { value: 'Alipay', label: 'Alipay (支付宝)' },
                    { value: 'Credit Card', label: 'Credit Card (信用卡)' },
                    { value: 'Stripe', label: 'Stripe' },
                    { value: 'Bank Transfer', label: 'Bank Transfer (银行转账)' },
                    { value: 'WeChat Pay', label: 'WeChat Pay (微信支付)' },
                    { value: 'UnionPay', label: 'UnionPay (银联)' },
                    { value: 'Apple Pay', label: 'Apple Pay' },
                    { value: 'Google Pay', label: 'Google Pay' },
                  ]}
                  placeholder="选择或输入支付方式..."
                  classNamePrefix="react-select"
                  styles={{
                    control: (base) => ({ ...base, minHeight: '42px', borderColor: 'rgb(203 213 225)', boxShadow: '0 1px 2px 0 rgb(0 0 0 / 0.05)', '&:hover': { borderColor: 'rgb(99 102 241)' }}),
                    input: (base) => ({ ...base, margin: '0px', paddingBottom: '0px', paddingTop: '0px' }),
                    valueContainer: (base) => ({ ...base, padding: '0px 8px' }),
                    placeholder: (base) => ({ ...base, color: 'rgb(100 116 139)' })
                  }}
                />
              </div>
 
              <div className="mt-4">
                <label htmlFor="renewalNotes" className="block text-sm font-medium text-slate-700 mb-1">续费备注</label>
                <textarea
                  id="renewalNotes"
                  value={renewalNotes}
                  onChange={(e) => setRenewalNotes(e.target.value)}
                  rows={3}
                  placeholder="例如: 优惠码 XYZ, 自动续费已绑定信用卡"
                  className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                />
              </div>

              <div className="mt-4 flex items-center">
                <input
                  id="autoRenewEnabled"
                  name="autoRenewEnabled"
                  type="checkbox"
                  checked={autoRenewEnabled}
                  onChange={(e) => setAutoRenewEnabled(e.target.checked)}
                  className="h-4 w-4 text-indigo-600 border-slate-300 rounded focus:ring-indigo-500"
                />
                <label htmlFor="autoRenewEnabled" className="ml-2 block text-sm text-slate-900">
                  启用自动续费
                </label>
              </div>
                </>
              )}
              {/* End of conditional rendering for renewalCycle */}
              </div> // Closing div for activeTab === 'renewal'
            )}
          </div>
 
          {error && <p className="text-red-500 text-sm mt-4">错误: {error}</p>}

          <div className="mt-6 flex justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="bg-slate-200 hover:bg-slate-300 text-slate-800 font-semibold py-2 px-4 rounded-lg shadow-sm transition-colors duration-150"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={isLoading}
              className="bg-indigo-600 hover:bg-indigo-700 text-white font-semibold py-2 px-4 rounded-lg shadow-sm transition-colors duration-150 disabled:bg-indigo-400 disabled:cursor-not-allowed"
            >
              {isLoading ? '保存中...' : '保存更改'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default EditVpsModal;
