import React from 'react';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { getProgressVariantClass } from '@/utils/vpsUtils';

interface UsageProgressProps {
  value: number | null;
  label: string;
  usageText: string;
  Icon: React.ElementType;
  iconClassName?: string;
}

export const UsageProgress: React.FC<UsageProgressProps> = ({ value, label, usageText, Icon, iconClassName }) => {
  if (value === null) return null;
  
  const variantClass = getProgressVariantClass(value);

  return (
    <div className="text-xs text-slate-600">
      <div className="flex items-center mb-0.5">
        <Icon className={cn("w-4 h-4 mr-1.5 flex-shrink-0", iconClassName)} />
        <span>{label}: <span className="font-semibold">{usageText}</span></span>
      </div>
      <Progress value={value} className={cn("h-2", variantClass)} />
    </div>
  );
};