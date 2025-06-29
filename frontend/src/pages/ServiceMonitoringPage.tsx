import React, { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import ServiceMonitorModal from '../components/ServiceMonitorModal';
import { createMonitor, updateMonitor, getMonitors, deleteMonitor } from '../services/serviceMonitorService';
import type { ServiceMonitor, ServiceMonitorInput } from '../types';
import toast from 'react-hot-toast';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Plus, Pencil, Trash2 } from 'lucide-react';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

const ServiceMonitoringPage: React.FC = () => {
  const [monitors, setMonitors] = useState<ServiceMonitor[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingMonitor, setEditingMonitor] = useState<ServiceMonitor | null>(null);
  const [isAlertOpen, setIsAlertOpen] = useState(false);
  const [deletingMonitorId, setDeletingMonitorId] = useState<number | null>(null);

  const fetchMonitors = async () => {
    try {
      setIsLoading(true);
      const data = await getMonitors();
      setMonitors(data);
      setError(null);
    } catch (err) {
      setError('无法获取服务监控列表。');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchMonitors();
  }, []);

  const handleOpenCreateModal = () => {
    setEditingMonitor(null);
    setIsModalOpen(true);
  };

  const handleOpenEditModal = (monitor: ServiceMonitor) => {
    setEditingMonitor(monitor);
    setIsModalOpen(true);
  };

  const handleDeleteClick = (id: number) => {
    setDeletingMonitorId(id);
    setIsAlertOpen(true);
  };

  const confirmDelete = async () => {
    if (deletingMonitorId === null) return;
    try {
      await deleteMonitor(deletingMonitorId);
      toast.success('监控已成功删除！');
      fetchMonitors(); // Refresh the list
    } catch (err) {
      toast.error('删除监控失败。');
      console.error(err);
    } finally {
      setIsAlertOpen(false);
      setDeletingMonitorId(null);
    }
  };

  const handleSave = async (data: ServiceMonitorInput, id?: number) => {
    try {
      if (id) {
        await updateMonitor(id, data);
        toast.success('监控已成功更新！');
      } else {
        await createMonitor(data);
        toast.success('监控已成功创建！');
      }
      setIsModalOpen(false);
      fetchMonitors(); // Refresh the list
    } catch (err) {
      toast.error('保存监控失败。');
      console.error(err);
    }
  };

  return (
    <div className="container mx-auto p-4 md:p-6 lg:p-8">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-3xl font-bold">服务监控</h1>
        <Button onClick={handleOpenCreateModal}>
          <Plus className="w-4 h-4 mr-2" />
          创建监控
        </Button>
      </div>

      <ServiceMonitorModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSave={handleSave}
        monitorToEdit={editingMonitor}
      />

      <AlertDialog open={isAlertOpen} onOpenChange={setIsAlertOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>确定要删除吗？</AlertDialogTitle>
            <AlertDialogDescription>
              此操作无法撤销。这将永久删除该服务监控项。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setDeletingMonitorId(null)}>取消</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDelete}>确定</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {isLoading && <p className="text-center text-muted-foreground">正在加载监控列表...</p>}
      {error && (
        <Alert variant="destructive">
          <AlertTitle>错误</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
      
      {!isLoading && !error && (
        monitors.length > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {monitors.map((monitor) => (
              <Card key={monitor.id} className="flex flex-col">
                <CardHeader>
                  <CardTitle className="hover:text-primary">
                    <Link to={`/monitors/${monitor.id}`}>{monitor.name}</Link>
                  </CardTitle>
                  <CardDescription className="truncate" title={monitor.target}>
                    {monitor.monitorType.toUpperCase()} - {monitor.target}
                  </CardDescription>
                </CardHeader>
                <CardContent className="flex-grow">
                  <p className="text-sm text-muted-foreground">检查频率: {monitor.frequencySeconds}秒</p>
                </CardContent>
                <CardFooter className="flex justify-end space-x-2">
                  <Button variant="outline" size="sm" onClick={() => handleOpenEditModal(monitor)}>
                    <Pencil className="w-4 h-4 mr-1" /> 编辑
                  </Button>
                  <Button variant="destructive" size="sm" onClick={() => handleDeleteClick(monitor.id)}>
                    <Trash2 className="w-4 h-4 mr-1" /> 删除
                  </Button>
                </CardFooter>
              </Card>
            ))}
          </div>
        ) : (
          <div className="text-center py-16 border-2 border-dashed rounded-lg">
            <h3 className="text-xl font-semibold text-muted-foreground">未找到服务监控项</h3>
            <p className="text-muted-foreground mt-2">点击“创建监控”按钮来添加第一个监控项。</p>
            <Button className="mt-4" onClick={handleOpenCreateModal}>
              <Plus className="w-4 h-4 mr-2" />
              创建监控
            </Button>
          </div>
        )
      )}
    </div>
  );
};

export default ServiceMonitoringPage;