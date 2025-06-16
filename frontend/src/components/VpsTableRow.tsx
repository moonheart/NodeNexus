import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import type { VpsListItemResponse } from '../types';
import {
  ArrowUpIcon,
  ArrowDownIcon,
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

interface VpsTableRowProps {
  server: VpsListItemResponse;
  onSelectionChange?: (vpsId: number, isSelected: boolean) => void;
}

const VpsTableRow: React.FC<VpsTableRowProps> = ({ server }) => {
  const { tableRowBadgeClass, tableRowTextClass, icon } = getVpsStatusAppearance(server.status);
  const metrics = server.latestMetrics;

  const memUsagePercent = metrics && metrics.memoryTotalBytes > 0
    ? (metrics.memoryUsageBytes / metrics.memoryTotalBytes) * 100
    : null;
  const upSpeed = metrics ? formatNetworkSpeed(metrics.networkTxInstantBps) : 'N/A';
  const downSpeed = metrics ? formatNetworkSpeed(metrics.networkRxInstantBps) : 'N/A';

  const { usedTrafficBytes, trafficUsagePercent } = calculateTrafficUsage(server);

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
          <div className="truncate" title={server.name}>
            <RouterLink to={`/vps/${server.id}`} className="text-indigo-600 hover:text-indigo-700 hover:underline">
              {server.name}
            </RouterLink>
            <RenderVpsTags tags={server.tags} />
          </div>
        </div>
      </td>
      <td className="px-4 py-3 text-sm">
        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold ${tableRowBadgeClass} ${tableRowTextClass}`}>
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
      <td className="px-4 py-3 text-sm text-slate-600 truncate" title={server.osType ?? 'N/A'}>
        {server.osType ?? 'N/A'}
      </td>
      <td className="px-4 py-3 text-sm text-slate-600">
        {metrics ? (
          <div className="w-28">
            <div className="flex items-center justify-between text-xs mb-0.5">
              <span className={`font-semibold ${getUsageColorClass(metrics.cpuUsagePercent).replace('bg-', 'text-')}`}>
                {metrics.cpuUsagePercent.toFixed(1)}%
              </span>
            </div>
            <SharedProgressBar value={metrics.cpuUsagePercent} colorClass={getUsageColorClass(metrics.cpuUsagePercent)} heightClass="h-1.5" />
          </div>
        ) : (
          <span className="text-xs text-slate-400">N/A</span>
        )}
      </td>
      <td className="px-4 py-3 text-sm text-slate-600">
        {memUsagePercent !== null && metrics ? (
          <div className="w-28">
            <div className="flex items-center justify-between text-xs mb-0.5">
              <span className={`font-semibold ${getUsageColorClass(memUsagePercent).replace('bg-', 'text-')}`}>
                {memUsagePercent.toFixed(1)}%
              </span>
              <span className="text-slate-500 text-xxs">
                {formatBytesForDisplay(metrics.memoryUsageBytes, 0)}/{formatBytesForDisplay(metrics.memoryTotalBytes, 0)}
              </span>
            </div>
            <SharedProgressBar value={memUsagePercent} colorClass={getUsageColorClass(memUsagePercent)} heightClass="h-1.5" />
          </div>
        ) : (
          <span className="text-xs text-slate-400">N/A</span>
        )}
      </td>
      {/* Traffic Usage Column */}
      <td className="px-4 py-3 text-sm text-slate-600">
        {server.trafficLimitBytes && server.trafficLimitBytes > 0 && trafficUsagePercent !== null && usedTrafficBytes !== null ? (
          <div className="w-28"> {/* Fixed width for the progress bar and text container */}
            <div className="flex items-center justify-between text-xs mb-0.5">
              <span className={`font-semibold ${getUsageColorClass(trafficUsagePercent).replace('bg-', 'text-')}`}>{trafficUsagePercent.toFixed(1)}%</span>
              <span className="text-slate-500 text-xxs">
                {formatBytesForDisplay(usedTrafficBytes, 0)}/{formatBytesForDisplay(server.trafficLimitBytes, 0)}
              </span>
            </div>
            <SharedProgressBar value={trafficUsagePercent} colorClass={getUsageColorClass(trafficUsagePercent)} heightClass="h-1.5" />
          </div>
        ) : server.trafficLimitBytes && server.trafficLimitBytes > 0 ? (
          <span className="text-xs text-slate-400">计算中...</span>
        ) : (
          <span className="text-xs text-slate-400">未配置</span>
        )}
      </td>
      <td className="px-4 py-3 text-sm text-slate-600">
        {renewalInfo.isApplicable && renewalInfo.progressPercent !== null ? (
          <div className="w-28"> {/* Fixed width for consistency */}
            <div className="flex items-center justify-between text-xs mb-0.5">
              <span className={`font-semibold ${renewalInfo.colorClass.replace('bg-', 'text-')}`}>{renewalInfo.statusText}</span>
            </div>
            <SharedProgressBar value={renewalInfo.progressPercent} colorClass={renewalInfo.colorClass} heightClass="h-1.5" />
          </div>
        ) : (
          <span className="text-xs text-slate-400">未配置</span>
        )}
      </td>
      <td className="px-4 py-3 text-sm text-slate-600 whitespace-nowrap w-28">
        <div className="flex items-center">
          <ArrowUpIcon className="w-3.5 h-3.5 mr-1 text-emerald-500 inline-block flex-shrink-0" />
          <span>{upSpeed}</span>
        </div>
      </td>
      <td className="px-4 py-3 text-sm text-slate-600 whitespace-nowrap w-28">
        <div className="flex items-center">
          <ArrowDownIcon className="w-3.5 h-3.5 mr-1 text-sky-500 inline-block flex-shrink-0" />
          <span>{downSpeed}</span>
        </div>
      </td>
    </tr>
  );
};

export default VpsTableRow;