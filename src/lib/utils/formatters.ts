// src/lib/utils/formatters.ts
// Formatting utilities

/**
 * Format bytes to human-readable string
 */
export function formatBytes(bytes: number, decimals: number = 2): string {
  if (bytes === 0) return '0 Bytes';

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];

  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

/**
 * Format duration in seconds to human-readable string
 */
export function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return `${Math.round(seconds)}s`;
  }

  const mins = Math.floor(seconds / 60);
  const secs = Math.round(seconds % 60);

  if (mins < 60) {
    return secs > 0 ? `${mins}m ${secs}s` : `${mins}m`;
  }

  const hours = Math.floor(mins / 60);
  const remainingMins = mins % 60;

  return remainingMins > 0 ? `${hours}h ${remainingMins}m` : `${hours}h`;
}

/**
 * Format ETA seconds to human-readable string
 */
export function formatEta(seconds: number | null): string {
  if (seconds === null || seconds < 0) return '--';
  return formatDuration(seconds);
}

/**
 * Format throughput in MiB/s
 */
export function formatThroughput(mibPerSec: number): string {
  if (mibPerSec < 1) {
    return `${(mibPerSec * 1024).toFixed(1)} KiB/s`;
  }
  if (mibPerSec >= 1024) {
    return `${(mibPerSec / 1024).toFixed(2)} GiB/s`;
  }
  return `${mibPerSec.toFixed(2)} MiB/s`;
}

/**
 * Format percentage with specified decimals
 */
export function formatPercent(value: number, decimals: number = 1): string {
  return `${value.toFixed(decimals)}%`;
}

/**
 * Format number with thousand separators
 */
export function formatNumber(num: number): string {
  return num.toLocaleString();
}

/**
 * Format hex offset
 */
export function formatHexOffset(offset: number): string {
  return `0x${offset.toString(16).toUpperCase().padStart(8, '0')}`;
}

/**
 * Format timestamp to locale string
 */
export function formatTimestamp(date: Date | string): string {
  const d = typeof date === 'string' ? new Date(date) : date;
  return d.toLocaleString();
}

/**
 * Format timestamp to time only
 */
export function formatTime(date: Date | string): string {
  const d = typeof date === 'string' ? new Date(date) : date;
  return d.toLocaleTimeString();
}
