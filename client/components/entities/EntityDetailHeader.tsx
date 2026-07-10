/**
 * EntityDetailHeader — Header for /entities/:id showing icon tile,
 * canonical name (with inline rename), type badge, and doc-count.
 *
 * Plan 06-07 Task 1
 */

import { useState, useEffect, useRef } from "react";
import {
  Pencil,
  Check,
  X,
  Calendar,
  DollarSign,
  Users,
  Building2,
  MapPin,
  Mail,
  Tag,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { EntityTypeBadge } from "./EntityTypeBadge";
import type { CanonicalEntity } from "@/lib/types";
import { tokenMap } from "./EntityTypeBadge";

/** Icon resolution for entity types — used for 28px icon tile in header */
const entityTypeIconMap: Record<string, LucideIcon> = {
  date: Calendar,
  amount: DollarSign,
  person: Users,
  organization: Building2,
  location: MapPin,
  email: Mail,
};

function resolveEntityIcon(entityType: string): LucideIcon {
  return entityTypeIconMap[entityType] ?? Tag;
}

interface EntityDetailHeaderProps {
  entity: CanonicalEntity;
  onRename: (newName: string) => void;
}

export function EntityDetailHeader({ entity, onRename }: EntityDetailHeaderProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState(entity.canonicalName);
  const inputRef = useRef<HTMLInputElement>(null);

  // Sync editValue when entity changes (e.g. after successful rename)
  useEffect(() => {
    setEditValue(entity.canonicalName);
  }, [entity.canonicalName]);

  // Focus + select all when entering edit mode
  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isEditing]);

  const tokens = tokenMap[entity.entityType] ?? {
    text: "text-text-tertiary",
    bg: "bg-bg-tertiary",
    border: "border-border-secondary",
  };
  const IconComp = resolveEntityIcon(entity.entityType);

  function handlePencilClick() {
    setEditValue(entity.canonicalName);
    setIsEditing(true);
  }

  function handleSave() {
    const trimmed = editValue.trim();
    if (trimmed && trimmed !== entity.canonicalName) {
      onRename(trimmed);
    }
    setIsEditing(false);
  }

  function handleCancel() {
    setEditValue(entity.canonicalName);
    setIsEditing(false);
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      handleSave();
    } else if (e.key === "Escape") {
      e.preventDefault();
      handleCancel();
    }
  }

  return (
    <div className="flex items-start gap-4">
      {/* Icon tile */}
      <div className={`p-3 rounded-lg ${tokens.bg} ${tokens.text}`}>
        <IconComp size={28} />
      </div>

      {/* Right column */}
      <div className="flex-1 space-y-2">
        {isEditing ? (
          <div className="flex items-center gap-2">
            <input
              ref={inputRef}
              type="text"
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onKeyDown={handleKeyDown}
              maxLength={200}
              className="input-base text-2xl font-semibold py-1 flex-1"
              aria-label="Rename canonical name input"
            />
            <button
              onClick={handleSave}
              className="p-1 rounded hover:bg-bg-tertiary text-accent-primary"
              aria-label="Save canonical name"
            >
              <Check size={16} />
            </button>
            <button
              onClick={handleCancel}
              className="p-1 rounded hover:bg-bg-tertiary text-text-tertiary"
              aria-label="Cancel rename"
            >
              <X size={16} />
            </button>
          </div>
        ) : (
          <div className="flex items-center gap-2">
            <h1 className="page-title text-text-primary">{entity.canonicalName}</h1>
            <button
              onClick={handlePencilClick}
              className="ml-2 p-1 rounded hover:bg-bg-tertiary text-text-tertiary hover:text-text-primary transition-colors"
              aria-label="Rename canonical name"
            >
              <Pencil size={16} />
            </button>
          </div>
        )}

        <div className="flex items-center gap-3">
          <EntityTypeBadge entityType={entity.entityType} />
          <span className="text-text-secondary text-sm">{entity.documentCount} documents</span>
        </div>
      </div>
    </div>
  );
}
