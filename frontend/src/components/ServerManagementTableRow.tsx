import React from 'react';
import { useTranslation } from 'react-i18next';
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
  const { t } = useTranslation();
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
          aria-label={t('serverManagement.table.selectAll')}
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
          {server.ipAddress || t('vps.na')}
        </div>
      </TableCell>
      <TableCell className="truncate" title={server.osType ?? t('vps.na')}>
        {server.osType ?? t('vps.na')}
      </TableCell>
      <TableCell>{server.agentVersion || t('vps.na')}</TableCell>
      <TableCell>{server.group || t('vps.na')}</TableCell>
      <TableCell>
        {renewalInfo.isApplicable ? (
          <Badge variant={renewalInfo.variant}>{renewalInfo.statusText}</Badge>
        ) : (
          <span className="text-xs text-muted-foreground">{t('vps.notConfigured')}</span>
        )}
      </TableCell>
      <TableCell className="text-right">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="h-8 w-8 p-0">
              <span className="sr-only">{t('common.actions.openMenu')}</span>
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => onEdit(server)}>
              <Pencil className="mr-2 h-4 w-4" />
              {t('common.actions.edit')}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onTriggerUpdate(server.id)}
              disabled={server.status !== 'online'}
            >
              <RefreshCw className="mr-2 h-4 w-4" />
              {t('serverManagement.actions.updateAgent')}
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => onCopyCommand(server)}>
              <Copy className="mr-2 h-4 w-4" />
              {t('serverManagement.actions.copyCommand')}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onDelete(server.id)}
              className="text-destructive focus:text-destructive"
            >
              <Trash2 className="mr-2 h-4 w-4" />
              {t('common.actions.delete')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </TableCell>
    </TableRow>
  );
};

export default ServerManagementTableRow;