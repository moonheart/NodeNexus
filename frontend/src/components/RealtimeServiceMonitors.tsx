import React, { useState, useEffect } from 'react';
import { getMonitorsByVpsId } from '../services/serviceMonitorService';
import type { ServiceMonitor } from '../types';
import RealtimeMonitorChart from './RealtimeMonitorChart';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useTranslation } from 'react-i18next';

interface RealtimeServiceMonitorsProps {
  vpsId: number;
}

const RealtimeServiceMonitors: React.FC<RealtimeServiceMonitorsProps> = ({ vpsId }) => {
  const { t } = useTranslation();
  const [monitors, setMonitors] = useState<ServiceMonitor[]>([]);
  const [selectedMonitorId, setSelectedMonitorId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchMonitors = async () => {
      setIsLoading(true);
      setError(null);
      try {
        const data = await getMonitorsByVpsId(vpsId);
        setMonitors(Array.isArray(data) ? data : []);
        if (Array.isArray(data) && data.length > 0) {
          setSelectedMonitorId(String(data[0].id));
        }
      } catch (err) {
        console.error("Failed to fetch monitors for VPS:", err);
        setError(t('vpsDetailPage.serviceMonitoring.errors.loadMonitors'));
      } finally {
        setIsLoading(false);
      }
    };

    fetchMonitors();
  }, [vpsId, t]);

  if (isLoading) {
    return <div className="text-center p-8">{t('vpsDetailPage.serviceMonitoring.loadingMonitors')}</div>;
  }

  if (error) {
    return <div className="text-center p-8 text-destructive">{error}</div>;
  }

  if (monitors.length === 0) {
    return <div className="text-center p-8">{t('vpsDetailPage.serviceMonitoring.noData')}</div>;
  }

  return (
    <div className="space-y-4">
      <div className="w-full md:w-1/2 lg:w-1/3 mx-auto">
        <Select value={selectedMonitorId ?? ''} onValueChange={setSelectedMonitorId}>
          <SelectTrigger>
            <SelectValue placeholder={t('vpsDetailPage.serviceMonitoring.selectMonitorPlaceholder')} />
          </SelectTrigger>
          <SelectContent>
            {monitors.map(monitor => (
              <SelectItem key={monitor.id} value={String(monitor.id)}>
                {monitor.name} ({monitor.monitorType})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="h-80">
        {selectedMonitorId ? (
          <RealtimeMonitorChart monitorId={parseInt(selectedMonitorId, 10)} />
        ) : (
          <div className="flex items-center justify-center h-full">
            <p className="text-muted-foreground">{t('vpsDetailPage.serviceMonitoring.selectMonitorPrompt')}</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default RealtimeServiceMonitors;