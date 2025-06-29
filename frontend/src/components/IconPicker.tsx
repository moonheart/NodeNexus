import React, { useState } from 'react';
import { icons, ExternalLink, Check, ChevronsUpDown } from 'lucide-react';
import type { LucideProps } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { cn } from '@/lib/utils';

// A curated list of common icon names for quick selection.
const iconNames = [
  'Server', 'Database', 'Cloud', 'Terminal', 'Code', 'Globe', 'HardDrive',
  'Shield', 'Key', 'Wallet', 'Box', 'Archive', 'Cpu', 'Router', 'GitBranch',
  'File', 'Folder', 'Network', 'Activity', 'ChartBar', 'Layers', 'Package',
  'Unplug', 'PlugZap', 'Power', 'Settings', 'Wrench', 'Bug', 'Rocket', 'Star'
] as const;

// A helper component to safely render a Lucide icon by its string name.
const LucideIcon = ({ name, ...props }: { name: string } & LucideProps) => {
  const IconComponent = icons[name as keyof typeof icons];
  if (!IconComponent) {
    return null; // Return null if the icon name is invalid
  }
  return <IconComponent {...props} />;
};

interface IconPickerProps {
  value: string;
  onChange: (name: string) => void;
}

const IconPicker: React.FC<IconPickerProps> = ({ value, onChange }) => {
  const [open, setOpen] = useState(false);

  return (
    <div>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            role="combobox"
            aria-expanded={open}
            className="w-full justify-between"
          >
            <div className="flex items-center gap-2">
              {value ? <LucideIcon name={value} className="h-4 w-4" /> : null}
              {value ? value : "Select an icon..."}
            </div>
            <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-[300px] p-0">
          <Command>
            <CommandInput placeholder="Search icon..." />
            <CommandList>
              <CommandEmpty>No icon found.</CommandEmpty>
              <CommandGroup>
                {iconNames.map((name) => (
                  <CommandItem
                    key={name}
                    value={name}
                    onSelect={(currentValue) => {
                      onChange(currentValue === value ? "" : currentValue);
                      setOpen(false);
                    }}
                  >
                    <Check
                      className={cn(
                        "mr-2 h-4 w-4",
                        value === name ? "opacity-100" : "opacity-0"
                      )}
                    />
                    <LucideIcon name={name} className="mr-2 h-4 w-4" />
                    {name}
                  </CommandItem>
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
      <div className="text-right mt-2">
        <a
          href="https://lucide.dev/icons/"
          target="_blank"
          rel="noopener noreferrer"
          className="text-xs text-muted-foreground hover:text-primary flex items-center justify-end gap-1"
        >
          Browse all icons <ExternalLink className="w-3 h-3" />
        </a>
      </div>
    </div>
  );
};

export default IconPicker;