import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import type { VpsListItemResponse } from '../types';
import { ArrowUp, ArrowDown } from 'lucide-react';
import {
  formatBytesForDisplay,
  formatNetworkSpeed,
  calculateRenewalInfo,
  calculateTrafficUsage,
  getVpsStatusAppearance,
 getProgressVariantClass,
} from '../utils/vpsUtils';
import { TableCell, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { Progress } from '@/components/ui/progress';
import { VpsTags } from './VpsTags';

interface VpsTableRowProps {
  server: VpsListItemResponse;
}

const UsageCell: React.FC<{ value: number | null, text: string, tooltipContent?: string }> = ({ value, text, tooltipContent }) => {
   const variantClass = getProgressVariantClass(value);
   const cellContent = (
     <div className="w-28">
       <div className="flex items-center justify-between text-xs mb-0.5">
         <span className="font-semibold">{text}</span>
       </div>
       <Progress value={value ?? 0} className={`h-1.5 ${variantClass}`} />
     </div>
   );

  if (tooltipContent) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          {cellContent}
        </TooltipTrigger>
        <TooltipContent>
          {tooltipContent}
        </TooltipContent>
      </Tooltip>
    );
  }
  return cellContent;
};

const VpsTableRow: React.FC<VpsTableRowProps> = ({ server }) => {
  const { icon: StatusIcon, variant: statusVariant } = getVpsStatusAppearance(server.status);
  const metrics = server.latestMetrics;

  const memUsagePercent = metrics && metrics.memoryTotalBytes > 0 ? (metrics.memoryUsageBytes / metrics.memoryTotalBytes) * 100 : null;
  const upSpeed = metrics ? formatNetworkSpeed(metrics.networkTxInstantBps) : 'N/A';
  const downSpeed = metrics ? formatNetworkSpeed(metrics.networkRxInstantBps) : 'N/A';

  const { usedTrafficBytes, trafficUsagePercent } = calculateTrafficUsage(server);
  const renewalInfo = calculateRenewalInfo(server.nextRenewalDate, server.lastRenewalDate, server.serviceStartDate, server.renewalCycle, server.renewalCycleCustomDays);

  return (
    <TooltipProvider>
      <TableRow>
        <TableCell className="font-medium">
          <div className="flex flex-col">
            <RouterLink to={`/vps/${server.id}`} className="text-primary hover:underline">
              {server.name}
            </RouterLink>
            <VpsTags tags={server.tags} className="mt-1" />
          </div>
        </TableCell>
        <TableCell>
          <Badge variant={statusVariant}>
            <StatusIcon className="w-3.5 h-3.5 mr-1.5" />
            {server.status.toUpperCase()}
          </Badge>
        </TableCell>
        <TableCell>
          <div className="flex items-center">
            {server.metadata?.country_code && (
              <span className={`fi fi-${server.metadata.country_code.toLowerCase()} mr-2`}></span>
            )}
            {server.ipAddress || 'N/A'}
          </div>
        </TableCell>
        <TableCell className="truncate" title={server.osType ?? 'N/A'}>{server.osType ?? 'N/A'}</TableCell>
        <TableCell>
          {metrics ? (
            <UsageCell value={metrics.cpuUsagePercent} text={`${metrics.cpuUsagePercent.toFixed(1)}%`} />
          ) : <span className="text-xs text-muted-foreground">N/A</span>}
        </TableCell>
        <TableCell>
          {memUsagePercent !== null && metrics ? (
            <UsageCell
              value={memUsagePercent}
              text={`${memUsagePercent.toFixed(1)}%`}
              tooltipContent={`${formatBytesForDisplay(metrics.memoryUsageBytes, 0)} / ${formatBytesForDisplay(metrics.memoryTotalBytes, 0)}`}
            />
          ) : <span className="text-xs text-muted-foreground">N/A</span>}
        </TableCell>
        <TableCell>
          {server.trafficLimitBytes && server.trafficLimitBytes > 0 ? (
            trafficUsagePercent !== null ? (
              <UsageCell
                value={trafficUsagePercent}
                text={`${trafficUsagePercent.toFixed(1)}%`}
                tooltipContent={`${formatBytesForDisplay(usedTrafficBytes, 0)} / ${formatBytesForDisplay(server.trafficLimitBytes, 0)}`}
              />
            ) : <span className="text-xs text-muted-foreground">计算中...</span>
          ) : <span className="text-xs text-muted-foreground">未配置</span>}
        </TableCell>
        <TableCell>
          {renewalInfo.isApplicable ? (
            <UsageCell
              value={renewalInfo.progressPercent}
              text={renewalInfo.statusText}
            />
          ) : <span className="text-xs text-muted-foreground">未配置</span>}
        </TableCell>
        <TableCell className="whitespace-nowrap">
          <div className="flex items-center">
            <ArrowUp className="w-3.5 h-3.5 mr-1 text-emerald-500" />
            <span>{upSpeed}</span>
          </div>
        </TableCell>
        <TableCell className="whitespace-nowrap">
          <div className="flex items-center">
            <ArrowDown className="w-3.5 h-3.5 mr-1 text-sky-500" />
            <span>{downSpeed}</span>
          </div>
        </TableCell>
      </TableRow>
    </TooltipProvider>
  );
};

export default VpsTableRow;