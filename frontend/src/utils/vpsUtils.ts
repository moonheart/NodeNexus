import React from 'react';
import type { VpsListItemResponse, ServerStatus as ServerStatusType, Tag } from '../types';
import {
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
} from '../components/Icons';
import { STATUS_ONLINE, STATUS_OFFLINE, STATUS_REBOOTING, STATUS_PROVISIONING, STATUS_ERROR, STATUS_UNKNOWN } from '../types';

// Helper function to format bytes into a readable string (e.g., "10.5 GB")
export const formatBytesForDisplay = (bytes: number | null | undefined, decimals = 1): string => {
  if (bytes === null || typeof bytes === 'undefined' || bytes === 0) return '0 B';
  if (bytes < 0) return 'N/A';

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  
  if (i >= sizes.length) return parseFloat((bytes / Math.pow(k, sizes.length -1)).toFixed(dm)) + ' ' + sizes[sizes.length -1];

  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
};

export const getUsageColorClass = (value: number): string => {
  if (value > 90) return 'bg-red-500';
  if (value > 70) return 'bg-yellow-500';
  return 'bg-green-500';
};

export const formatNetworkSpeed = (bps: number | undefined | null): string => {
  if (typeof bps !== 'number' || bps === null) return 'N/A';
  if (bps < 1024) return `${bps.toFixed(0)} B/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
};

export const getContrastingTextColor = (hexColor: string): string => {
  if (!hexColor) return '#000000';
  const hex = hexColor.replace('#', '');
  if (hex.length !== 6) return '#000000'; // Default for invalid hex
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  const yiq = ((r * 299) + (g * 587) + (b * 114)) / 1000;
  return (yiq >= 128) ? '#000000' : '#ffffff';
};

// Helper function to calculate remaining days and progress for renewal
export const calculateRenewalInfo = (
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
  } else if (remainingDays <= 15) {
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

export const calculateTrafficUsage = (
  server: Pick<VpsListItemResponse, 'trafficBillingRule' | 'trafficLimitBytes' | 'trafficCurrentCycleRxBytes' | 'trafficCurrentCycleTxBytes'>
): { usedTrafficBytes: number | null; trafficUsagePercent: number | null } => {
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

  return { usedTrafficBytes, trafficUsagePercent };
};


export interface VpsStatusAppearance {
  icon: React.ReactNode;
  tableRowBadgeClass: string;
  tableRowTextClass: string;
  cardBorderClass: string;
  cardBadgeBgClass: string;
  cardTextClass: string; 
}

export const getVpsStatusAppearance = (status: ServerStatusType): VpsStatusAppearance => {
  const iconProps = { className: "w-4 h-4" };
  switch (status) {
    case STATUS_ONLINE:
      return {
        icon: React.createElement(CheckCircleIcon, iconProps),
        tableRowBadgeClass: 'bg-green-100',
        tableRowTextClass: 'text-green-700',
        cardBorderClass: 'border-green-500',
        cardBadgeBgClass: 'bg-green-500',
        cardTextClass: 'text-green-700',
      };
    case STATUS_OFFLINE:
      return {
        icon: React.createElement(XCircleIcon, iconProps),
        tableRowBadgeClass: 'bg-red-100',
        tableRowTextClass: 'text-red-700',
        cardBorderClass: 'border-red-500',
        cardBadgeBgClass: 'bg-red-500',
        cardTextClass: 'text-red-700',
      };
    case STATUS_REBOOTING:
      return {
        icon: React.createElement(ExclamationTriangleIcon, iconProps),
        tableRowBadgeClass: 'bg-yellow-100',
        tableRowTextClass: 'text-yellow-700',
        cardBorderClass: 'border-yellow-500',
        cardBadgeBgClass: 'bg-yellow-500',
        cardTextClass: 'text-yellow-700',
      };
    case STATUS_PROVISIONING:
      return {
        icon: React.createElement(ExclamationTriangleIcon, iconProps),
        tableRowBadgeClass: 'bg-blue-100',
        tableRowTextClass: 'text-blue-700',
        cardBorderClass: 'border-blue-500',
        cardBadgeBgClass: 'bg-blue-500',
        cardTextClass: 'text-blue-700',
      };
    case STATUS_ERROR:
      return {
        icon: React.createElement(XCircleIcon, iconProps),
        tableRowBadgeClass: 'bg-red-200', 
        tableRowTextClass: 'text-red-800', 
        cardBorderClass: 'border-red-700', 
        cardBadgeBgClass: 'bg-red-700',   
        cardTextClass: 'text-red-800',
      };
    case STATUS_UNKNOWN:
    default:
      return {
        icon: React.createElement(ExclamationTriangleIcon, iconProps),
        tableRowBadgeClass: 'bg-slate-100',
        tableRowTextClass: 'text-slate-700',
        cardBorderClass: 'border-slate-400',
        cardBadgeBgClass: 'bg-slate-400',
        cardTextClass: 'text-slate-700',
      };
  }
};

interface SharedProgressBarProps {
  value: number;
  colorClass: string;
  heightClass?: string; 
}

export const SharedProgressBar: React.FC<SharedProgressBarProps> = ({ value, colorClass, heightClass = 'h-2' }) => (
  React.createElement("div", { className: `w-full bg-slate-200 rounded-full dark:bg-slate-700 ${heightClass}` },
    React.createElement("div", { className: `${colorClass} ${heightClass} rounded-full`, style: { width: `${Math.max(0, Math.min(value, 100))}%` } })
  )
);


interface RenderVpsTagsProps {
  tags: Tag[] | undefined;
}

export const RenderVpsTags: React.FC<RenderVpsTagsProps> = ({ tags }) => {
  if (!tags || tags.length === 0) {
    return null;
  }

  return (
    React.createElement("div", { className: "mt-2 flex flex-wrap gap-1" },
      tags.filter(tag => tag.isVisible).map(tag => {
        const tagComponent = React.createElement("span",
            {
              key: `${tag.id}-span`,
              className: "px-2 py-0.5 text-xs font-medium rounded-full",
              style: {
                backgroundColor: tag.color,
                color: getContrastingTextColor(tag.color),
              }
            },
            tag.name
          );

        if (tag.url) {
          return (
            React.createElement("a", { href: tag.url, target: "_blank", rel: "noopener noreferrer", key: tag.id },
              tagComponent
            )
          );
        }
        return React.createElement("div", { key: tag.id }, tagComponent);
      })
    )
  );
};