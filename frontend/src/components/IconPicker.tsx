import React from 'react';
import { ExternalLink } from 'lucide-react';
import { DynamicIcon, type IconName } from 'lucide-react/dynamic';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface IconPickerProps {
  value: string;
  onChange: (name: string) => void;
}

const IconPicker: React.FC<IconPickerProps> = ({ value, onChange }) => {
  const iconName = value as IconName;

  return (
    <div className="flex items-center gap-2">
      <div className="flex-shrink-0 h-10 w-10 border rounded-md flex items-center justify-center">
        {value ? <DynamicIcon name={iconName} className="h-6 w-6" /> : null}
      </div>
      <Input
        type="text"
        placeholder="Enter icon name"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="flex-grow"
      />
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="outline" size="icon" asChild>
              <a href="https://lucide.dev/icons/" target="_blank" rel="noopener noreferrer">
                <ExternalLink className="h-4 w-4" />
              </a>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>Browse all icons on lucide.dev</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    </div>
  );
};

export default IconPicker;