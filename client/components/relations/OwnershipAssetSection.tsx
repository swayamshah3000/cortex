/**
 * OwnershipAssetSection — one asset-type section (Property, Vehicle, etc.)
 * rendered as a card grid on OwnershipPage.
 *
 * Empty sections are hidden entirely (returns null) per Phase 11.5 D-18 /
 * must_haves. Each card links to the object entity's detail page and carries
 * a DeleteTripleButton for user override (D-12).
 *
 * Phase 11.5 Plan 07, Task 3.
 */

import { Link } from "react-router-dom";
import { Home, Car, TrendingUp, Briefcase, Landmark, Package } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { DeleteTripleButton } from "@/components/relations/DeleteTripleButton";
import type { AssetType, TripleWithEntities } from "@/lib/types";

export interface OwnershipAssetSectionProps {
  assetType: AssetType;
  assets: TripleWithEntities[];
}

const ASSET_TYPE_ICON: Record<AssetType, React.ReactNode> = {
  Property: <Home size={18} />,
  Vehicle: <Car size={18} />,
  Investment: <TrendingUp size={18} />,
  Business: <Briefcase size={18} />,
  Financial: <Landmark size={18} />,
  Other: <Package size={18} />,
};

export function OwnershipAssetSection({ assetType, assets }: OwnershipAssetSectionProps) {
  if (assets.length === 0) {
    return null;
  }

  return (
    <section
      className="space-y-3"
      data-testid={`ownership-section-${assetType.toLowerCase()}`}
    >
      <h3 className="text-md font-semibold flex items-center gap-2 text-text-primary">
        {ASSET_TYPE_ICON[assetType]}
        {assetType} ({assets.length})
      </h3>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {assets.map((t) => (
          <div
            key={t.triple.id}
            className="relative border border-border-secondary rounded-lg p-4 hover:bg-bg-secondary transition-colors"
          >
            <div className="absolute top-3 right-3">
              <DeleteTripleButton
                tripleId={t.triple.id}
                affectedEntityIds={[t.triple.subjectId, t.triple.objectId]}
              />
            </div>
            <Link
              to={`/entity/${encodeURIComponent(t.object.entityType)}/${encodeURIComponent(t.object.canonicalName)}`}
              className="block pr-6"
            >
              <p className="text-sm font-bold text-text-primary truncate">
                {t.object.canonicalName}
              </p>
              <p className="text-xs text-text-tertiary mt-0.5 capitalize">
                {t.object.entityType}
              </p>
              <div className="flex items-center gap-2 mt-3">
                <Badge variant="outline" className="text-xs">
                  {t.triple.docIds.length} {t.triple.docIds.length === 1 ? "doc" : "docs"}
                </Badge>
                {t.triple.userAdded && (
                  <Badge variant="secondary" className="text-xs">
                    manual
                  </Badge>
                )}
              </div>
            </Link>
          </div>
        ))}
      </div>
    </section>
  );
}
