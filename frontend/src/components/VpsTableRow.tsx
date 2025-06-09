import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import type { VpsListItemResponse, ServerStatus as ServerStatusType } from '../types';
import {
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
  ArrowUpIcon,
  ArrowDownIcon,
  PencilIcon,
} from './Icons';
import { STATUS_ONLINE, STATUS_OFFLINE, STATUS_REBOOTING, STATUS_PROVISIONING, STATUS_ERROR, STATUS_UNKNOWN } from '../types';

// Helper function to format bytes into a readable string (e.g., "10.5 GB")
const formatBytesForDisplay = (bytes: number | null | undefined, decimals = 1): string => {
  if (bytes === null || typeof bytes === 'undefined' || bytes === 0) return '0 B';
  if (bytes < 0) return 'N/A';

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  
  if (i >= sizes.length) return parseFloat((bytes / Math.pow(k, sizes.length -1)).toFixed(dm)) + ' ' + sizes[sizes.length -1];

  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
};

const getUsageColorClass = (value: number): string => {
  if (value > 90) return 'bg-red-500';
  if (value > 70) return 'bg-yellow-500';
  return 'bg-green-500';
};

const ProgressBar: React.FC<{ value: number; colorClass: string }> = ({ value, colorClass }) => (
  <div className="w-full bg-slate-200 rounded-full h-1.5 dark:bg-slate-700">
    <div className={`${colorClass} h-1.5 rounded-full`} style={{ width: `${Math.max(0, Math.min(value, 100))}%` }}></div>
  </div>
);

interface VpsTableRowProps {
  server: VpsListItemResponse;
  onEdit: (server: VpsListItemResponse) => void;
  isSelected: boolean;
  onSelectionChange: (vpsId: number, isSelected: boolean) => void;
}

const getStatusAppearance = (status: ServerStatusType): { badgeClass: string; textClass: string; icon: React.ReactNode } => {
  switch (status) {
    case STATUS_ONLINE:
      return { badgeClass: 'bg-green-100', textClass: 'text-green-700', icon: <CheckCircleIcon className="w-4 h-4" /> };
    case STATUS_OFFLINE:
      return { badgeClass: 'bg-red-100', textClass: 'text-red-700', icon: <XCircleIcon className="w-4 h-4" /> };
    case STATUS_REBOOTING:
      return { badgeClass: 'bg-yellow-100', textClass: 'text-yellow-700', icon: <ExclamationTriangleIcon className="w-4 h-4" /> };
    case STATUS_PROVISIONING:
      return { badgeClass: 'bg-blue-100', textClass: 'text-blue-700', icon: <ExclamationTriangleIcon className="w-4 h-4" /> };
    case STATUS_ERROR:
      return { badgeClass: 'bg-red-200', textClass: 'text-red-800', icon: <XCircleIcon className="w-4 h-4" /> };
    case STATUS_UNKNOWN:
    default:
      return { badgeClass: 'bg-slate-100', textClass: 'text-slate-700', icon: <ExclamationTriangleIcon className="w-4 h-4" /> };
  }
};

const formatNetworkSpeed = (bps: number | undefined | null): string => {
  if (typeof bps !== 'number' || bps === null) return 'N/A';
  if (bps < 1024) return `${bps.toFixed(0)} B/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
};

const getContrastingTextColor = (hexColor: string): string => {
  if (!hexColor) return '#000000';
  const hex = hexColor.replace('#', '');
  if (hex.length !== 6) return '#000000';
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  const yiq = ((r * 299) + (g * 587) + (b * 114)) / 1000;
  return (yiq >= 128) ? '#000000' : '#ffffff';
};

// Helper function to calculate remaining days and progress for renewal (same as in VpsCard.tsx)
const calculateRenewalInfo = (
  nextRenewalDateStr?: string | null,
  lastRenewalDateStr?: string | null,
  serviceStartDateStr?: string | null,
  renewalCycle?: string | null,
  renewalCycleCustomDays?: number | null
): {
  remainingDays: number | null;
  progressPercent: number | null;
  statusText: string;
  colorClass: string;
  isApplicable: boolean;
} => {
  if (!nextRenewalDateStr) {
    return { remainingDays: null, progressPercent: null, statusText: 'N/A', colorClass: 'bg-slate-300', isApplicable: false };
  }

  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const nextRenewalDate = new Date(nextRenewalDateStr);
  nextRenewalDate.setHours(0,0,0,0);

  const timeDiff = nextRenewalDate.getTime() - today.getTime();
  const remainingDays = Math.ceil(timeDiff / (1000 * 3600 * 24));

  let totalCycleDays: number | null = null;
  const referenceStartDateStr = lastRenewalDateStr || serviceStartDateStr;

  if (referenceStartDateStr) {
    const referenceStartDate = new Date(referenceStartDateStr);
    referenceStartDate.setHours(0,0,0,0);
    const cycleTimeDiff = nextRenewalDate.getTime() - referenceStartDate.getTime();
    if (cycleTimeDiff > 0) {
      totalCycleDays = Math.ceil(cycleTimeDiff / (1000 * 3600 * 24));
    }
  }
  
  if (totalCycleDays === null || totalCycleDays <=0 ) {
    if (renewalCycle === 'custom_days' && renewalCycleCustomDays && renewalCycleCustomDays > 0) {
      totalCycleDays = renewalCycleCustomDays;
    } else if (renewalCycle) {
      const estimates: { [key: string]: number } = {
        'monthly': 30, 'quarterly': 91, 'semi_annually': 182, 'annually': 365,
        'biennially': 730, 'triennially': 1095
      };
      totalCycleDays = estimates[renewalCycle] || null;
    }
  }

  let progressPercent: number | null = null;
  if (totalCycleDays && totalCycleDays > 0 && remainingDays !== null) {
    const daysPassed = totalCycleDays - Math.max(0, remainingDays);
    progressPercent = (daysPassed / totalCycleDays) * 100;
  } else if (remainingDays !== null && remainingDays >=0 && remainingDays <= 7) {
    progressPercent = 100 - (remainingDays / 7 * 50);
  }

  let statusText = '';
  let colorClass = 'bg-green-500';

  if (remainingDays === null) {
    statusText = 'N/A';
    colorClass = 'bg-slate-300';
  } else if (remainingDays < 0) {
    statusText = `过期 ${Math.abs(remainingDays)}天`;
    colorClass = 'bg-red-600';
    progressPercent = 100;
  } else if (remainingDays === 0) {
    statusText = '今天到期';
    colorClass = 'bg-red-500';
  } else if (remainingDays <= 7) {
    statusText = `剩 ${remainingDays}天`;
    colorClass = 'bg-red-500';
  } else if (remainingDays <= 15) { // Changed from 30 to 15
    statusText = `剩 ${remainingDays}天`;
    colorClass = 'bg-yellow-500';
  } else {
    statusText = `剩 ${remainingDays}天`;
  }

  return {
    remainingDays,
    progressPercent: progressPercent !== null ? Math.max(0, Math.min(progressPercent, 100)) : null,
    statusText,
    colorClass,
    isApplicable: true,
  };
};


const VpsTableRow: React.FC<VpsTableRowProps> = ({ server, onEdit, isSelected, onSelectionChange }) => {
  const { badgeClass, textClass, icon } = getStatusAppearance(server.status);
  const metrics = server.latestMetrics;

  const cpuUsage = metrics ? `${metrics.cpuUsagePercent.toFixed(1)}%` : 'N/A';
  const memUsage = metrics && metrics.memoryTotalBytes > 0
    ? `${formatBytesForDisplay(metrics.memoryUsageBytes, 1)} / ${formatBytesForDisplay(metrics.memoryTotalBytes, 1)}`
    : 'N/A';
  const upSpeed = metrics ? formatNetworkSpeed(metrics.networkTxInstantBps) : 'N/A';
  const downSpeed = metrics ? formatNetworkSpeed(metrics.networkRxInstantBps) : 'N/A';

  // Traffic usage calculation
  let usedTrafficBytes: number | null = null;
  if (server.trafficBillingRule && server.trafficLimitBytes && server.trafficLimitBytes > 0) {
    const rxBytes = server.trafficCurrentCycleRxBytes ?? 0;
    const txBytes = server.trafficCurrentCycleTxBytes ?? 0;
    switch (server.trafficBillingRule) {
      case 'sum_in_out':
        usedTrafficBytes = rxBytes + txBytes;
        break;
      case 'out_only':
        usedTrafficBytes = txBytes;
        break;
      case 'max_in_out':
        usedTrafficBytes = Math.max(rxBytes, txBytes);
        break;
      default:
        usedTrafficBytes = null;
    }
  }

  const trafficUsagePercent = (server.trafficLimitBytes && usedTrafficBytes !== null && server.trafficLimitBytes > 0)
    ? (usedTrafficBytes / server.trafficLimitBytes) * 100
    : null;

  const renewalInfo = calculateRenewalInfo(
    server.nextRenewalDate,
    server.lastRenewalDate,
    server.serviceStartDate,
    server.renewalCycle,
    server.renewalCycleCustomDays
  );
  
  return (
    <tr className="bg-white hover:bg-slate-50 transition-colors duration-150 border-b border-slate-200 last:border-b-0">
      <td className="px-4 py-3 text-sm font-medium text-slate-800">
        <div className="flex items-center">
          <input
            type="checkbox"
            className="checkbox checkbox-primary checkbox-sm mr-4"
            checked={isSelected}
            onChange={(e) => onSelectionChange(server.id, e.target.checked)}
            aria-label={`Select ${server.name}`}
          />
          <div className="truncate" title={server.name}>
            <RouterLink to={`/vps/${server.id}`} className="text-indigo-600 hover:text-indigo-700 hover:underline">
              {server.name}
            </RouterLink>
            {server.tags && server.tags.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-1">
                {server.tags.filter(tag => tag.isVisible).map(tag => {
                  const tagComponent = (
                    <span
                      className="px-2 py-0.5 text-xs font-medium rounded-full"
                      style={{
                        backgroundColor: tag.color,
                        color: getContrastingTextColor(tag.color),
                      }}
                    >
                      {tag.name}
                    </span>
                  );

                  if (tag.url) {
                    return (
                      <a href={tag.url} target="_blank" rel="noopener noreferrer" key={tag.id}>
                        {tagComponent}
                      </a>
                    );
                  }
                  return <div key={tag.id}>{tagComponent}</div>;
                })}
              </div>
            )}
          </div>
        </div>
      </td>
      <td className="px-4 py-3 text-sm">
        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold ${badgeClass} ${textClass}`}>
          {icon && <span className="mr-1.5">{icon}</span>}
          {server.status.toUpperCase()}
        </span>
      </td>
      <td className="px-4 py-3 text-sm text-slate-600">
        <div className="flex items-center">
          {server.metadata?.country_code && (
            <span className={`fi fi-${server.metadata.country_code.toLowerCase()} mr-2`}></span>
          )}
          {server.ipAddress || 'N/A'}
        </div>
      </td>
      <td className="px-4 py-3 text-sm text-slate-600 truncate" title={server.metadata?.os_name ? String(server.metadata.os_name) : 'N/A'}>
        {server.metadata?.os_name ? String(server.metadata.os_name) : 'N/A'}
      </td>
      <td className="px-4 py-3 text-sm text-slate-600">{cpuUsage}</td>
      <td className="px-4 py-3 text-sm text-slate-600">{memUsage}</td>
      {/* Traffic Usage Column */}
      <td className="px-4 py-3 text-sm text-slate-600">
        {server.trafficLimitBytes && server.trafficLimitBytes > 0 && trafficUsagePercent !== null && usedTrafficBytes !== null ? (
          <div className="w-28"> {/* Fixed width for the progress bar and text container */}
            <div className="flex items-center justify-between text-xs mb-0.5">
              <span className={`font-semibold ${textClass}`}>{trafficUsagePercent.toFixed(1)}%</span>
              <span className="text-slate-500 text-xxs">
                {formatBytesForDisplay(usedTrafficBytes, 0)}/{formatBytesForDisplay(server.trafficLimitBytes, 0)}
              </span>
            </div>
            <ProgressBar value={trafficUsagePercent} colorClass={getUsageColorClass(trafficUsagePercent)} />
          </div>
        ) : server.trafficLimitBytes && server.trafficLimitBytes > 0 ? (
          <span className="text-xs text-slate-400">计算中...</span>
        ) : (
          <span className="text-xs text-slate-400">未配置</span>
        )}
      </td>
      {/* Renewal Info Column */}
      <td className="px-4 py-3 text-sm text-slate-600">
        {renewalInfo.isApplicable && renewalInfo.progressPercent !== null ? (
          <div className="w-28"> {/* Fixed width for consistency */}
            <div className="flex items-center justify-between text-xs mb-0.5">
              <span className={`font-semibold ${renewalInfo.colorClass.replace('bg-', 'text-')}`}>{renewalInfo.statusText}</span>
              {/* Optional: Show percentage if space allows or on hover */}
              {/* <span className="text-slate-500">{renewalInfo.progressPercent.toFixed(0)}%</span> */}
            </div>
            <ProgressBar value={renewalInfo.progressPercent} colorClass={renewalInfo.colorClass} />
          </div>
        ) : (
          <span className="text-xs text-slate-400">未配置</span>
        )}
      </td>
      <td className="px-4 py-3 text-sm text-slate-600 whitespace-nowrap">
        <ArrowUpIcon className="w-3.5 h-3.5 mr-1 text-emerald-500 inline-block" /> {upSpeed}
      </td>
      <td className="px-4 py-3 text-sm text-slate-600 whitespace-nowrap">
        <ArrowDownIcon className="w-3.5 h-3.5 mr-1 text-sky-500 inline-block" /> {downSpeed}
      </td>
      <td className="px-4 py-3 text-center">
       <div className="flex items-center justify-center space-x-2">
         <RouterLink
           to={`/vps/${server.id}`}
           className="text-indigo-600 hover:text-indigo-700 font-medium text-xs py-1 px-3 rounded-md hover:bg-indigo-50 transition-colors"
           aria-label={`View details for ${server.name}`}
         >
           详情
         </RouterLink>
         <button
           onClick={() => onEdit(server)}
           className="text-slate-600 hover:text-slate-800 font-medium text-xs py-1 px-3 rounded-md hover:bg-slate-100 transition-colors flex items-center"
           aria-label={`Edit ${server.name}`}
         >
           <PencilIcon className="w-3.5 h-3.5 mr-1" />
           编辑
         </button>
       </div>
      </td>
    </tr>
  );
};

export default VpsTableRow;