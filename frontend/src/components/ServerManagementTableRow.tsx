import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import type { VpsListItemResponse } from '../types';
import {
  calculateRenewalInfo,
  getVpsStatusAppearance,
} from '../utils/vpsUtils';
import { TableCell, TableRow } from '@/components/ui/table';
import { Checkbox } from '@/components/ui/checkbox';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { MoreHorizontal, Pencil, RefreshCw, Copy, Trash2 } from 'lucide-react';

interface ServerManagementTableRowProps {
  server: VpsListItemResponse;
  onEdit: (server: VpsListItemResponse) => void;
  onCopyCommand: (server: VpsListItemResponse) => void;
  onTriggerUpdate: (vpsId: number) => void;
  onDelete: (vpsId: number) => void;
  onSelectionChange: (vpsId: number, isSelected: boolean) => void;
  isSelected: boolean;
}

const ServerManagementTableRow: React.FC<ServerManagementTableRowProps> = ({
  server,
  onEdit,
  onCopyCommand,
  onTriggerUpdate,
  onDelete,
  onSelectionChange,
  isSelected,
}) => {
  const statusAppearance = getVpsStatusAppearance(server.status);
  const IconComponent = statusAppearance.icon;

  const renewalInfo = calculateRenewalInfo(
    server.nextRenewalDate,
    server.lastRenewalDate,
    server.serviceStartDate,
    server.renewalCycle,
    server.renewalCycleCustomDays
  );

  return (
    <TableRow key={server.id}>
      <TableCell className="w-8">
        <Checkbox
          checked={isSelected}
          onCheckedChange={(checked) => onSelectionChange(server.id, !!checked)}
          aria-label="Select row"
        />
      </TableCell>
      <TableCell className="font-medium">
        <RouterLink to={`/vps/${server.id}`} className="text-primary hover:underline">
          {server.name}
        </RouterLink>
      </TableCell>
      <TableCell>
        <Badge variant={statusAppearance.variant}>
          <IconComponent className="w-3.5 h-3.5 mr-1.5" />
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
      <TableCell className="truncate" title={server.osType ?? 'N/A'}>
        {server.osType ?? 'N/A'}
      </TableCell>
      <TableCell>{server.agentVersion || 'N/A'}</TableCell>
      <TableCell>{server.group || 'N/A'}</TableCell>
      <TableCell>
        {renewalInfo.isApplicable ? (
          <Badge variant={renewalInfo.variant}>{renewalInfo.statusText}</Badge>
        ) : (
          <span className="text-xs text-muted-foreground">未配置</span>
        )}
      </TableCell>
      <TableCell className="text-right">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="h-8 w-8 p-0">
              <span className="sr-only">Open menu</span>
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => onEdit(server)}>
              <Pencil className="mr-2 h-4 w-4" />
              Edit
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onTriggerUpdate(server.id)}
              disabled={server.status !== 'online'}
            >
              <RefreshCw className="mr-2 h-4 w-4" />
              Update Agent
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => onCopyCommand(server)}>
              <Copy className="mr-2 h-4 w-4" />
              Copy Command
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onDelete(server.id)}
              className="text-destructive focus:text-destructive"
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </TableCell>
    </TableRow>
  );
};

export default ServerManagementTableRow;