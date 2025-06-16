import React from 'react';
import { Link as RouterLink } from 'react-router-dom';
import type { VpsListItemResponse } from '../types';
import {
  PencilIcon,
  RefreshCwIcon,
} from './Icons';
import {
  calculateRenewalInfo,
  getVpsStatusAppearance,
} from '../utils/vpsUtils';

interface ServerManagementTableRowProps {
  server: VpsListItemResponse;
  onEdit: (server: VpsListItemResponse) => void;
  onTriggerUpdate: (vpsId: number) => void;
  onSelectionChange: (vpsId: number, isSelected: boolean) => void;
  isSelected: boolean;
}

const ServerManagementTableRow: React.FC<ServerManagementTableRowProps> = ({ server, onEdit, onTriggerUpdate, onSelectionChange, isSelected }) => {
  const { tableRowBadgeClass, tableRowTextClass, icon } = getVpsStatusAppearance(server.status);


  const renewalInfo = calculateRenewalInfo(
    server.nextRenewalDate,
    server.lastRenewalDate,
    server.serviceStartDate,
    server.renewalCycle,
    server.renewalCycleCustomDays
  );
  
  return (
    <tr className="bg-white hover:bg-slate-50 transition-colors duration-150 border-b border-slate-200 last:border-b-0">
      <td className="px-4 py-3">
        <input
          type="checkbox"
          className="h-4 w-4 text-indigo-600 border-slate-300 rounded focus:ring-indigo-500"
          checked={isSelected}
          onChange={(e) => onSelectionChange(server.id, e.target.checked)}
        />
      </td>
      <td className="px-4 py-3 text-sm font-medium text-slate-800">
        <div className="truncate" title={server.name}>
          <RouterLink to={`/vps/${server.id}`} className="text-indigo-600 hover:text-indigo-700 hover:underline">
            {server.name}
          </RouterLink>
        </div>
      </td>
      <td className="px-4 py-3 text-sm">
        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold ${tableRowBadgeClass} ${tableRowTextClass}`}>
          {icon && <span className="mr-1.5">{icon}</span>}
          {server.status.toUpperCase()}
        </span>
      </td>
      <td className="px-4 py-3 text-sm text-slate-600">
        <div className="flex items-center">
          {server.metadata?.country_code && (
            <span className={`fi fi-${server.metadata.country_code.toLowerCase()} mr-2`}></span>
          )}
          {server.ipAddress || 'N/A'}
        </div>
      </td>
      <td className="px-4 py-3 text-sm text-slate-600 truncate" title={server.osType ?? 'N/A'}>
        {server.osType ?? 'N/A'}
      </td>
      <td className="px-4 py-3 text-sm text-slate-600">{server.agentVersion || 'N/A'}</td>
      <td className="px-4 py-3 text-sm text-slate-600">{server.group || 'N/A'}</td>
      <td className="px-4 py-3 text-sm text-slate-600">
        {renewalInfo.isApplicable && renewalInfo.progressPercent !== null ? (
          <div className="w-28"> {/* Fixed width for consistency */}
            <div className="flex items-center justify-between text-xs mb-0.5">
              <span className={`font-semibold ${renewalInfo.colorClass.replace('bg-', 'text-')}`}>{renewalInfo.statusText}</span>
            </div>
          </div>
        ) : (
          <span className="text-xs text-slate-400">未配置</span>
        )}
      </td>
      <td className="px-4 py-3 text-left">
       <div className="flex items-center justify-start space-x-2">
         <button
           onClick={() => onEdit(server)}
           className="text-slate-600 hover:text-slate-800 font-medium text-xs py-1 px-3 rounded-md hover:bg-slate-100 transition-colors flex items-center"
           aria-label={`Edit ${server.name}`}
         >
           <PencilIcon className="w-3.5 h-3.5 mr-1" />
           编辑
         </button>
         <button
           onClick={() => onTriggerUpdate(server.id)}
           className="text-slate-600 hover:text-slate-800 font-medium text-xs py-1 px-3 rounded-md hover:bg-slate-100 transition-colors flex items-center"
           aria-label={`Update agent on ${server.name}`}
           disabled={server.status !== 'online'}
           title={server.status !== 'online' ? 'Agent is not online' : 'Trigger agent update'}
         >
           <RefreshCwIcon className={`w-3.5 h-3.5 mr-1 ${server.status === 'online' ? '' : 'text-slate-400'}`} />
           更新
         </button>
       </div>
      </td>
    </tr>
  );
};

export default ServerManagementTableRow;