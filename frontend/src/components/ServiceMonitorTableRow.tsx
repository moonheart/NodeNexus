import React from 'react';
import { TableCell, TableRow } from '@/components/ui/table';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Pencil, Trash2 } from 'lucide-react';
import type { ServiceMonitor } from '../types';
import { Link } from 'react-router-dom';

interface ServiceMonitorTableRowProps {
  monitor: ServiceMonitor;
  onEdit: (monitor: ServiceMonitor) => void;
  onDelete: (id: number) => void;
}

const ServiceMonitorTableRow: React.FC<ServiceMonitorTableRowProps> = ({ monitor, onEdit, onDelete }) => {
  const getStatusVariant = (status: string) => {
    switch (status) {
      case 'UP':
        return 'bg-green-500';
      case 'DOWN':
        return 'bg-red-500';
      default:
        return 'bg-gray-400';
    }
  };

  return (
    <TableRow key={monitor.id}>
      <TableCell>
        <Link to={`/monitors/${monitor.id}`} className="flex items-center space-x-2">
          <span className={`h-3 w-3 rounded-full ${getStatusVariant(monitor.lastStatus || 'UNKNOWN')}`} />
          <span>{monitor.name}</span>
        </Link>
      </TableCell>
      <TableCell>
        <Badge variant="secondary">{monitor.monitorType.toUpperCase()}</Badge>
      </TableCell>
      <TableCell className="truncate max-w-xs" title={monitor.target}>
        {monitor.target}
      </TableCell>
      <TableCell>{monitor.frequencySeconds}s</TableCell>
      <TableCell>
        {monitor.lastCheck ? new Date(monitor.lastCheck).toLocaleString() : 'N/A'}
      </TableCell>
      <TableCell className="truncate max-w-xs" title={monitor.statusMessage || ''}>
        {monitor.statusMessage || 'N/A'}
      </TableCell>
      <TableCell className="text-right">
        <Button variant="outline" size="sm" onClick={() => onEdit(monitor)} className="mr-2">
          <Pencil className="w-4 h-4 mr-1" /> 编辑
        </Button>
        <Button variant="destructive" size="sm" onClick={() => onDelete(monitor.id)}>
          <Trash2 className="w-4 h-4 mr-1" /> 删除
        </Button>
      </TableCell>
    </TableRow>
  );
};

export default ServiceMonitorTableRow;