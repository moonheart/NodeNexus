import React, { useState, useEffect } from 'react'; // Added useEffect
import CreateVpsModal from '../components/CreateVpsModal'; // Adjust path as needed
import type { Vps } from '../types'; // Adjust path as needed
import { getVpsList } from '../services/vpsService'; // Added vpsService import

const HomePage: React.FC = () => {
  const [isCreateVpsModalOpen, setIsCreateVpsModalOpen] = useState(false);
  const [vpsList, setVpsList] = useState<Vps[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchVpsList = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await getVpsList();
      setVpsList(data);
    } catch (err) {
      setError('无法获取VPS列表。');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchVpsList();
  }, []);

  const handleOpenCreateVpsModal = () => {
    setIsCreateVpsModalOpen(true);
  };

  const handleCloseCreateVpsModal = () => {
    setIsCreateVpsModalOpen(false);
  };

  const handleVpsCreated = (newVps: Vps) => {
    console.log('VPS Created:', newVps);
    fetchVpsList(); // Refresh the list
    handleCloseCreateVpsModal(); // Close modal after creation
  };

  return (
    <div style={{ padding: '20px' }}>
      <h1>欢迎来到首页!</h1>
      <p>您已成功登录。</p>
      <button onClick={handleOpenCreateVpsModal} style={{ padding: '10px 15px', margin: '20px 0', cursor: 'pointer', backgroundColor: '#007bff', color: 'white', border: 'none', borderRadius: '4px' }}>
        创建新的VPS
      </button>
      <CreateVpsModal
        isOpen={isCreateVpsModalOpen}
        onClose={handleCloseCreateVpsModal}
        onVpsCreated={handleVpsCreated}
      />

      <h2>您的VPS列表</h2>
      {isLoading && <p>加载中...</p>}
      {error && <p style={{ color: 'red' }}>{error}</p>}
      {!isLoading && !error && vpsList.length === 0 && (
        <p>您还没有任何VPS。点击上面的按钮创建一个吧！</p>
      )}
      {!isLoading && !error && vpsList.length > 0 && (
        <table style={{ width: '100%', borderCollapse: 'collapse', marginTop: '20px' }}>
          <thead>
            <tr style={{ backgroundColor: '#f0f0f0' }}>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>ID</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>名称</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>IP地址</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>状态</th>
              <th style={{ border: '1px solid #ddd', padding: '8px', textAlign: 'left' }}>创建时间</th>
            </tr>
          </thead>
          <tbody>
            {vpsList.map((vps) => (
              <tr key={vps.id}>
                <td style={{ border: '1px solid #ddd', padding: '8px' }}>{vps.id}</td>
                <td style={{ border: '1px solid #ddd', padding: '8px' }}>{vps.name}</td>
                <td style={{ border: '1px solid #ddd', padding: '8px' }}>{vps.ip_address || 'N/A'}</td>
                <td style={{ border: '1px solid #ddd', padding: '8px' }}>{vps.status}</td>
                <td style={{ border: '1px solid #ddd', padding: '8px' }}>{new Date(vps.created_at).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
};

export default HomePage;