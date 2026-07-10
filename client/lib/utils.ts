import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import { formatDistanceToNow } from "date-fns";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Format a byte count as a human-readable string (e.g. "1.2 GB").
 */
export function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const value = bytes / Math.pow(k, i);
  return `${value.toFixed(decimals)} ${units[i]}`;
}

/**
 * Safe wrapper around date-fns formatDistanceToNow that returns "—" for
 * null / undefined / non-parseable values instead of throwing RangeError.
 * Backend payloads may carry invalid timestamps in some states (no scan yet,
 * empty activity feed, etc.) and an uncaught throw in a component blanks
 * the whole window.
 */
export function safeDistance(
  value: string | number | Date | null | undefined,
  options: { addSuffix?: boolean } = { addSuffix: true },
): string {
  if (value == null) return "—";
  const d = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(d.getTime())) return "—";
  return formatDistanceToNow(d, options);
}
