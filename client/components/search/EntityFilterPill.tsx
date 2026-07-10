import { X, User, Building2, MapPin, Calendar, DollarSign, Mail, Phone, Hash } from "lucide-react";
import { cn } from "../../lib/utils";

/**
 * Returns a 10px icon for a given entity class string.
 * Mirrors the Phase 8 entity class icon map from EntityChip.tsx without importing from it
 * (loose coupling — EntityFilterPill is a search-specific component, not an entity component).
 *
 * Classes: Person | Organization | Location | Date | Amount | Email | Phone | Identifier
 */
function getClassIcon(cls: string) {
  switch (cls) {
    case "Person":
      return <User size={10} className="text-purple-400 flex-shrink-0" />;
    case "Organization":
      return <Building2 size={10} className="text-amber-400 flex-shrink-0" />;
    case "Location":
      return <MapPin size={10} className="text-blue-400 flex-shrink-0" />;
    case "Date":
      return <Calendar size={10} className="text-green-400 flex-shrink-0" />;
    case "Amount":
      return <DollarSign size={10} className="text-emerald-400 flex-shrink-0" />;
    case "Email":
      return <Mail size={10} className="text-cyan-400 flex-shrink-0" />;
    case "Phone":
      return <Phone size={10} className="text-teal-400 flex-shrink-0" />;
    case "Identifier":
      return <Hash size={10} className="text-slate-400 flex-shrink-0" />;
    default:
      return <Hash size={10} className="text-text-tertiary flex-shrink-0" />;
  }
}

/**
 * A removable accent-tinted pill representing one active entity filter on the SearchPage.
 *
 * encodedParam: "{class}:{value}" string from `?entity=` URL param (e.g. "Person:Alex Doe").
 * onRemove: called with the same encodedParam so the parent can drop it from URL search params.
 *
 * UI-SPEC §2: pill uses bg-accent-subtle, remove button has 44x44 touch target.
 */
export function EntityFilterPill({
  encodedParam,
  onRemove,
}: {
  encodedParam: string;
  onRemove: (encodedParam: string) => void;
}) {
  // Split on the FIRST ':' to handle values that may contain colons (e.g. "Identifier:AB:CD").
  const colonIdx = encodedParam.indexOf(":");
  const cls = colonIdx !== -1 ? encodedParam.slice(0, colonIdx) : encodedParam;
  const value = colonIdx !== -1 ? encodedParam.slice(colonIdx + 1) : "";

  return (
    <div
      className={cn(
        "inline-flex items-center gap-1 px-2 py-1 rounded-full border",
        "border-accent-primary/20 bg-accent-subtle text-accent-primary text-xs",
      )}
    >
      {getClassIcon(cls)}
      <span className="truncate max-w-[140px]">
        {cls}: {value}
      </span>
      {/* 44x44 touch target per UI-SPEC §2 */}
      <button
        type="button"
        onClick={() => onRemove(encodedParam)}
        aria-label={`Remove ${cls}: ${value} filter`}
        className="ml-1 rounded-full p-1 hover:bg-accent-primary/20 focus-visible:ring-1 focus-visible:ring-accent-primary"
        style={{ minWidth: 44, minHeight: 44 }}
      >
        <X size={10} />
      </button>
    </div>
  );
}
