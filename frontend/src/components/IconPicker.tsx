import React, { useState, useRef, useEffect } from 'react';
import { icons, ExternalLink } from 'lucide-react';
import type { LucideProps } from 'lucide-react';

// A curated list of common icon names for quick selection.
const iconNames = [
  'Server', 'Database', 'Cloud', 'Terminal', 'Code', 'Globe', 'HardDrive',
  'Shield', 'Key', 'Wallet', 'Box', 'Archive', 'Cpu', 'Router', 'GitBranch',
  'File', 'Folder', 'Network', 'Activity', 'ChartBar', 'Layers', 'Package',
  'Unplug', 'PlugZap', 'Power', 'Settings', 'Wrench', 'Bug', 'Rocket', 'Star'
] as const;

interface IconPickerProps {
  value: string;
  onChange: (name: string) => void;
}

// A helper component to safely render a Lucide icon by its string name.
const LucideIcon = ({ name, ...props }: { name: string } & LucideProps) => {
  const IconComponent = icons[name as keyof typeof icons];
  if (!IconComponent) {
    return null; // Return null if the icon name is invalid
  }
  return <IconComponent {...props} />;
};

const IconPicker: React.FC<IconPickerProps> = ({ value, onChange }) => {
  const [isOpen, setIsOpen] = useState(false);
  const pickerRef = useRef<HTMLDivElement>(null);

  // Effect to close the dropdown when clicking outside of it.
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (pickerRef.current && !pickerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [pickerRef]);

  return (
    <div ref={pickerRef}>
      <div className="relative">
        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
          <LucideIcon name={value} className="w-5 h-5 text-gray-400" />
        </div>
        <input
          type="text"
          className="w-full pl-10 pr-4 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onFocus={() => setIsOpen(true)}
          placeholder="Type or select an icon"
          autoComplete="off"
        />
        {isOpen && (
          <div className="absolute z-10 mt-1 w-full bg-white shadow-lg rounded-md border border-gray-200 max-h-60 overflow-y-auto">
            <div className="grid grid-cols-6 gap-1 p-2">
              {iconNames.map((name) => (
                <button
                  key={name}
                  type="button"
                  className={`p-2 rounded-md flex items-center justify-center hover:bg-slate-100 ${value === name ? 'bg-indigo-100 ring-2 ring-indigo-500' : ''}`}
                  onClick={() => {
                    onChange(name);
                    setIsOpen(false);
                  }}
                  title={name}
                >
                  <LucideIcon name={name} className="w-5 h-5" />
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
      <div className="text-right mt-1">
        <a
          href="https://lucide.dev/icons/"
          target="_blank"
          rel="noopener noreferrer"
          className="text-xs text-indigo-600 hover:underline flex items-center justify-end gap-1"
        >
          Browse all icons <ExternalLink className="w-3 h-3" />
        </a>
      </div>
    </div>
  );
};

export default IconPicker;