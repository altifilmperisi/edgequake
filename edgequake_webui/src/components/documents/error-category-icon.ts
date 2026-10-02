/**
 * Single mapping from an error category to its lucide icon (SSOT).
 * Shared by the failed-row popover and the document preview panel.
 */
import type { ErrorCategory } from '@/lib/error-categories';
import {
  AlertCircle,
  Brain,
  Cpu,
  Database,
  FileWarning,
  Trash2,
  Wifi,
  type LucideIcon,
} from 'lucide-react';

export function getCategoryIconComponent(category: ErrorCategory): LucideIcon {
  switch (category) {
    case 'llm':
    case 'llm_timeout':
      return Brain;
    case 'embedding':
      return Cpu;
    case 'storage':
      return Database;
    case 'pipeline':
      return FileWarning;
    case 'network':
      return Wifi;
    case 'lifecycle':
      return Trash2;
    default:
      return AlertCircle;
  }
}
