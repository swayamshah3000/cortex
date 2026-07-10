/**
 * AiNoProviderBanner.tsx
 *
 * Session-only dismissible banner shown when onboarding is completed but no AI
 * provider is connected (D-14, D-15).
 *
 * The "show or hide" decision lives in AppShell (Task 3), NOT here.
 * This component always renders when mounted; AppShell controls mounting via
 * the showBanner conditional.
 *
 * D-15 contract: dismiss is session-only — banner returns on next app launch
 * until a provider is connected. useAiBannerStore has NO persist middleware
 * (enforced by stores.test.ts).
 *
 * Banner copy (D-14, UI-SPEC Copywriting):
 *   "Connect an AI provider to enable Smart Spaces"   — main copy
 *   "Go to Settings →"                                 — link → /settings?tab=ai
 *
 * Color tokens: ALWAYS use semantic tokens — never hardcode hex values.
 */

import { useAiBannerStore } from "@/lib/stores";
import { useNavigate } from "react-router-dom";
import { X, AlertTriangle } from "lucide-react";

export function AiNoProviderBanner() {
  const { dismiss } = useAiBannerStore();
  const navigate = useNavigate();

  return (
    <div
      role="alert"
      aria-live="polite"
      className="flex items-center gap-3 px-4 py-4 bg-warning/10 border-b border-warning/20 text-sm"
    >
      {/* Left: warning icon */}
      <AlertTriangle size={14} className="text-warning flex-shrink-0" />

      {/* Middle: copy + settings link + D-34 sub-copy */}
      <div className="flex-1">
        <p className="text-text-secondary text-sm">
          Connect an AI provider to enable Smart Spaces.{" "}
          <button
            type="button"
            onClick={() => navigate("/settings?tab=ai")}
            className="text-accent-primary hover:underline"
          >
            Go to Settings &rarr;
          </button>
        </p>
        <p className="text-xs text-text-tertiary mt-1">
          Connect AI to extract people, organizations, and topic tags from your
          docs. Dates, amounts, and IDs work without AI.
        </p>
      </div>

      {/* Right: dismiss button (44px touch target per UI-SPEC) */}
      <button
        type="button"
        onClick={dismiss}
        aria-label="Dismiss banner"
        className="text-text-tertiary hover:text-text-primary min-w-[44px] min-h-[44px] flex items-center justify-center transition-colors"
      >
        <X size={14} />
      </button>
    </div>
  );
}
