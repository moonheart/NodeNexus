import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import type { VpsListItemResponse } from '../types';
import {
  CpuChipIcon,
  MemoryStickIcon,
  HardDiskIcon,
  ArrowUpIcon,
  ArrowDownIcon,
  PencilIcon,
  SignalIcon,
} from './Icons';
import {
  formatBytesForDisplay,
  getUsageColorClass,
  formatNetworkSpeed,
  calculateRenewalInfo,
  calculateTrafficUsage,
  getVpsStatusAppearance,
  SharedProgressBar,
  RenderVpsTags,
} from '../utils/vpsUtils';

interface VpsCardProps {
  server: VpsListItemResponse;
  onEdit?: (server: VpsListItemResponse) => void;
  isSelected?: boolean;
  onSelectionChange?: (vpsId: number, isSelected: boolean) => void;
}

const VpsCard: React.FC<VpsCardProps> = ({ server, onEdit, isSelected, onSelectionChange }) => {
  const { cardBorderClass, cardBadgeBgClass, cardTextClass } = getVpsStatusAppearance(server.status);
  const metrics = server.latestMetrics;

  const cpuUsage = metrics?.cpuUsagePercent ?? null;
  const memoryUsageBytes = metrics?.memoryUsageBytes ?? null;
  const memoryTotalBytes = metrics?.memoryTotalBytes ?? null;
  const memoryUsagePercent = memoryTotalBytes && memoryUsageBytes !== null ? (memoryUsageBytes / memoryTotalBytes) * 100 : null;
  
  const diskUsedBytes = metrics?.diskUsedBytes ?? null;
  const diskTotalBytes = metrics?.diskTotalBytes ?? null;
  const diskUsagePercent = diskTotalBytes && diskUsedBytes !== null ? (diskUsedBytes / diskTotalBytes) * 100 : null;

  const { usedTrafficBytes, trafficUsagePercent } = calculateTrafficUsage(server);

  const renewalInfo = calculateRenewalInfo(
    server.nextRenewalDate,
    server.lastRenewalDate,
    server.serviceStartDate,
    server.renewalCycle,
    server.renewalCycleCustomDays
  );

  return (
    <div className={`relative bg-white rounded-lg shadow-md hover:shadow-lg transition-shadow duration-300 overflow-hidden flex flex-col border-l-4 ${cardBorderClass}`}>
      {onSelectionChange && (
        <div className="absolute top-2 right-2 z-10">
          <input
            type="checkbox"
            className="checkbox checkbox-primary"
            checked={!!isSelected}
            onChange={(e) => onSelectionChange(server.id, e.target.checked)}
            aria-label={`Select ${server.name}`}
          />
        </div>
      )}
      <div className="p-4">
        <div className="flex items-center justify-between mb-1">
          <h3 className="text-base font-semibold text-slate-800 truncate" title={server.name}>
            <RouterLink to={`/vps/${server.id}`} className="hover:text-indigo-600 transition-colors">
              {server.name}
            </RouterLink>
          </h3>
          <span className={`px-2 py-0.5 text-xs font-semibold rounded-full text-white ${cardBadgeBgClass}`}>
            {server.status.toUpperCase()}
          </span>
        </div>
        <p className="text-xs text-slate-500 flex items-center mb-1">
          {server.metadata?.country_code && (
            <span className={`fi fi-${server.metadata.country_code.toLowerCase()} mr-1.5`}></span>
          )}
          {server.ipAddress || server.osType || 'N/A'}
        </p>
        {server.metadata?.os_name && (
          <p className="text-xs text-slate-500 mb-1 truncate" title={`OS: ${server.metadata.long_os_version} ${server.metadata.kernel_version ? `(${server.metadata.kernel_version})` : ''}`}>
            OS: {server.metadata.long_os_version} {server.metadata.kernel_version ? `(${server.metadata.kernel_version})` : ''}
          </p>
        )}
        {server.metadata?.cpu_static_info?.brand && (
          <p className="text-xs text-slate-500 mb-1 truncate" title={`CPU: ${server.metadata.cpu_static_info.brand}`}>
            CPU: {server.metadata.cpu_static_info.brand}
          </p>
        )}
        <RenderVpsTags tags={server.tags} />
        {/* Optional: Add OS Type or other quick info if available and desired */}
        {/* <p className="text-xs text-slate-500">{server.osType || 'Unknown OS'}</p> */}
      </div>

      <div className="p-4 space-y-3 border-t border-slate-200 flex-grow">
        {cpuUsage !== null && (
          <div className="text-xs text-slate-600">
            <div className="flex items-center mb-0.5">
              <CpuChipIcon className="w-4 h-4 mr-1.5 text-indigo-500 flex-shrink-0" />
              <span>CPU: <span className={`font-semibold ${cardTextClass}`}>{cpuUsage.toFixed(1)}%</span></span>
            </div>
            <SharedProgressBar value={cpuUsage} colorClass={getUsageColorClass(cpuUsage)} />
          </div>
        )}

        {memoryUsagePercent !== null && memoryTotalBytes !== null && memoryUsageBytes !== null && (
          <div className="text-xs text-slate-600">
            <div className="flex items-center mb-0.5">
              <MemoryStickIcon className="w-4 h-4 mr-1.5 text-purple-500 flex-shrink-0" />
              <span>RAM: <span className={`font-semibold ${cardTextClass}`}>{memoryUsagePercent.toFixed(1)}%</span>
                <span className="text-slate-500 text-xxs"> ({formatBytesForDisplay(memoryUsageBytes, 1)} / {formatBytesForDisplay(memoryTotalBytes, 1)})</span>
              </span>
            </div>
            <SharedProgressBar value={memoryUsagePercent} colorClass={getUsageColorClass(memoryUsagePercent)} />
          </div>
        )}
        
        {diskUsagePercent !== null && diskTotalBytes !== null && diskUsedBytes !== null && (
          <div className="text-xs text-slate-600">
            <div className="flex items-center mb-0.5">
              <HardDiskIcon className="w-4 h-4 mr-1.5 text-orange-500 flex-shrink-0" />
              <span>Disk: <span className={`font-semibold ${cardTextClass}`}>{diskUsagePercent.toFixed(1)}%</span>
              <span className="text-slate-500 text-xxs"> ({ (diskUsedBytes / (1024*1024*1024)).toFixed(1) }GB / { (diskTotalBytes / (1024*1024*1024)).toFixed(1) }GB)</span>
              </span>
            </div>
            <SharedProgressBar value={diskUsagePercent} colorClass={getUsageColorClass(diskUsagePercent)} />
          </div>
        )}

        {/* Traffic Usage Progress Bar */}
        {server.trafficLimitBytes && server.trafficLimitBytes > 0 && trafficUsagePercent !== null && usedTrafficBytes !== null && (
          <div className="text-xs text-slate-600">
            <div className="flex items-center justify-between mb-0.5">
              <div className="flex items-center">
                <SignalIcon className="w-4 h-4 mr-1.5 text-cyan-500 flex-shrink-0" />
                <span>流量: <span className={`font-semibold ${cardTextClass}`}>{trafficUsagePercent.toFixed(1)}%</span></span>
              </div>
              <span className="text-slate-500 text-xxs">
                {formatBytesForDisplay(usedTrafficBytes)} / {formatBytesForDisplay(server.trafficLimitBytes)}
              </span>
            </div>
            <SharedProgressBar value={trafficUsagePercent} colorClass={getUsageColorClass(trafficUsagePercent)} />
          </div>
        )}

        {/* Renewal Progress Bar */}
        {renewalInfo.isApplicable && renewalInfo.progressPercent !== null && (
          <div className="text-xs text-slate-600">
            <div className="flex items-center justify-between mb-0.5">
              <div className="flex items-center">
                 <SignalIcon className="w-4 h-4 mr-1.5 text-blue-500 flex-shrink-0" /> {/* Placeholder Icon */}
                <span>续费: <span className={`font-semibold ${renewalInfo.colorClass.replace('bg-', 'text-')}`}>{renewalInfo.statusText}</span></span>
              </div>
            </div>
            <SharedProgressBar value={renewalInfo.progressPercent} colorClass={renewalInfo.colorClass} />
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

      <div className={`p-3 bg-slate-50 border-t border-slate-200 grid ${onEdit ? 'grid-cols-2' : 'grid-cols-1'} gap-2`}>
       <RouterLink
         to={`/vps/${server.id}`}
         className="block w-full text-center bg-indigo-500 hover:bg-indigo-600 text-white font-medium py-1.5 px-3 rounded-md transition-colors duration-200 text-sm"
       >
         查看详情
       </RouterLink>
       {onEdit && (
        <button
          onClick={() => onEdit(server)}
          className="w-full text-center bg-slate-200 hover:bg-slate-300 text-slate-700 font-medium py-1.5 px-3 rounded-md transition-colors duration-200 text-sm flex items-center justify-center"
        >
          <PencilIcon className="w-4 h-4 mr-1.5" />
          编辑
        </button>
       )}
      </div>
    </div>
  );
};

export default VpsCard;