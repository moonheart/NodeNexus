import React from 'react';
import type { Vps, VpsListItemResponse } from '../types';
import CommandCopyUI from './CommandCopyUI';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

interface CopyCommandModalProps {
  isOpen: boolean;
  onClose: () => void;
  vps: Vps | VpsListItemResponse | null;
}

const CopyCommandModal: React.FC<CopyCommandModalProps> = ({ isOpen, onClose, vps }) => {
  if (!vps) {
    return null;
  }

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[700px]">
        <DialogHeader>
          <DialogTitle>为 "{vps.name}" 安装 Agent</DialogTitle>
          <DialogDescription>
            请为您的服务器选择对应的操作系统，并复制安装命令。
          </DialogDescription>
        </DialogHeader>
        
        <div className="py-4">
          <CommandCopyUI vps={vps} />
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            关闭
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default CopyCommandModal;