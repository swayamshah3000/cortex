/**
 * Shared date/time formatting utilities.
 *
 * Extracted from SpacesPage.tsx (Plan 09-06) to be shared across SpaceCard,
 * SpacesPage SpaceRow, and SpaceDetailPage.
 *
 * SpaceDetailPage.tsx currently has an inline duplicate — Plan 09-07 will replace it.
 */

/**
 * Returns a human-readable relative time string for an ISO date string.
 * Examples: "4m ago", "2h ago", "5d ago".
 */
export function formatRelativeTime(isoDate: string): string {
  const ms = Date.now() - new Date(isoDate).getTime();
  const minutes = Math.floor(ms / 60_000);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}
