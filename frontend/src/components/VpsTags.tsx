import React from 'react';
import type { Tag } from '../types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { DynamicIcon, type IconName } from 'lucide-react/dynamic';

interface VpsTagsProps {
  tags: Tag[] | undefined;
  className?: string;
}

const getContrastingTextColor = (hexColor: string): string => {
    if (!hexColor) return '#000000';
    const hex = hexColor.replace('#', '');
    if (hex.length !== 6) return '#000000';
    const r = parseInt(hex.substring(0, 2), 16);
    const g = parseInt(hex.substring(2, 4), 16);
    const b = parseInt(hex.substring(4, 6), 16);
    const yiq = ((r * 299) + (g * 587) + (b * 114)) / 1000;
    return (yiq >= 128) ? '#000000' : '#ffffff';
};

export const VpsTags: React.FC<VpsTagsProps> = React.memo(({ tags, className }) => {
  if (!tags || tags.length === 0) {
    return null;
  }

  return (
    <div className={cn("mt-2 flex flex-wrap gap-1", className)}>
      {tags.filter(tag => tag.isVisible).map(tag => {
        const iconName = tag.icon as IconName;
        const tagContent = (
          <Badge
            key={tag.id}
            className="text-xs font-medium flex items-center gap-1"
            style={{
              backgroundColor: tag.color,
              color: getContrastingTextColor(tag.color),
            }}
          >
            {tag.icon && <DynamicIcon name={iconName} className="h-3 w-3" />}
            <span>{tag.name}</span>
          </Badge>
        );

        if (tag.url) {
          return (
            <a href={tag.url} target="_blank" rel="noopener noreferrer" key={tag.id}>
              {tagContent}
            </a>
          );
        }
        return tagContent;
      })}
    </div>
  );
});

VpsTags.displayName = 'VpsTags';