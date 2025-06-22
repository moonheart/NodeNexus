import React, { useState, useEffect } from 'react';
import { createVps } from '../services/vpsService';
import type { Vps } from '../types';
import axios from 'axios';
import CommandCopyUI from './CommandCopyUI';

interface CreateVpsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onVpsCreated?: () => void;
}

const CreateVpsModal: React.FC<CreateVpsModalProps> = ({ isOpen, onClose, onVpsCreated }) => {
  const [vpsName, setVpsName] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createdVps, setCreatedVps] = useState<Vps | null>(null);

  useEffect(() => {
    if (!isOpen) {
      setVpsName('');
      setError(null);
      setCreatedVps(null);
      setIsLoading(false);
    }
  }, [isOpen]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsLoading(true);
    setError(null);

    if (!vpsName.trim()) {
      setError('VPS名称不能为空。');
      setIsLoading(false);
      return;
    }

    try {
      const payload: import('../services/vpsService').CreateVpsPayload = {
        name: vpsName.trim(),
      };
      const newVps = await createVps(payload);
      setCreatedVps(newVps);
      setVpsName(''); // Clear input

      if (onVpsCreated) {
        onVpsCreated();
      }
    } catch (err: unknown) {
      console.error('Failed to create VPS:', err);
      let errorMessage = '创建VPS失败，请稍后再试。';
      if (axios.isAxiosError(err) && err.response?.data?.error) {
        errorMessage = err.response.data.error;
      } else if (err instanceof Error) {
        errorMessage = err.message;
      }
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  };

  const renderSuccessView = () => {
    if (!createdVps) return null;

    return (
      <div>
        <h3 style={{ marginTop: '0', color: 'green' }}>VPS "{createdVps.name}" 创建成功!</h3>
        <p>请为您的服务器选择对应的操作系统，并复制安装命令来安装 Agent：</p>
        <CommandCopyUI vps={createdVps} />
        <button onClick={onClose} style={{ marginTop: '20px', padding: '8px 12px', cursor: 'pointer', backgroundColor: '#6c757d', color: 'white', border: 'none', borderRadius: '4px' }}>
          关闭
        </button>
      </div>
    );
  };

  if (!isOpen) {
    return null;
  }

  return (
    <div style={{
      position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
      backgroundColor: 'rgba(0, 0, 0, 0.5)',
      display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000
    }}>
      <div style={{
        background: 'white', padding: '25px', borderRadius: '8px',
        boxShadow: '0 4px 6px rgba(0, 0, 0, 0.1)', width: '90%', maxWidth: '500px'
      }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
          <h2>创建新的VPS</h2>
          <button onClick={onClose} style={{ background: 'none', border: 'none', fontSize: '1.5rem', cursor: 'pointer' }}>&times;</button>
        </div>

        {!createdVps ? (
          <form onSubmit={handleSubmit}>
            <div style={{ marginBottom: '15px' }}>
              <label htmlFor="vpsNameModal" style={{ display: 'block', marginBottom: '5px' }}>VPS 名称:</label>
              <input
                type="text"
                id="vpsNameModal"
                value={vpsName}
                onChange={(e) => setVpsName(e.target.value)}
                placeholder="例如：我的Web服务器"
                required
                style={{ width: '100%', padding: '10px', boxSizing: 'border-box', borderRadius: '4px', border: '1px solid #ccc' }}
              />
            </div>

            {error && <p style={{ color: 'red', marginTop: '0', marginBottom: '10px' }}>错误: {error}</p>}
            <button type="submit" disabled={isLoading} style={{ padding: '10px 15px', cursor: 'pointer', width: '100%', backgroundColor: '#007bff', color: 'white', border: 'none', borderRadius: '4px' }}>
              {isLoading ? '创建中...' : '创建VPS'}
            </button>
          </form>
        ) : (
          renderSuccessView()
        )}
      </div>
    </div>
  );
};

export default CreateVpsModal;