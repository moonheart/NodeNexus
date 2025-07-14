import type { VpsListItemResponse, ServerStatus as ServerStatusType } from '../types';
import { CheckCircle, XCircle, AlertTriangle, Power, HelpCircle } from 'lucide-react';
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

export const formatNetworkSpeed = (bps: number | undefined | null): string => {
  if (typeof bps !== 'number' || bps === null) return 'N/A';
  if (bps < 1024) return `${bps.toFixed(0)} B/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
};

// Helper to format uptime
export const formatUptime = (totalSeconds: number | null | undefined): string => {
  if (totalSeconds == null || totalSeconds < 0) return 'N/A';
  if (totalSeconds === 0) return '0 seconds';
  const days = Math.floor(totalSeconds / (3600 * 24));
  const hours = Math.floor((totalSeconds % (3600 * 24)) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);
  let uptimeString = '';
  if (days > 0) uptimeString += `${days}d `;
  if (hours > 0) uptimeString += `${hours}h `;
  if (minutes > 0) uptimeString += `${minutes}m `;
  if (seconds > 0 || uptimeString === '') uptimeString += `${seconds}s`;
  return uptimeString.trim();
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
  variant: 'default' | 'destructive' | 'secondary';
  isApplicable: boolean;
} => {
  if (!nextRenewalDateStr) {
    return { remainingDays: null, progressPercent: null, statusText: 'N/A', variant: 'secondary', isApplicable: false };
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
  let variant: 'default' | 'destructive' | 'secondary' = 'default';

  if (remainingDays === null) {
    statusText = 'N/A';
    variant = 'secondary';
  } else if (remainingDays < 0) {
    statusText = `过期 ${Math.abs(remainingDays)}天`;
    variant = 'destructive';
    progressPercent = 100;
  } else if (remainingDays === 0) {
    statusText = '今天到期';
    variant = 'destructive';
  } else if (remainingDays <= 7) {
    statusText = `剩 ${remainingDays}天`;
    variant = 'destructive';
  } else if (remainingDays <= 15) {
    statusText = `剩 ${remainingDays}天`;
    variant = 'default'; // Yellow is not a standard variant
  } else {
    statusText = `剩 ${remainingDays}天`;
  }

  return {
    remainingDays,
    progressPercent: progressPercent !== null ? Math.max(0, Math.min(progressPercent, 100)) : null,
    statusText,
    variant,
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
  icon: React.ElementType;
  variant: 'default' | 'destructive' | 'secondary' | 'outline' | 'success';
  cardBorderClass: string;
}

export const getVpsStatusAppearance = (status: ServerStatusType): VpsStatusAppearance => {
  switch (status) {
    case STATUS_ONLINE:
      return { icon: CheckCircle, variant: 'success', cardBorderClass: 'border-success' };
    case STATUS_OFFLINE:
      return { icon: XCircle, variant: 'destructive', cardBorderClass: 'border-destructive' };
    case STATUS_REBOOTING:
      return { icon: Power, variant: 'secondary', cardBorderClass: 'border-warning' };
    case STATUS_PROVISIONING:
      return { icon: AlertTriangle, variant: 'secondary', cardBorderClass: 'border-primary' };
    case STATUS_ERROR:
      return { icon: XCircle, variant: 'destructive', cardBorderClass: 'border-destructive' };
    case STATUS_UNKNOWN:
    default:
      return { icon: HelpCircle, variant: 'outline', cardBorderClass: 'border-border' };
  }
};
export const getProgressVariantClass = (value: number | null | undefined): string => {
  if (value === null || typeof value === 'undefined') {
    return '[&_[data-slot=progress-indicator]]:bg-muted';
  }
  if (value >= 90) {
    return '[&_[data-slot=progress-indicator]]:bg-destructive';
  }
  if (value >= 70) {
    return '[&_[data-slot=progress-indicator]]:bg-warning';
  }
  return '[&_[data-slot=progress-indicator]]:bg-success';
};