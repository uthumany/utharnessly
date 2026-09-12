import { Badge } from '@vr_patel/tui';
import type { ColorMode } from '../types.js';

export function statusBadge(kind: 'success' | 'error' | 'warning' | 'info' | 'neutral', label: string, mode: ColorMode): string {
  return mode === 'mono' ? `[${label}]` : Badge[kind](label);
}
