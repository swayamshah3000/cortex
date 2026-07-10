/**
 * EntityTypeBadge — Pill badge showing entity type with color token.
 * Used by EntityCard and EntityDetailHeader (Plan 07).
 */

import {
  Calendar,
  DollarSign,
  Users,
  Building2,
  MapPin,
  Mail,
  Tag,
} from "lucide-react";

/** Token map for entity type colors (6 supported types). */
const tokenMap: Record<string, { text: string; bg: string; border: string }> = {
  person: {
    text: "text-purple-400",
    bg: "bg-purple-400/10",
    border: "border-purple-400/30",
  },
  organization: {
    text: "text-amber-400",
    bg: "bg-amber-400/10",
    border: "border-amber-400/30",
  },
  location: {
    text: "text-red-400",
    bg: "bg-red-400/10",
    border: "border-red-400/30",
  },
  date: {
    text: "text-blue-400",
    bg: "bg-blue-400/10",
    border: "border-blue-400/30",
  },
  amount: {
    text: "text-green-400",
    bg: "bg-green-400/10",
    border: "border-green-400/30",
  },
  email: {
    text: "text-cyan-400",
    bg: "bg-cyan-400/10",
    border: "border-cyan-400/30",
  },
};

const fallbackToken = {
  text: "text-text-tertiary",
  bg: "bg-bg-tertiary",
  border: "border-border-secondary",
};

function entityTypeIcon(entityType: string, size = 12) {
  switch (entityType) {
    case "date":
      return <Calendar size={size} />;
    case "amount":
      return <DollarSign size={size} />;
    case "person":
      return <Users size={size} />;
    case "organization":
      return <Building2 size={size} />;
    case "location":
      return <MapPin size={size} />;
    case "email":
      return <Mail size={size} />;
    default:
      return <Tag size={size} />;
  }
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

interface EntityTypeBadgeProps {
  entityType: string;
}

export function EntityTypeBadge({ entityType }: EntityTypeBadgeProps) {
  const token = tokenMap[entityType] ?? fallbackToken;

  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs font-medium ${token.bg} ${token.text} border ${token.border}`}
    >
      {entityTypeIcon(entityType, 12)}
      {capitalize(entityType)}
    </span>
  );
}

/** Exported for use in EntityCard and other components that need color tokens. */
export { tokenMap };
