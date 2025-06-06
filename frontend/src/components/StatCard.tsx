import React from 'react';

interface StatCardProps {
  title: string;
  value: string | number;
  unit?: string;
  icon?: React.ReactNode;
  colorClass?: string; // Tailwind CSS text color class, e.g., 'text-blue-500'
  description?: string;
  onClick?: () => void;
  isActive?: boolean; // To highlight the card if it's an active filter
}

const StatCard: React.FC<StatCardProps> = ({
  title,
  value,
  unit,
  icon,
  colorClass = 'text-slate-700', // Default color
  description,
  onClick,
  isActive,
}) => {
  const cardBaseClasses = "p-5 bg-white rounded-lg shadow-md transition-all duration-300 flex flex-col justify-between";
  // Enhanced active state: more prominent shadow, slight scale, and a border matching the icon color
  const activeClasses = isActive
    ? `ring-2 ring-offset-2 ${colorClass.replace('text-', 'ring-').replace('slate-700', 'indigo-500')} shadow-xl scale-105` // Use a specific ring color for active, e.g. indigo
    : "hover:shadow-lg";
  const clickableClasses = onClick ? "cursor-pointer" : "";

  // Determine icon background color based on text color for better contrast or theming
  // Example: if text is text-green-500, background could be bg-green-100
  const iconBgClass = icon ? `${colorClass.replace('text-', 'bg-')}` : '';

  const content = (
    <>
      <div>
        <div className="flex items-center justify-between mb-1">
          <h3 className="text-sm font-medium text-slate-500 uppercase tracking-wider">{title}</h3>
          {icon && (
            <div className={`p-2 rounded-full ${iconBgClass}`}>
              {React.cloneElement(icon as React.ReactElement<{ className?: string }>, {
                className: `w-5 h-5 ${colorClass}`,
              })}
            </div>
          )}
        </div>
        <p className={`text-3xl font-semibold ${colorClass}`}>
          {value}
          {unit && <span className="text-base ml-1">{unit}</span>}
        </p>
      </div>
      {description && <p className="text-xs text-slate-400 mt-2">{description}</p>}
    </>
  );

  if (onClick) {
    return (
      <button
        type="button"
        onClick={onClick}
        className={`w-full text-left ${cardBaseClasses} ${activeClasses} ${clickableClasses}`}
        aria-pressed={isActive}
      >
        {content}
      </button>
    );
  }

  return (
    <div className={`${cardBaseClasses} ${activeClasses}`}>
      {content}
    </div>
  );
};

export default StatCard;