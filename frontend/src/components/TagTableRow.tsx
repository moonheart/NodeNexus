import React from 'react';
import type { Tag as TagType } from '../types';
import { Pencil, Trash2, Link as LinkIcon, Eye, EyeOff, icons } from 'lucide-react';
import type { LucideProps } from 'lucide-react';
import { Button } from "@/components/ui/button";
import { TableCell, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";

// Helper component to safely render a Lucide icon by its string name.
const LucideIcon = ({ name, ...props }: { name: string } & LucideProps) => {
  const IconComponent = icons[name as keyof typeof icons];
  if (!IconComponent) {
    return null; // Or a default icon
  }
  return <IconComponent {...props} />;
};

interface TagTableRowProps {
  tag: TagType;
  onEdit: (tag: TagType) => void;
  onDelete: (tagId: number) => void;
  onToggleVisibility: (tagId: number, isVisible: boolean) => void;
}

const TagTableRow: React.FC<TagTableRowProps> = ({ tag, onEdit, onDelete, onToggleVisibility }) => {
  return (
    <TableRow key={tag.id}>
      <TableCell>
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 rounded-full" style={{ backgroundColor: tag.color }} />
          <div className="flex items-center gap-2">
            {tag.icon && <LucideIcon name={tag.icon} className="w-5 h-5" />}
            <span className="font-medium">{tag.name}</span>
          </div>
        </div>
      </TableCell>
      <TableCell>
        <Badge variant="secondary">{tag.id}</Badge>
      </TableCell>
      <TableCell className="text-center">{tag.vpsCount}</TableCell>
      <TableCell>
        {tag.url ? (
          <a href={tag.url} target="_blank" rel="noopener noreferrer" className="flex items-center gap-1 text-primary hover:underline">
            <LinkIcon className="w-4 h-4" />
            <span>Link</span>
          </a>
        ) : (
          <span className="text-muted-foreground">N/A</span>
        )}
      </TableCell>
      <TableCell className="text-right">
        <div className="flex items-center justify-end gap-1">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => onToggleVisibility(tag.id, !tag.isVisible)}
            title={tag.isVisible ? 'Visible' : 'Hidden'}
            className={!tag.isVisible ? 'text-muted-foreground' : ''}
          >
            {tag.isVisible ? <Eye className="w-5 h-5" /> : <EyeOff className="w-5 h-5" />}
          </Button>
          <Button variant="ghost" size="icon" onClick={() => onEdit(tag)}>
            <Pencil className="w-4 h-4" />
          </Button>
          <Button variant="ghost" size="icon" onClick={() => onDelete(tag.id)} className="text-destructive hover:text-destructive/80">
            <Trash2 className="w-4 h-4" />
          </Button>
        </div>
      </TableCell>
    </TableRow>
  );
};

export default TagTableRow;