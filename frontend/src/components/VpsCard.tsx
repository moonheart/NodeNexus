import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import type { VpsListItemResponse, ServerStatus } from '../types';
import {
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
  CpuChipIcon,
  MemoryStickIcon,
  HardDiskIcon,
  GlobeAltIcon,
  ArrowUpIcon,
  ArrowDownIcon,
  PencilIcon,
} from './Icons'; // Assuming Icons.tsx is in the same directory or adjust path
import { STATUS_ONLINE, STATUS_OFFLINE, STATUS_REBOOTING, STATUS_PROVISIONING, STATUS_ERROR, STATUS_UNKNOWN } from '../types';

interface VpsCardProps {
  server: VpsListItemResponse;
  onEdit: (server: VpsListItemResponse) => void;
}

const getStatusAppearance = (status: ServerStatus): { cardBorderClass: string; badgeBgClass: string; textClass: string; icon?: React.ReactNode } => {
  switch (status) {
    case STATUS_ONLINE:
      return { cardBorderClass: 'border-green-500', badgeBgClass: 'bg-green-500', textClass: 'text-green-700', icon: <CheckCircleIcon className="w-4 h-4" /> };
    case STATUS_OFFLINE:
      return { cardBorderClass: 'border-red-500', badgeBgClass: 'bg-red-500', textClass: 'text-red-700', icon: <XCircleIcon className="w-4 h-4" /> };
    case STATUS_REBOOTING:
      return { cardBorderClass: 'border-yellow-500', badgeBgClass: 'bg-yellow-500', textClass: 'text-yellow-700', icon: <ExclamationTriangleIcon className="w-4 h-4" /> };
    case STATUS_PROVISIONING:
      return { cardBorderClass: 'border-blue-500', badgeBgClass: 'bg-blue-500', textClass: 'text-blue-700', icon: <ExclamationTriangleIcon className="w-4 h-4" /> };
    case STATUS_ERROR:
      return { cardBorderClass: 'border-red-700', badgeBgClass: 'bg-red-700', textClass: 'text-red-800', icon: <XCircleIcon className="w-4 h-4" /> };
    case STATUS_UNKNOWN: // Explicitly handle UNKNOWN
    default: // Fallback for any other unhandled statuses
      return { cardBorderClass: 'border-slate-400', badgeBgClass: 'bg-slate-400', textClass: 'text-slate-700', icon: <ExclamationTriangleIcon className="w-4 h-4" /> };
  }
};

const getUsageColorClass = (value: number): string => {
  if (value > 90) return 'bg-red-500';
  if (value > 70) return 'bg-yellow-500';
  return 'bg-green-500';
};

const ProgressBar: React.FC<{ value: number; colorClass: string }> = ({ value, colorClass }) => (
  <div className="w-full bg-slate-200 rounded-full h-2 dark:bg-slate-700">
    <div className={`${colorClass} h-2 rounded-full`} style={{ width: `${Math.max(0, Math.min(value, 100))}%` }}></div>
  </div>
);

const formatNetworkSpeed = (bps: number | undefined | null): string => {
  if (typeof bps !== 'number' || bps === null) return 'N/A';
  if (bps < 1024) return `${bps.toFixed(0)} B/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
};

const VpsCard: React.FC<VpsCardProps> = ({ server, onEdit }) => {
  const { cardBorderClass, badgeBgClass, textClass: statusTextClass } = getStatusAppearance(server.status);
  const metrics = server.latestMetrics;

  const cpuUsage = metrics?.cpuUsagePercent ?? null;
  const memoryUsageBytes = metrics?.memoryUsageBytes ?? null;
  const memoryTotalBytes = metrics?.memoryTotalBytes ?? null;
  const memoryUsagePercent = memoryTotalBytes && memoryUsageBytes !== null ? (memoryUsageBytes / memoryTotalBytes) * 100 : null;
  
  const diskUsedBytes = metrics?.diskUsedBytes ?? null;
  const diskTotalBytes = metrics?.diskTotalBytes ?? null;
  const diskUsagePercent = diskTotalBytes && diskUsedBytes !== null ? (diskUsedBytes / diskTotalBytes) * 100 : null;

  return (
    <div className={`bg-white rounded-lg shadow-md hover:shadow-lg transition-shadow duration-300 overflow-hidden flex flex-col border-l-4 ${cardBorderClass}`}>
      <div className="p-4">
        <div className="flex items-center justify-between mb-1">
          <h3 className="text-base font-semibold text-slate-800 truncate" title={server.name}>
            <RouterLink to={`/vps/${server.id}`} className="hover:text-indigo-600 transition-colors">
              {server.name}
            </RouterLink>
          </h3>
          <span className={`px-2 py-0.5 text-xs font-semibold rounded-full text-white ${badgeBgClass}`}>
            {server.status.toUpperCase()}
          </span>
        </div>
        <p className="text-xs text-slate-500 flex items-center mb-1">
          <GlobeAltIcon className="w-3.5 h-3.5 mr-1.5 text-slate-400 flex-shrink-0" />
          {server.ipAddress || 'N/A'}
        </p>
        {server.tags && (
          <div className="mt-2 flex flex-wrap gap-1">
            {server.tags.split(',').map(tag => tag.trim()).filter(tag => tag).map((tag, index) => (
              <span key={index} className="px-2 py-0.5 text-xs font-medium rounded-full bg-sky-100 text-sky-800">
                {tag}
              </span>
            ))}
          </div>
        )}
        {/* Optional: Add OS Type or other quick info if available and desired */}
        {/* <p className="text-xs text-slate-500">{server.osType || 'Unknown OS'}</p> */}
      </div>

      <div className="p-4 space-y-3 border-t border-slate-200 flex-grow">
        {cpuUsage !== null && (
          <div className="text-xs text-slate-600">
            <div className="flex items-center mb-0.5">
              <CpuChipIcon className="w-4 h-4 mr-1.5 text-indigo-500 flex-shrink-0" />
              <span>CPU: <span className={`font-semibold ${statusTextClass}`}>{cpuUsage.toFixed(1)}%</span></span>
            </div>
            <ProgressBar value={cpuUsage} colorClass={getUsageColorClass(cpuUsage)} />
          </div>
        )}

        {memoryUsagePercent !== null && memoryTotalBytes !== null && memoryUsageBytes !== null && (
          <div className="text-xs text-slate-600">
            <div className="flex items-center mb-0.5">
              <MemoryStickIcon className="w-4 h-4 mr-1.5 text-purple-500 flex-shrink-0" />
              <span>RAM: <span className={`font-semibold ${statusTextClass}`}>{memoryUsagePercent.toFixed(1)}%</span>
                <span className="text-slate-500 text-xxs"> ({ (memoryUsageBytes / (1024*1024)).toFixed(0) }MB / { (memoryTotalBytes / (1024*1024)).toFixed(0) }MB)</span>
              </span>
            </div>
            <ProgressBar value={memoryUsagePercent} colorClass={getUsageColorClass(memoryUsagePercent)} />
          </div>
        )}
        
        {diskUsagePercent !== null && diskTotalBytes !== null && diskUsedBytes !== null && (
          <div className="text-xs text-slate-600">
            <div className="flex items-center mb-0.5">
              <HardDiskIcon className="w-4 h-4 mr-1.5 text-orange-500 flex-shrink-0" />
              <span>Disk: <span className={`font-semibold ${statusTextClass}`}>{diskUsagePercent.toFixed(1)}%</span>
              <span className="text-slate-500 text-xxs"> ({ (diskUsedBytes / (1024*1024*1024)).toFixed(1) }GB / { (diskTotalBytes / (1024*1024*1024)).toFixed(1) }GB)</span>
              </span>
            </div>
            <ProgressBar value={diskUsagePercent} colorClass={getUsageColorClass(diskUsagePercent)} />
          </div>
        )}

        <div className="flex justify-between text-xs text-slate-600 pt-1">
            <div className="flex items-center">
                <ArrowUpIcon className="w-3.5 h-3.5 mr-1 text-emerald-500"/> {formatNetworkSpeed(metrics?.networkTxInstantBps)}
            </div>
            <div className="flex items-center">
                <ArrowDownIcon className="w-3.5 h-3.5 mr-1 text-sky-500"/> {formatNetworkSpeed(metrics?.networkRxInstantBps)}
            </div>
        </div>
      </div>

      <div className="p-3 bg-slate-50 border-t border-slate-200 grid grid-cols-2 gap-2">
       <RouterLink
         to={`/vps/${server.id}`}
         className="block w-full text-center bg-indigo-500 hover:bg-indigo-600 text-white font-medium py-1.5 px-3 rounded-md transition-colors duration-200 text-sm"
       >
         查看详情
       </RouterLink>
       <button
         onClick={() => onEdit(server)}
         className="w-full text-center bg-slate-200 hover:bg-slate-300 text-slate-700 font-medium py-1.5 px-3 rounded-md transition-colors duration-200 text-sm flex items-center justify-center"
       >
         <PencilIcon className="w-4 h-4 mr-1.5" />
         编辑
       </button>
      </div>
    </div>
  );
};

export default VpsCard;