import {
  Server,
  CheckCircle,
  AlertTriangle,
  XCircle,
  ArrowUp,
  ArrowDown,
  List,
  LayoutGrid,
  Cpu,
  MemoryStick,
  HardDrive,
  Globe,
  ArrowLeft,
  ChevronUp,
  ChevronDown,
  ArrowUpDown,
  Pencil,
  FilePenLine,
  X,
  Check,
  Signal, // Added
  Eye,
  EyeOff,
  Plus,
  RefreshCw,
  Clipboard,
  type LucideProps,
} from 'lucide-react';

export const ServerIcon = Server;
export const CheckCircleIcon = CheckCircle;
export const ExclamationTriangleIcon = AlertTriangle;
export const XCircleIcon = XCircle;
export const ArrowUpIcon = ArrowUp;
export const ArrowDownIcon = ArrowDown;
export const ListBulletIcon = List;
export const Squares2X2Icon = LayoutGrid;
export const CpuChipIcon = Cpu;
export const MemoryStickIcon = MemoryStick;
export const HardDiskIcon = HardDrive;
export const GlobeAltIcon = Globe;
export const ArrowLeftIcon = ArrowLeft;
export const ChevronUpIcon = ChevronUp;
export const ChevronDownIcon = ChevronDown;
export const ArrowsUpDownIcon = ArrowUpDown;
export const PencilIcon = Pencil;
export const PencilSquareIcon = FilePenLine;
export const XMarkIcon = X;
export const CheckIcon = Check;
export const SignalIcon = Signal; // Added
export const EyeIcon = Eye;
export const EyeOffIcon = EyeOff;
export const PlusIcon = Plus;
export const RefreshCwIcon = RefreshCw;
export const ClipboardIcon = Clipboard;

export type { LucideProps as SVGProps };
export const ChartBarIcon: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" {...props}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75zM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625zM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125z" />
    </svg>
);