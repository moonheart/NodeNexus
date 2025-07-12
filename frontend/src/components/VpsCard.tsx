import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { VpsListItemResponse } from '../types';
import { Cpu, MemoryStick, HardDrive, ArrowUp, ArrowDown, Signal, CalendarClock } from 'lucide-react';
import {
  formatBytesForDisplay,
  formatNetworkSpeed,
  calculateRenewalInfo,
  calculateTrafficUsage,
  getVpsStatusAppearance,
} from '../utils/vpsUtils';
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { UsageProgress } from './UsageProgress';
import { VpsMetricsChart } from './VpsMetricsChart';
import { VpsTags } from './VpsTags';
import { cn } from '@/lib/utils';

interface VpsCardProps {
  server: VpsListItemResponse;
  activeChartTab: string;
  onChartTabChange: (tab: string) => void;
}

const VpsCard: React.FC<VpsCardProps> = React.memo(({ server, activeChartTab, onChartTabChange }) => {
  const { t } = useTranslation();
  const { icon: StatusIcon, variant: statusVariant, cardBorderClass } = getVpsStatusAppearance(server.status);
  const metrics = server.latestMetrics;

  const cpuUsage = metrics?.cpuUsagePercent ?? null;
  const memoryUsageBytes = metrics?.memoryUsageBytes ?? null;
  const memoryTotalBytes = metrics?.memoryTotalBytes ?? null;
  const memoryUsagePercent = memoryTotalBytes && memoryUsageBytes !== null ? (memoryUsageBytes / memoryTotalBytes) * 100 : null;

  const diskUsedBytes = metrics?.diskUsedBytes ?? null;
  const diskTotalBytes = metrics?.diskTotalBytes ?? null;
  const diskUsagePercent = diskTotalBytes && diskUsedBytes !== null ? (diskUsedBytes / diskTotalBytes) * 100 : null;

  const { usedTrafficBytes, trafficUsagePercent } = calculateTrafficUsage(server);
  const renewalInfo = calculateRenewalInfo(server.nextRenewalDate, server.lastRenewalDate, server.serviceStartDate, server.renewalCycle, server.renewalCycleCustomDays);

  return (
    <TooltipProvider>
      <Card className={cn("flex flex-col transition-shadow duration-300 hover:shadow-lg py-0 gap-2", cardBorderClass)}>
        <RouterLink to={`/vps/${server.id}`} className="hover:bg-muted/50 transition-colors">
          <CardHeader className="pb-2 pt-4 px-4 rounded-t-xl bg-secondary/20">
            <CardTitle className="text-base font-semibold truncate" title={server.name}>
              {server.name}
            </CardTitle>
            <CardDescription>
              <div className="text-xs text-muted-foreground flex items-center">
                {server.metadata?.country_code && (
                  <span className={`fi fi-${server.metadata.country_code.toLowerCase()} mr-1.5`}></span>
                )}
                {server.ipAddress || t('vps.noIpAddress')}
              </div>
            </CardDescription>
            <CardAction>
              <Badge variant={statusVariant} className="flex-shrink-0 rounded-xl">
                <StatusIcon className="w-3.5 h-3.5 mr-1" />
                {server.status.toUpperCase()}
              </Badge>
            </CardAction>
          </CardHeader>
        </RouterLink>
        <CardContent className="flex-grow space-y-3 pt-02 pb-2 px-4">
          <VpsTags tags={server.tags} className="mt-0" />
          <div className="grid grid-cols-2 gap-x-4 gap-y-3">
            <UsageProgress
              Icon={Cpu}
              label={t('vps.cpu')}
              value={cpuUsage}
              usageText={`${cpuUsage?.toFixed(1)}%`}
              iconClassName="text-indigo-500"
            />

            <Tooltip>
              <TooltipTrigger asChild>
                <div>
                  <UsageProgress
                    Icon={MemoryStick}
                    label={t('vps.ram')}
                    value={memoryUsagePercent}
                    usageText={`${memoryUsagePercent?.toFixed(1)}%`}
                    iconClassName="text-purple-500"
                  />
                </div>
              </TooltipTrigger>
              <TooltipContent>
                {formatBytesForDisplay(memoryUsageBytes)} / {formatBytesForDisplay(memoryTotalBytes)}
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <div>
                  <UsageProgress
                    Icon={HardDrive}
                    label={t('vps.disk')}
                    value={diskUsagePercent}
                    usageText={`${diskUsagePercent?.toFixed(1)}%`}
                    iconClassName="text-orange-500"
                  />
                </div>
              </TooltipTrigger>
              <TooltipContent>
                {formatBytesForDisplay(diskUsedBytes)} / {formatBytesForDisplay(diskTotalBytes)}
              </TooltipContent>
            </Tooltip>

            {server.trafficLimitBytes && server.trafficLimitBytes > 0 && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <div>
                    <UsageProgress
                      Icon={Signal}
                      label={t('vps.traffic')}
                      value={trafficUsagePercent}
                      usageText={`${trafficUsagePercent?.toFixed(1)}%`}
                      iconClassName="text-cyan-500"
                    />
                  </div>
                </TooltipTrigger>
                <TooltipContent>
                  {formatBytesForDisplay(usedTrafficBytes)} / {formatBytesForDisplay(server.trafficLimitBytes)}
                </TooltipContent>
              </Tooltip>
            )}
          </div>

          {renewalInfo.isApplicable && (
            <UsageProgress
              Icon={CalendarClock}
              label={t('vps.renewal')}
              value={renewalInfo.progressPercent}
              usageText={renewalInfo.statusText}
              iconClassName={`text-${renewalInfo.variant}`}
            />
          )}
          <div className="pt-2">
            <VpsMetricsChart vpsId={server.id} activeTab={activeChartTab} onTabChange={onChartTabChange} />
          </div>
        </CardContent>
        <CardFooter className="flex justify-between text-xs text-muted-foreground pt-2 pb-2 px-4 border-t">
          <div className="flex items-center">
            <ArrowUp className="w-3.5 h-3.5 mr-1 text-emerald-500" /> {formatNetworkSpeed(metrics?.networkTxInstantBps)}
          </div>
          <div className="flex items-center">
            <ArrowDown className="w-3.5 h-3.5 mr-1 text-sky-500" /> {formatNetworkSpeed(metrics?.networkRxInstantBps)}
          </div>
        </CardFooter>
      </Card>
    </TooltipProvider>
  );
});
VpsCard.displayName = 'VpsCard';

export default VpsCard;