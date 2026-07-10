/**
 * EntityTypeFilterBar — 7-pill filter for All + 6 entity types.
 * Uses FilterChip styling pattern from SearchPage.tsx.
 */

import { cn } from "@/lib/utils";

const ENTITY_TYPES = [
  { value: "all", label: "All" },
  { value: "person", label: "Person" },
  { value: "organization", label: "Organization" },
  { value: "location", label: "Location" },
  { value: "date", label: "Date" },
  { value: "amount", label: "Amount" },
  { value: "email", label: "Email" },
];

interface EntityTypeFilterBarProps {
  active: string;
  onSelect: (value: string) => void;
}

export function EntityTypeFilterBar({ active, onSelect }: EntityTypeFilterBarProps) {
  return (
    <div className="flex flex-wrap gap-2">
      {ENTITY_TYPES.map(({ value, label }) => (
        <button
          key={value}
          type="button"
          onClick={() => onSelect(value)}
          className={cn(
            "px-3 py-1 rounded-full text-xs font-medium transition-colors border",
            active === value
              ? "bg-accent-primary text-white border-accent-primary"
              : "bg-bg-secondary text-text-secondary border-border-primary hover:bg-bg-tertiary",
          )}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
