import React, { useState, useEffect } from 'react';
import { createVps } from '../services/vpsService';
import type { Vps } from '../types';
import axios from 'axios';
interface CreateVpsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onVpsCreated?: (newVps: Vps) => void; // Optional callback after successful creation
}

const CreateVpsModal: React.FC<CreateVpsModalProps> = ({ isOpen, onClose, onVpsCreated }) => {
  const [vpsName, setVpsName] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [agentConfigToCopy, setAgentConfigToCopy] = useState<string | null>(null);

  // Reset form when modal is opened or closed
  useEffect(() => {
    if (!isOpen) {
      setVpsName('');
      setError(null);
      setSuccessMessage(null);
      setAgentConfigToCopy(null);
      setIsLoading(false);
    }
  }, [isOpen]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsLoading(true);
    setError(null);
    setSuccessMessage(null);
    setAgentConfigToCopy(null);

    if (!vpsName.trim()) {
      setError('VPS名称不能为空。');
      setIsLoading(false);
      return;
    }

    try {
      const payload: import('../services/vpsService').CreateVpsPayload = {
        name: vpsName.trim(),
      };

      const newVps: Vps = await createVps(payload);
      setSuccessMessage(`VPS "${newVps.name}" 创建成功！ID: ${newVps.id}`);
      
      const configContent = `# Agent Configuration for VPS: ${newVps.name} (ID: ${newVps.id})
# 请将此内容保存到 Agent 的 config.toml 文件中
# 并确保将 YOUR_SERVER_IP_OR_DOMAIN:50051 替换为实际的服务器gRPC地址和端口

server_address = "http://YOUR_SERVER_IP_OR_DOMAIN:50051"
vps_id = ${newVps.id}
agent_secret = "${newVps.agent_secret}"
`;
      setAgentConfigToCopy(configContent);
      setVpsName(''); // Clear input

      if (onVpsCreated) {
        onVpsCreated(newVps);
      }
      // Optionally close modal on success after a delay, or let user close it.
      // setTimeout(onClose, 3000); // Example: close after 3 seconds
    } catch (err: unknown) {
      console.error('Failed to create VPS:', err);
      let errorMessage = '创建VPS失败，请稍后再试。';
      if (axios.isAxiosError(err)) {
        if (err.response?.data?.error) {
          errorMessage = err.response.data.error;
        } else if (err.message) {
          errorMessage = err.message;
        }
      } else if (err instanceof Error) {
        errorMessage = err.message;
      }
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  };

  const handleCopyToClipboard = async () => {
    if (agentConfigToCopy) {
      try {
        await navigator.clipboard.writeText(agentConfigToCopy);
        setSuccessMessage( (prev) => prev ? prev + ' Agent配置已复制到剪贴板！' : 'Agent配置已复制到剪贴板！');
      } catch (err) {
        console.error('Failed to copy agent config:', err);
        setError('无法复制Agent配置，请手动复制。');
      }
    }
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

        {!agentConfigToCopy ? (
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
          <div>
            <h3 style={{ marginTop: '0', color: 'green' }}>VPS 创建成功!</h3>
            {successMessage && !successMessage.includes('Agent配置已复制到剪贴板！') && <p style={{ color: 'green'}}>{successMessage}</p>}
            <p>请将以下配置保存到您的 Agent 的 `agent_config.toml` 文件中，并根据实际情况修改 `server_address`：</p>
            <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all', backgroundColor: '#f0f0f0', padding: '10px', borderRadius: '4px', border: '1px solid #ddd', maxHeight: '150px', overflowY: 'auto' }}>
              <code>{agentConfigToCopy}</code>
            </pre>
            <button onClick={handleCopyToClipboard} style={{ marginTop: '10px', padding: '8px 12px', cursor: 'pointer', marginRight: '10px' }}>
              复制Agent配置
            </button>
            {successMessage && successMessage.includes('Agent配置已复制到剪贴板！') && <span style={{ color: 'green' }}>已复制!</span>}
            <button onClick={onClose} style={{ marginTop: '10px', padding: '8px 12px', cursor: 'pointer' }}>
              关闭
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default CreateVpsModal;