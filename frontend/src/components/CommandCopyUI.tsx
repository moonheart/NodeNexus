import React, { useState, useEffect } from 'react';
import type { Vps, VpsListItemResponse } from '../types';
import { generateInstallCommand, detectOsType } from '../utils/commandUtils';
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Copy } from 'lucide-react';
import { toast } from 'react-hot-toast';

type OsType = 'linux' | 'macos' | 'windows';

interface CommandCopyUIProps {
  vps: Vps | VpsListItemResponse;
}

const CommandCopyUI: React.FC<CommandCopyUIProps> = ({ vps }) => {
  const [activeTab, setActiveTab] = useState<OsType>('linux');

  useEffect(() => {
    const detectedOs = detectOsType('osType' in vps ? vps.osType : null);
    setActiveTab(detectedOs);
  }, [vps]);

  const handleCopyToClipboard = (command: string) => {
    navigator.clipboard.writeText(command).then(() => {
      toast.success('Command copied to clipboard!');
    }, (err) => {
      console.error('Failed to copy command:', err);
      toast.error('Failed to copy command.');
    });
  };

  const renderTabContent = (os: OsType) => {
    const command = generateInstallCommand(vps, os);
    return (
      <TabsContent value={os}>
        <div className="relative">
          <Textarea
            readOnly
            value={command}
            className="font-mono text-xs h-32 pr-12"
            rows={5}
          />
          <Button
            variant="ghost"
            size="icon"
            className="absolute top-2 right-2 h-7 w-7"
            onClick={() => handleCopyToClipboard(command)}
          >
            <Copy className="h-4 w-4" />
          </Button>
        </div>
      </TabsContent>
    );
  };

  return (
    <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as OsType)} className="w-full">
      <TabsList className="grid w-full grid-cols-3">
        <TabsTrigger value="linux">Linux</TabsTrigger>
        <TabsTrigger value="macos">macOS</TabsTrigger>
        <TabsTrigger value="windows">Windows</TabsTrigger>
      </TabsList>
      {renderTabContent('linux')}
      {renderTabContent('macos')}
      {renderTabContent('windows')}
    </Tabs>
  );
};

export default CommandCopyUI;