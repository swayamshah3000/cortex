/**
 * EntityChip — Reusable clickable entity chip component.
 * Extracted from DocumentPage.tsx inline entity render (Plan 06-06).
 *
 * Phase 8 (08-08): Extended to handle all 8 locked entity classes per UI-SPEC §5.
 * - New classes: Phone (text-teal-400) and Identifier (text-orange-400)
 * - Subclass Badge: shown only for Identifier entities with a known (non-"unknown") subclass
 * - Icon lookup: prefers entity.class (Phase 8), falls back to legacy entity.entityType (Phase 6)
 *
 * Phase 11 (11-05): Refactored from <Link> to <button> for dual-navigation (D-02, D-03, D-17).
 * - Left click  → /search?entity={class}:{value} (filter SearchPage)
 * - Right click → /entity/{class}/{value} (EntityDetailPage, Phase 11 route)
 * - Added isActive prop for accent styling when chip matches active URL filter
 * - TODO(Phase 11 v2): Add touch-based long-press support for right-click equivalent on mobile.
 *   See 11-RESEARCH.md pitfall #7. Desktop-first for now; onContextMenu covers macOS/Linux/Windows.
 */

import { useNavigate } from "react-router-dom";
import {
  Calendar,
  DollarSign,
  Users,
  Building2,
  MapPin,
  Mail,
  Tag,
  Phone,
  Fingerprint,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// 8-class icon map (UI-SPEC §5 — locked schema)
// ---------------------------------------------------------------------------

/** Map from Phase 8 class string (capitalized) → { icon, colorClass } */
function getIconForClass(cls: string | undefined): React.ReactNode {
  switch (cls) {
    case "Person":
      return <Users size={14} className="text-purple-400" />;
    case "Organization":
      return <Building2 size={14} className="text-amber-400" />;
    case "Location":
      return <MapPin size={14} className="text-red-400" />;
    case "Date":
      return <Calendar size={14} className="text-blue-400" />;
    case "Amount":
      return <DollarSign size={14} className="text-green-400" />;
    case "Email":
      return <Mail size={14} className="text-cyan-400" />;
    case "Phone":
      return <Phone size={14} className="text-teal-400" />;
    case "Identifier":
      return <Fingerprint size={14} className="text-orange-400" />;
    default:
      return <Tag size={14} className="text-text-tertiary" />;
  }
}

/**
 * Maps legacy Phase 6 lowercase entityType strings to Phase 8 class names.
 * Allows backward-compatible rendering without a class field.
 */
function mapLegacyEntityTypeToClass(entityType: string): string | undefined {
  switch (entityType.toLowerCase()) {
    case "person":
      return "Person";
    case "organization":
      return "Organization";
    case "location":
      return "Location";
    case "date":
      return "Date";
    case "amount":
      return "Amount";
    case "email":
      return "Email";
    case "phone":
      return "Phone";
    case "identifier":
      return "Identifier";
    default:
      return undefined;
  }
}

// EntityChip accepts either the full Phase 8 ExtractedEntity OR the minimal Phase 6 shape
// (value + entityType + optional canonicalId). `label` is in ExtractedEntity but not used
// by EntityChip rendering — keeping the prop shape permissive preserves backward compat.
interface EntityChipProps {
  entity: {
    value: string;
    entityType: string;
    canonicalId?: string;
    // Phase 8 additions (optional for Phase 6 backward compat)
    class?: string;
    subclass?: string;
    confidence?: number;
  };
  // Phase 11: when true, renders accent styling (chip matches active ?entity= URL param)
  isActive?: boolean;
}

// WR-06: allowlist of known entity classes — used to guard navigation URLs so that
// an unrecognized class (e.g. from malformed IPC data) cannot inject an arbitrary
// path segment. "Unknown" is the safe fallback for unrecognized values.
const KNOWN_ENTITY_CLASSES = new Set([
  "Person",
  "Organization",
  "Location",
  "Date",
  "Amount",
  "Email",
  "Phone",
  "Identifier",
]);

export function EntityChip({ entity, isActive = false }: EntityChipProps) {
  const navigate = useNavigate();

  // Prefer Phase 8 class field; fall back to legacy entityType mapping; last resort: raw entityType
  const resolvedClass =
    entity.class ?? mapLegacyEntityTypeToClass(entity.entityType) ?? entity.entityType;
  const icon = getIconForClass(resolvedClass);

  // Subclass Badge: only for Identifier entities with a known (non-"unknown") subclass (UI-SPEC §5)
  const showSubclassBadge =
    resolvedClass === "Identifier" &&
    entity.subclass != null &&
    entity.subclass !== "" &&
    entity.subclass !== "unknown";

  // WR-06: validate resolvedClass is one of the 8 known classes before building URLs.
  // mapLegacyEntityTypeToClass can return undefined, falling back to entity.entityType
  // directly as resolvedClass — an unknown entityType would embed an arbitrary string
  // in the route. Using "Unknown" as the safe fallback.
  const safeClass = KNOWN_ENTITY_CLASSES.has(resolvedClass) ? resolvedClass : "Unknown";

  // Phase 11 dual-navigation handlers
  const handleClick = () => {
    navigate(`/search?entity=${encodeURIComponent(`${safeClass}:${entity.value}`)}`);
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    navigate(
      `/entity/${encodeURIComponent(safeClass)}/${encodeURIComponent(entity.value)}`,
    );
  };

  return (
    <button
      type="button"
      onClick={handleClick}
      onContextMenu={handleContextMenu}
      aria-label={`Filter by ${resolvedClass}: ${entity.value}. Right-click for entity detail page.`}
      className={cn(
        "inline-flex items-center gap-2 px-2 py-1 rounded-full border transition-colors cursor-pointer",
        "focus-visible:ring-2 focus-visible:ring-accent-primary focus-visible:ring-offset-2 focus-visible:outline-none",
        isActive
          ? "bg-accent-subtle text-accent-primary border-accent-primary/20"
          : "bg-bg-tertiary border-border-secondary hover:bg-accent-subtle",
      )}
    >
      {icon}
      <span className="text-sm text-text-primary truncate max-w-[160px]">{entity.value}</span>
      {showSubclassBadge && (
        <Badge variant="outline" className="text-xs font-mono ml-0.5">
          {entity.subclass}
        </Badge>
      )}
    </button>
  );
}
