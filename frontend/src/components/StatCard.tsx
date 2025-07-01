import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface StatCardProps {
  title: string;
  value: string | number;
  unit?: string;
  icon?: React.ReactNode;
  description?: string;
  onClick?: () => void;
  isActive?: boolean;
  className?: string;
  valueClassName?: string;
}

const StatCard: React.FC<StatCardProps> = ({
  title,
  value,
  unit,
  icon,
  description,
  onClick,
  isActive,
  className,
  valueClassName,
}) => {
  const cardClasses = cn(
    "transition-all",
    onClick && "cursor-pointer hover:border-primary/50",
    isActive && "border-primary shadow-lg",
    className
  );

  const content = (
    <>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
        {icon && <div className={cn("text-muted-foreground", valueClassName)}>{icon}</div>}
      </CardHeader>
      <CardContent>
        <div className={cn("text-2xl font-bold", valueClassName)}>
          {value}
          {unit && <span className="text-base font-normal text-muted-foreground ml-1">{unit}</span>}
        </div>
        {description && (
          <p className="text-xs text-muted-foreground">{description}</p>
        )}
      </CardContent>
    </>
  );

  if (onClick) {
    return (
      <Card
        className={cardClasses}
        onClick={onClick}
        role="button"
        aria-pressed={isActive}
        tabIndex={0}
        onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && onClick()}
      >
        {content}
      </Card>
    );
  }

  return (
    <Card className={cardClasses}>
      {content}
    </Card>
  );
};

export default React.memo(StatCard);