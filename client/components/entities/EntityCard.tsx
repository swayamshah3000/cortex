/**
 * EntityCard — Card variant used on EntitiesPage grid.
 * Analog of SubSpaceCard in SpaceDetailPage.tsx.
 */

import { Link } from "react-router-dom";
import {
  Calendar,
  DollarSign,
  Users,
  Building2,
  MapPin,
  Mail,
  Tag,
} from "lucide-react";
import { tokenMap } from "./EntityTypeBadge";
import type { EntitySummary } from "@/lib/types";

function entityTypeIconLarge(entityType: string) {
  switch (entityType) {
    case "date":
      return <Calendar size={18} />;
    case "amount":
      return <DollarSign size={18} />;
    case "person":
      return <Users size={18} />;
    case "organization":
      return <Building2 size={18} />;
    case "location":
      return <MapPin size={18} />;
    case "email":
      return <Mail size={18} />;
    default:
      return <Tag size={18} />;
  }
}

interface EntityCardProps {
  entity: EntitySummary & { aliases?: string[] };
}

export function EntityCard({ entity }: EntityCardProps) {
  const token = tokenMap[entity.entityType] ?? {
    text: "text-text-tertiary",
    bg: "bg-bg-tertiary",
    border: "border-border-secondary",
  };

  const showAliases = entity.aliases && entity.aliases.length > 1;

  return (
    <Link
      to={`/entities/${entity.id}`}
      className="card p-4 hover:shadow-md hover:border-accent-primary/50 transition-all"
    >
      <div className="flex items-center gap-3">
        <div className={`p-2 rounded-lg ${token.bg} ${token.text} flex-shrink-0`}>
          {entityTypeIconLarge(entity.entityType)}
        </div>
        <div className="flex-1 min-w-0">
          <p className="font-medium text-text-primary truncate">{entity.canonicalName}</p>
          <p className="text-xs text-text-tertiary">
            {entity.documentCount} doc{entity.documentCount !== 1 ? "s" : ""}
          </p>
          {showAliases && (
            <p className="text-xs text-text-tertiary">{entity.aliases!.length} aliases</p>
          )}
        </div>
      </div>
    </Link>
  );
}
