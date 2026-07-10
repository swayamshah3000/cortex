/**
 * EntityHintChip — Outline-only chip showing the canonical entity hint for a space.
 *
 * Displays "{ClassName}: {value}" (e.g. "Person: Alex Doe") with an entity-class icon
 * from the Phase 8 semantic map. Non-interactive (div, not button/Link).
 *
 * Visual contract (09-UI-SPEC.md §4):
 * - bg-transparent border border-border-secondary rounded-full (outline-only — distinct from
 *   Phase 8 topic chip (filled indigo) and tag chip (filled neutral))
 * - px-2 py-1 (8px / 4px — both grid-aligned, 4px base grid)
 * - max-w-[160px], inline-flex items-center gap-1
 * - Icon: entity-class icon at size 10px, strokeWidth 1.5, color from semantic map
 * - Text: text-xs text-text-tertiary truncate — full hint string
 *
 * Phase 11 may upgrade this to a Link navigating to entity-detail page.
 *
 * Plan 09-06 Task 1.
 */

import {
  Users,
  Building2,
  MapPin,
  Calendar,
  DollarSign,
  Mail,
  Phone,
  Fingerprint,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

interface IconEntry {
  Icon: LucideIcon;
  color: string;
}

/**
 * Entity class → icon + color map.
 * Matches Phase 8 EntityChip map (09-UI-SPEC.md §4).
 * Keys are lowercase to enable case-insensitive lookup.
 */
const ICON_MAP: Record<string, IconEntry> = {
  person: { Icon: Users, color: "text-purple-400" },
  organization: { Icon: Building2, color: "text-amber-400" },
  location: { Icon: MapPin, color: "text-red-400" },
  date: { Icon: Calendar, color: "text-blue-400" },
  amount: { Icon: DollarSign, color: "text-green-400" },
  email: { Icon: Mail, color: "text-cyan-400" },
  phone: { Icon: Phone, color: "text-teal-400" },
  identifier: { Icon: Fingerprint, color: "text-orange-400" },
};

const DEFAULT_ENTRY: IconEntry = { Icon: Fingerprint, color: "text-orange-400" };

interface EntityHintChipProps {
  hint: string; // "{ClassName}: {value}" — e.g. "Person: Alex Doe"
}

export function EntityHintChip({ hint }: EntityHintChipProps) {
  // Split on first ": " to derive entity class name for icon lookup
  const sepIdx = hint.indexOf(": ");
  const className = sepIdx !== -1 ? hint.slice(0, sepIdx).toLowerCase() : "";

  const { Icon, color } = ICON_MAP[className] ?? DEFAULT_ENTRY;

  return (
    <div
      role="note"
      aria-label={`Space organized around ${hint}`}
      className="inline-flex items-center gap-1 bg-transparent border border-border-secondary rounded-full px-2 py-1 max-w-[160px]"
    >
      <Icon size={10} strokeWidth={1.5} className={color} />
      <span className="text-xs text-text-tertiary truncate">{hint}</span>
    </div>
  );
}
