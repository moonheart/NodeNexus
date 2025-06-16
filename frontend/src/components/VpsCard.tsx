import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import type { VpsListItemResponse } from '../types';
import {
  CpuChipIcon,
  MemoryStickIcon,
  HardDiskIcon,
  ArrowUpIcon,
  ArrowDownIcon,
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
}

const VpsCard: React.FC<VpsCardProps> = ({ server }) => {
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
    <div className={`bg-white rounded-lg shadow-md hover:shadow-lg transition-shadow duration-300 overflow-hidden flex flex-col border-l-4 ${cardBorderClass}`}>
      <RouterLink to={`/vps/${server.id}`} className="p-4 block hover:bg-slate-50 transition-colors">
        <div className="flex items-center justify-between mb-1">
          <h3 className="text-base font-semibold text-slate-800 truncate" title={server.name}>
            {server.name}
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
      </RouterLink>
      <div className="px-4 pb-4">
        <RenderVpsTags tags={server.tags} />
      </div>

      <div className="p-4 space-y-3 border-t border-slate-200 flex-grow">
        <div className="space-y-3">
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
      </div>
    </div>
  );
};

export default VpsCard;