import React from 'react';
import type { Vps, VpsListItemResponse } from '../types';
import CommandCopyUI from './CommandCopyUI';

interface CopyCommandModalProps {
  isOpen: boolean;
  onClose: () => void;
  vps: Vps | VpsListItemResponse | null;
}

const CopyCommandModal: React.FC<CopyCommandModalProps> = ({ isOpen, onClose, vps }) => {
  if (!isOpen || !vps) {
    return null;
  }

  return (
    <div style={{
      position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
      backgroundColor: 'rgba(0, 0, 0, 0.5)',
      display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1050
    }}>
      <div style={{
        background: 'white', padding: '25px', borderRadius: '8px',
        boxShadow: '0 4px 6px rgba(0, 0, 0, 0.1)', width: '90%', maxWidth: '550px'
      }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
          <h3 style={{ marginTop: '0' }}>为 "{vps.name}" 安装 Agent</h3>
          <button onClick={onClose} style={{ background: 'none', border: 'none', fontSize: '1.5rem', cursor: 'pointer' }}>&times;</button>
        </div>
        
        <p>请为您的服务器选择对应的操作系统，并复制安装命令：</p>
        
        <CommandCopyUI vps={vps} />

        <div style={{ marginTop: '20px', display: 'flex', justifyContent: 'flex-end' }}>
          <button onClick={onClose} style={{ padding: '8px 12px', cursor: 'pointer', backgroundColor: '#6c757d', color: 'white', border: 'none', borderRadius: '4px' }}>
            关闭
          </button>
        </div>
      </div>
    </div>
  );
};

export default CopyCommandModal;