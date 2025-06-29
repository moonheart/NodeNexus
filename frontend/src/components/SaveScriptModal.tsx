import React, { useState, useEffect } from 'react';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

interface Props {
    isOpen: boolean;
    onClose: () => void;
    onSave: (name: string, description: string) => void;
    initialCommand: string;
}

const SaveScriptModal: React.FC<Props> = ({ isOpen, onClose, onSave, initialCommand }) => {
    const [name, setName] = useState('');
    const [description, setDescription] = useState('');

    useEffect(() => {
        if (isOpen) {
            setName('');
            setDescription('');
        }
    }, [isOpen]);

    const handleSave = () => {
        if (name.trim()) {
            onSave(name, description);
            onClose();
        } else {
            // Consider replacing with a toast notification or inline error message
            alert('Script name is required.');
        }
    };

    return (
        <Dialog open={isOpen} onOpenChange={onClose}>
            <DialogContent className="sm:max-w-[425px]">
                <DialogHeader>
                    <DialogTitle>Save Command as Script</DialogTitle>
                    <DialogDescription>
                        Save the current command and working directory for future use.
                    </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                    <div className="grid grid-cols-4 items-center gap-4">
                        <Label htmlFor="script-name" className="text-right">
                            Name
                        </Label>
                        <Input
                            id="script-name"
                            value={name}
                            onChange={(e) => setName(e.target.value)}
                            className="col-span-3"
                            required
                        />
                    </div>
                    <div className="grid grid-cols-4 items-center gap-4">
                        <Label htmlFor="script-description" className="text-right">
                            Description
                        </Label>
                        <Textarea
                            id="script-description"
                            value={description}
                            onChange={(e) => setDescription(e.target.value)}
                            className="col-span-3"
                            rows={3}
                        />
                    </div>
                    <div className="grid grid-cols-4 items-start gap-4">
                        <Label className="text-right pt-2">Command</Label>
                        <div className="col-span-3 bg-muted p-2 rounded-md text-sm font-mono break-all max-h-24 overflow-y-auto">
                            {initialCommand}
                        </div>
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="outline" onClick={onClose}>Cancel</Button>
                    <Button onClick={handleSave}>Save Script</Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};

export default SaveScriptModal;