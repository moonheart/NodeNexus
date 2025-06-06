import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import type { VpsListItemResponse, ServerStatus as ServerStatusType } from '../types';
import {
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
  ArrowUpIcon,
  ArrowDownIcon,
} from './Icons';
import { STATUS_ONLINE, STATUS_OFFLINE, STATUS_REBOOTING, STATUS_PROVISIONING, STATUS_ERROR, STATUS_UNKNOWN } from '../types';

interface VpsTableRowProps {
  server: VpsListItemResponse;
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

const VpsTableRow: React.FC<VpsTableRowProps> = ({ server }) => {
  const { badgeClass, textClass, icon } = getStatusAppearance(server.status);
  const metrics = server.latestMetrics;

  const cpuUsage = metrics ? `${metrics.cpuUsagePercent.toFixed(1)}%` : 'N/A';
  const memUsage = metrics && metrics.memoryTotalBytes > 0
    ? `${(metrics.memoryUsageBytes / (1024 * 1024)).toFixed(0)}MB / ${(metrics.memoryTotalBytes / (1024 * 1024)).toFixed(0)}MB`
    : 'N/A';
  const upSpeed = metrics ? formatNetworkSpeed(metrics.networkTxInstantBps) : 'N/A';
  const downSpeed = metrics ? formatNetworkSpeed(metrics.networkRxInstantBps) : 'N/A';

  return (
    <tr className="bg-white hover:bg-slate-50 transition-colors duration-150 border-b border-slate-200 last:border-b-0">
      <td className="px-4 py-3 text-sm font-medium text-slate-800 truncate" title={server.name}>
        <RouterLink to={`/vps/${server.id}`} className="text-indigo-600 hover:text-indigo-700 hover:underline">
          {server.name}
        </RouterLink>
      </td>
      <td className="px-4 py-3 text-sm">
        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold ${badgeClass} ${textClass}`}>
          {icon && <span className="mr-1.5">{icon}</span>}
          {server.status.toUpperCase()}
        </span>
      </td>
      <td className="px-4 py-3 text-sm text-slate-600">{server.ipAddress || 'N/A'}</td>
      <td className="px-4 py-3 text-sm text-slate-600">{cpuUsage}</td>
      <td className="px-4 py-3 text-sm text-slate-600">{memUsage}</td>
      <td className="px-4 py-3 text-sm text-slate-600 whitespace-nowrap">
        <ArrowUpIcon className="w-3.5 h-3.5 mr-1 text-emerald-500 inline-block" /> {upSpeed}
      </td>
      <td className="px-4 py-3 text-sm text-slate-600 whitespace-nowrap">
        <ArrowDownIcon className="w-3.5 h-3.5 mr-1 text-sky-500 inline-block" /> {downSpeed}
      </td>
      <td className="px-4 py-3 text-center">
        <RouterLink
          to={`/vps/${server.id}`}
          className="text-indigo-600 hover:text-indigo-700 font-medium text-xs py-1 px-3 rounded-md hover:bg-indigo-50 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-opacity-50 transition-colors"
          aria-label={`View details for ${server.name}`}
        >
          详情
        </RouterLink>
      </td>
    </tr>
  );
};

export default VpsTableRow;