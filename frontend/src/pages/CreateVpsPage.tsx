import React, { useState } from 'react';
import { createVps } from '../services/vpsService';
import type { Vps } from '../types'; // Ensure Vps type is imported
import axios from 'axios';

const CreateVpsPage: React.FC = () => {
  const [vpsName, setVpsName] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [agentConfigToCopy, setAgentConfigToCopy] = useState<string | null>(null);

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
      const newVps: Vps = await createVps({ name: vpsName });
      setSuccessMessage(`VPS "${newVps.name}" 创建成功！ID: ${newVps.id}`);
      
      // Prepare agent configuration for copying
      // IMPORTANT: User needs to replace YOUR_SERVER_IP_OR_DOMAIN
      const configContent = `# Agent Configuration for VPS: ${newVps.name} (ID: ${newVps.id})
# 请将此内容保存到 Agent 的 config.toml 文件中
# 并确保将 YOUR_SERVER_IP_OR_DOMAIN:50051 替换为实际的服务器gRPC地址和端口

server_address = "http://YOUR_SERVER_IP_OR_DOMAIN:50051"
vps_id = ${newVps.id}
agent_secret = "${newVps.agent_secret}"
`;
      setAgentConfigToCopy(configContent);
      setVpsName(''); // Clear input after successful creation

    } catch (err: unknown) {
      console.error('Failed to create VPS:', err);
      let errorMessage = '创建VPS失败，请稍后再试。';
      if (axios.isAxiosError(err)) { // Check if it's an AxiosError
        if (err.response?.data?.error) {
          errorMessage = err.response.data.error;
        } else if (err.message) {
          errorMessage = err.message;
        }
      } else if (err instanceof Error) { // Fallback for generic Error
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

  return (
    <div style={{ maxWidth: '500px', margin: '20px auto', padding: '20px', border: '1px solid #ccc', borderRadius: '8px' }}>
      <h1>创建新的VPS</h1>
      <form onSubmit={handleSubmit}>
        <div style={{ marginBottom: '15px' }}>
          <label htmlFor="vpsName" style={{ display: 'block', marginBottom: '5px' }}>VPS 名称:</label>
          <input
            type="text"
            id="vpsName"
            value={vpsName}
            onChange={(e) => setVpsName(e.target.value)}
            placeholder="例如：我的Web服务器"
            required
            style={{ width: '100%', padding: '8px', boxSizing: 'border-box' }}
          />
        </div>
        <button type="submit" disabled={isLoading} style={{ padding: '10px 15px', cursor: 'pointer' }}>
          {isLoading ? '创建中...' : '创建VPS'}
        </button>
      </form>

      {error && <p style={{ color: 'red', marginTop: '15px' }}>错误: {error}</p>}
      {successMessage && !agentConfigToCopy && <p style={{ color: 'green', marginTop: '15px' }}>{successMessage}</p>}
      
      {agentConfigToCopy && (
        <div style={{ marginTop: '20px', padding: '15px', border: '1px solid #eee', borderRadius: '4px', backgroundColor: '#f9f9f9' }}>
          <h3 style={{ marginTop: '0' }}>VPS 创建成功!</h3>
          {successMessage && <p style={{ color: 'green'}}>{successMessage.split(' Agent配置已复制到剪贴板！')[0]}</p>}
          <p>请将以下配置保存到您的 Agent 的 `agent_config.toml` 文件中，并根据实际情况修改 `server_address`：</p>
          <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all', backgroundColor: '#eee', padding: '10px', borderRadius: '4px', border: '1px solid #ddd' }}>
            <code>{agentConfigToCopy}</code>
          </pre>
          <button onClick={handleCopyToClipboard} style={{ marginTop: '10px', padding: '8px 12px', cursor: 'pointer' }}>
            复制Agent配置到剪贴板
          </button>
           {successMessage && successMessage.includes('Agent配置已复制到剪贴板！') && <p style={{ color: 'green', marginTop: '5px' }}>Agent配置已复制到剪贴板！</p>}
        </div>
      )}
    </div>
  );
};

export default CreateVpsPage;