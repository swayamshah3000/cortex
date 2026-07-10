import { useState, useEffect, useCallback } from "react";
import {
  Settings2,
  Database,
  Brain,
  Shield,
  HardDrive,
  Info,
  Save,
  Trash2,
  ExternalLink,
} from "lucide-react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettings, useUpdateSettings, useProviders } from "@/hooks/useTauri";
import { toast } from "sonner";
import { useTheme } from "next-themes";
import type { Settings } from "@/lib/types";
import { AiProvidersSection } from "@/components/ai/AiProvidersSection";
import { ExtractionSettings } from "@/components/ai/ExtractionSettings";

// --- Helper: format bytes --------------------------------------------------

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

// --- Tab Icons Map ---------------------------------------------------------

const TAB_ICONS: Record<string, React.ElementType> = {
  general: Settings2,
  indexing: Database,
  ai: Brain,
  privacy: Shield,
  storage: HardDrive,
  about: Info,
};

// --- Loading Skeleton ------------------------------------------------------

function SettingsSkeleton() {
  return (
    <div className="space-y-8">
      <div className="space-y-2">
        <Skeleton className="h-8 w-32" />
        <Skeleton className="h-4 w-64" />
      </div>
      <Skeleton className="h-10 w-full max-w-2xl" />
      <Skeleton className="h-96 w-full" />
    </div>
  );
}

// --- Main Settings Page ----------------------------------------------------

export default function SettingsPage() {
  const { data: settings, isLoading } = useSettings();
  const { mutate: updateSettings, isPending: isSaving } = useUpdateSettings();
  const { setTheme } = useTheme();
  const { data: providers } = useProviders();

  const [local, setLocal] = useState<Settings | null>(null);
  const [isDirty, setIsDirty] = useState(false);

  // D-20: true when OpenAI is connected (used for embedding key unification)
  const openaiConnected = providers?.some((p) => p.provider === "openai" && p.authenticated) ?? false;

  // Sync remote settings into local state
  useEffect(() => {
    if (settings && !local) {
      setLocal(settings);
    }
  }, [settings, local]);

  const update = useCallback(
    (patch: Partial<Settings>) => {
      setLocal((prev) => (prev ? { ...prev, ...patch } : prev));
      setIsDirty(true);
    },
    [],
  );

  const handleSave = useCallback(() => {
    if (!local) return;
    // Apply theme change immediately
    if (local.theme) {
      setTheme(local.theme);
    }
    updateSettings(local, {
      onSuccess: () => {
        setIsDirty(false);
        toast.success("Settings saved successfully.");
      },
      onError: () => {
        toast.error("Failed to save settings.");
      },
    });
  }, [local, updateSettings, setTheme]);

  if (isLoading || !local) {
    return <SettingsSkeleton />;
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="space-y-2">
          <h1 className="page-title text-text-primary">Settings</h1>
          <p className="text-text-secondary">
            Configure Cortex behavior, indexing, AI models, and privacy.
          </p>
        </div>
        {isDirty && (
          <button
            onClick={handleSave}
            disabled={isSaving}
            className="btn-primary flex items-center gap-2"
          >
            <Save size={16} />
            {isSaving ? "Saving..." : "Save Changes"}
          </button>
        )}
      </div>

      {/* Tabs */}
      <Tabs defaultValue="general" className="space-y-6">
        <TabsList className="w-full justify-start flex-wrap h-auto gap-1 bg-bg-secondary border border-border-primary p-1">
          {[
            { value: "general", label: "General" },
            { value: "indexing", label: "Indexing" },
            { value: "ai", label: "AI & Models" },
            { value: "privacy", label: "Privacy" },
            { value: "storage", label: "Storage" },
            { value: "about", label: "About" },
          ].map((tab) => {
            const Icon = TAB_ICONS[tab.value];
            return (
              <TabsTrigger
                key={tab.value}
                value={tab.value}
                className="flex items-center gap-1.5 data-[state=active]:bg-accent-primary data-[state=active]:text-white"
              >
                <Icon size={14} />
                {tab.label}
              </TabsTrigger>
            );
          })}
        </TabsList>

        {/* ----- General Tab ----- */}
        <TabsContent value="general">
          <div className="card p-6 space-y-8">
            <div>
              <h3 className="section-header text-text-primary mb-4">Appearance</h3>
              <div className="space-y-4">
                <div className="space-y-3">
                  <Label className="text-text-secondary text-sm">Theme</Label>
                  <RadioGroup
                    value={local.theme}
                    onValueChange={(val) => {
                      setTheme(val);
                      update({ theme: val });
                    }}
                    className="flex gap-4"
                  >
                    {["dark", "light", "system"].map((t) => (
                      <div key={t} className="flex items-center space-x-2">
                        <RadioGroupItem value={t} id={`theme-${t}`} />
                        <Label htmlFor={`theme-${t}`} className="text-text-primary capitalize cursor-pointer">
                          {t}
                        </Label>
                      </div>
                    ))}
                  </RadioGroup>
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <Label className="text-text-primary">Sidebar collapsed by default</Label>
                    <p className="text-text-tertiary text-xs mt-0.5">
                      Start with a compact sidebar on launch.
                    </p>
                  </div>
                  <Switch
                    checked={local.sidebarCollapsed}
                    onCheckedChange={(checked) => update({ sidebarCollapsed: checked })}
                  />
                </div>
              </div>
            </div>
          </div>
        </TabsContent>

        {/* ----- Indexing Tab ----- */}
        <TabsContent value="indexing">
          <div className="card p-6 space-y-8">
            <div>
              <h3 className="section-header text-text-primary mb-4">Indexing Behavior</h3>
              <div className="space-y-6">
                <div className="flex items-center justify-between">
                  <div>
                    <Label className="text-text-primary">Index on startup</Label>
                    <p className="text-text-tertiary text-xs mt-0.5">
                      Automatically scan watched folders when Cortex launches.
                    </p>
                  </div>
                  <Switch
                    checked={local.indexOnStartup}
                    onCheckedChange={(checked) => update({ indexOnStartup: checked })}
                  />
                </div>

                <div className="space-y-2">
                  <Label className="text-text-primary">Excluded Patterns</Label>
                  <p className="text-text-tertiary text-xs">
                    Comma-separated glob patterns to skip during indexing.
                  </p>
                  <input
                    type="text"
                    className="input-base w-full"
                    value={local.excludedPatterns.join(", ")}
                    onChange={(e) =>
                      update({
                        excludedPatterns: e.target.value
                          .split(",")
                          .map((s) => s.trim())
                          .filter(Boolean),
                      })
                    }
                  />
                </div>

                <div className="space-y-2">
                  <Label className="text-text-primary">Supported File Types</Label>
                  <p className="text-text-tertiary text-xs">
                    File types that will be indexed and searchable.
                  </p>
                  <div className="flex flex-wrap gap-3 mt-2">
                    {["pdf", "docx", "txt", "md", "xlsx", "csv", "png", "jpg"].map((ext) => (
                      <label
                        key={ext}
                        className="flex items-center gap-2 px-3 py-1.5 rounded-md border border-border-primary bg-bg-tertiary text-sm cursor-pointer hover:border-accent-primary transition-colors"
                      >
                        <input type="checkbox" defaultChecked className="accent-[hsl(var(--accent-primary))]" />
                        <span className="text-text-primary uppercase text-xs font-medium">{ext}</span>
                      </label>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </TabsContent>

        {/* ----- AI & Models Tab ----- */}
        <TabsContent value="ai">
          <div className="card p-6 space-y-8">
            {/* Embedding Model section */}
            <div>
              <h3 className="section-header text-text-primary mb-4">Embedding Model</h3>
              <div className="space-y-4">
                <RadioGroup
                  value={local.embeddingModel}
                  onValueChange={(val) => update({ embeddingModel: val })}
                  className="space-y-3"
                >
                  <label className="flex items-start gap-3 p-4 rounded-lg border border-border-primary hover:border-accent-primary/50 transition-colors cursor-pointer">
                    <RadioGroupItem value="local" id="model-local" className="mt-0.5" />
                    <div>
                      <p className="text-text-primary font-medium">Local (all-MiniLM-L6-v2)</p>
                      <p className="text-text-tertiary text-xs mt-1">
                        Runs entirely on your machine using ONNX Runtime. No data leaves your device. Good
                        balance of speed and quality.
                      </p>
                    </div>
                  </label>
                  <label className="flex items-start gap-3 p-4 rounded-lg border border-border-primary hover:border-accent-primary/50 transition-colors cursor-pointer">
                    <RadioGroupItem value="openai" id="model-openai" className="mt-0.5" />
                    <div>
                      <p className="text-text-primary font-medium">OpenAI (text-embedding-3-small)</p>
                      <p className="text-text-tertiary text-xs mt-1">
                        Higher quality embeddings via API. Requires API key. Document content is sent to
                        OpenAI servers.
                      </p>
                    </div>
                  </label>
                </RadioGroup>

                {/* D-20 embedding key unification — no duplicate API key input */}
                {local.embeddingModel === "openai" && (
                  openaiConnected ? (
                    <p className="text-sm text-text-secondary pl-7 mt-3">
                      Using your connected OpenAI API key.
                    </p>
                  ) : (
                    <p className="text-sm text-text-secondary pl-7 mt-3">
                      No OpenAI provider connected.{" "}
                      <a
                        href="#ai-providers"
                        className="text-accent-primary hover:underline"
                      >
                        Connect OpenAI below →
                      </a>
                    </p>
                  )
                )}
              </div>
            </div>

            {/* Divider */}
            <hr className="border-border-primary" />

            {/* AI Providers section */}
            <AiProvidersSection />

            {/* Entity Extraction section (D-22, D-33) — below AI Providers per UI-SPEC */}
            <ExtractionSettings />
          </div>
        </TabsContent>

        {/* ----- Privacy Tab ----- */}
        <TabsContent value="privacy">
          <div className="card p-6 space-y-8">
            <div>
              <h3 className="section-header text-text-primary mb-4">Privacy Controls</h3>
              <div className="space-y-6">
                <div className="flex items-center justify-between">
                  <div>
                    <Label className="text-text-primary">Privacy Mode</Label>
                    <p className="text-text-tertiary text-xs mt-0.5">
                      When enabled, stricter data handling is applied. No external API calls for
                      embeddings.
                    </p>
                  </div>
                  <Switch checked={false} onCheckedChange={() => {}} />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <Label className="text-text-primary">Telemetry</Label>
                    <p className="text-text-tertiary text-xs mt-0.5">
                      Anonymous usage statistics to help improve Cortex. No document content is ever
                      collected.
                    </p>
                  </div>
                  <Switch checked={false} onCheckedChange={() => {}} />
                </div>

                <div className="rounded-lg bg-bg-tertiary p-4 border border-border-secondary">
                  <div className="flex items-start gap-3">
                    <Shield size={18} className="text-success mt-0.5 flex-shrink-0" />
                    <div>
                      <p className="text-text-primary text-sm font-medium">
                        All processing runs locally
                      </p>
                      <p className="text-text-tertiary text-xs mt-1">
                        Document parsing, embedding generation (with local model), and Smart Space
                        clustering all happen on your device. Your files never leave your machine unless
                        you explicitly choose an API-based embedding model.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </TabsContent>

        {/* ----- Storage Tab ----- */}
        <TabsContent value="storage">
          <div className="card p-6 space-y-8">
            <div>
              <h3 className="section-header text-text-primary mb-4">Storage</h3>
              <div className="space-y-6">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <div className="space-y-1">
                    <Label className="text-text-tertiary text-xs uppercase tracking-wider">
                      Index Size
                    </Label>
                    <p className="text-text-primary text-lg font-semibold">
                      {formatBytes(local.indexSize)}
                    </p>
                  </div>
                  <div className="space-y-1">
                    <Label className="text-text-tertiary text-xs uppercase tracking-wider">
                      Watched Folders
                    </Label>
                    <p className="text-text-primary text-lg font-semibold">
                      {local.watchedFolders.length}
                    </p>
                  </div>
                </div>

                <div className="space-y-2">
                  <Label className="text-text-tertiary text-xs uppercase tracking-wider">
                    Storage Path
                  </Label>
                  <div className="rounded-md bg-bg-tertiary border border-border-primary px-3 py-2">
                    <code className="text-sm text-text-secondary">{local.storagePath}</code>
                  </div>
                </div>

                <div className="pt-2">
                  <button
                    className="btn-secondary flex items-center gap-2 text-error hover:text-error"
                    onClick={() => toast.info("Clear index will be available in a future update.")}
                  >
                    <Trash2 size={16} />
                    Clear Index
                  </button>
                  <p className="text-text-tertiary text-xs mt-2">
                    This will remove all indexed data. Documents will not be deleted.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </TabsContent>

        {/* ----- About Tab ----- */}
        <TabsContent value="about">
          <div className="card p-6 space-y-8">
            <div className="space-y-4">
              <div>
                <h2 className="text-2xl font-bold text-text-primary" style={{ fontFamily: "'Plus Jakarta Sans', sans-serif" }}>
                  Cortex
                </h2>
                <p className="text-text-secondary text-sm mt-1">
                  Self-Organizing Document Intelligence
                </p>
              </div>

              <div className="grid grid-cols-2 gap-4 max-w-sm">
                <div>
                  <Label className="text-text-tertiary text-xs uppercase tracking-wider">
                    Version
                  </Label>
                  <p className="text-text-primary font-medium">v1.0.0</p>
                </div>
                <div>
                  <Label className="text-text-tertiary text-xs uppercase tracking-wider">
                    Framework
                  </Label>
                  <p className="text-text-primary font-medium">Tauri 2</p>
                </div>
              </div>

              <div className="space-y-2">
                <Label className="text-text-tertiary text-xs uppercase tracking-wider">
                  Technologies
                </Label>
                <div className="flex flex-wrap gap-2">
                  {[
                    "React 19",
                    "TypeScript",
                    "Tauri 2",
                    "Rust",
                    "RuVector",
                    "TailwindCSS 4",
                    "ONNX Runtime",
                    "Recharts",
                  ].map((tech) => (
                    <span
                      key={tech}
                      className="px-2.5 py-1 rounded-md bg-bg-tertiary border border-border-primary text-xs text-text-secondary"
                    >
                      {tech}
                    </span>
                  ))}
                </div>
              </div>

              <div className="flex gap-4 pt-2">
                <a
                  href="https://github.com/cortex-app/cortex"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-accent-primary hover:text-accent-hover flex items-center gap-1 transition-colors"
                >
                  <ExternalLink size={14} />
                  GitHub
                </a>
                <a
                  href="https://cortex.app/docs"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-accent-primary hover:text-accent-hover flex items-center gap-1 transition-colors"
                >
                  <ExternalLink size={14} />
                  Documentation
                </a>
              </div>
            </div>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
