import React from 'react';
import { useTranslation } from 'react-i18next';
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

// Expanded time range options for more universal use
export type TimeRangeValue = '10m' | '1h' | '6h' | '12h' | '1d' | '3d' | '7d';

export interface TimeRangeOption {
  labelKey: string;
  getDates: (now: Date) => { startTime: Date; endTime: Date };
  interval: string | null;
}

// Centralized configuration for time ranges
export const TIME_RANGE_CONFIG: Record<TimeRangeValue, Omit<TimeRangeOption, 'labelKey'>> = {
  '10m': {
    getDates: (now: Date) => ({ startTime: new Date(now.getTime() - 10 * 60 * 1000), endTime: now }),
    interval: null, // Raw data
  },
  '1h': {
    getDates: (now: Date) => ({ startTime: new Date(now.getTime() - 60 * 60 * 1000), endTime: now }),
    interval: '1m',
  },
  '6h': {
    getDates: (now: Date) => ({ startTime: new Date(now.getTime() - 6 * 60 * 60 * 1000), endTime: now }),
    interval: '5m',
  },
  '12h': {
    getDates: (now: Date) => ({ startTime: new Date(now.getTime() - 12 * 60 * 60 * 1000), endTime: now }),
    interval: '10m',
  },
  '1d': {
    getDates: (now: Date) => ({ startTime: new Date(now.getTime() - 24 * 60 * 60 * 1000), endTime: now }),
    interval: '15m',
  },
  '3d': {
    getDates: (now: Date) => ({ startTime: new Date(now.getTime() - 3 * 24 * 60 * 60 * 1000), endTime: now }),
    interval: '30m',
  },
  '7d': {
    getDates: (now: Date) => ({ startTime: new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000), endTime: now }),
    interval: '1h',
  },
};

// Labels for the toggle group
const TIME_RANGE_LABELS: Record<TimeRangeValue, string> = {
    '10m': 'timeRanges.m10',
    '1h': 'timeRanges.h1',
    '6h': 'timeRanges.h6',
    '12h': 'timeRanges.h12',
    '1d': 'timeRanges.d1',
    '3d': 'timeRanges.d3',
    '7d': 'timeRanges.d7',
};


/**
 * A utility function to get the detailed time range information.
 * @param value The selected TimeRangeValue.
 * @returns An object with startTime, endTime (as ISO strings), and the interval.
 */
export const getTimeRangeDetails = (value: TimeRangeValue, now = new Date()) => {
  const config = TIME_RANGE_CONFIG[value];
  const { startTime, endTime } = config.getDates(now);
  return {
    startTime: startTime.toISOString(),
    endTime: endTime.toISOString(),
    interval: config.interval,
  };
};

interface TimeRangeSelectorProps {
  value: TimeRangeValue;
  onValueChange: (value: TimeRangeValue) => void;
  // Allow parent to specify which options to show
  options?: TimeRangeValue[];
}

export const TimeRangeSelector: React.FC<TimeRangeSelectorProps> = ({
  value,
  onValueChange,
  options = ['1h', '6h', '1d', '7d'], // Default options
}) => {
  const { t } = useTranslation();

  return (
    <ToggleGroup
      type="single"
      value={value}
      onValueChange={(newValue) => {
        if (newValue) {
          onValueChange(newValue as TimeRangeValue);
        }
      }}
      aria-label={t('timeRanges.title')}
      className="justify-end"
    >
      {options.map((optionValue) => (
        <ToggleGroupItem key={optionValue} value={optionValue} aria-label={t(TIME_RANGE_LABELS[optionValue])}>
          {t(TIME_RANGE_LABELS[optionValue])}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
};