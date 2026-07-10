/**
 * CommandPalette - Cmd+K overlay for search and navigation.
 *
 * Uses cmdk library for keyboard-driven command list.
 * Groups: Navigation, Spaces, Search Results, Actions.
 */

import { useEffect, useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { Command } from "cmdk";
import {
  Home,
  Brain,
  Search,
  Clock,
  Star,
  Tag,
  Folder,
  BarChart3,
  Settings,
  Moon,
  Sun,
  PanelLeftClose,
  RefreshCw,
  FileText,
} from "lucide-react";
import { useTheme } from "next-themes";
import { cn } from "@/lib/utils";
import { useCommandPaletteStore, useSidebarStore, useOnboardingStore } from "@/lib/stores";
import {
  useSpaces,
  useDocumentSearch,
  useReclusterSpaces,
} from "@/hooks/useTauri";
import { resolveIcon } from "@/lib/icons";

// Custom debounce hook
function useDebouncedValue<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);
  return debounced;
}

export function CommandPalette() {
  const [inputValue, setInputValue] = useState("");
  const debouncedQuery = useDebouncedValue(inputValue, 150);
  const navigate = useNavigate();
  const { theme, setTheme } = useTheme();
  const { isOpen, close } = useCommandPaletteStore();
  const { toggle: toggleSidebar } = useSidebarStore();
  const { data: spaces } = useSpaces();
  const recluster = useReclusterSpaces();

  // Only search when 3+ chars typed
  const searchQuery = debouncedQuery.length >= 3 ? debouncedQuery : "";
  const { data: searchResults } = useDocumentSearch(searchQuery);

  // Navigate and close
  const go = useCallback(
    (path: string) => {
      navigate(path);
      close();
      setInputValue("");
    },
    [navigate, close],
  );

  // Execute action and close
  const runAction = useCallback(
    (action: () => void) => {
      action();
      close();
      setInputValue("");
    },
    [close],
  );

  // Global Cmd+K listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        useCommandPaletteStore.getState().toggle();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  if (!isOpen) return null;

  const navItems = [
    { path: "/", label: "Dashboard", icon: Home, shortcut: "1" },
    { path: "/spaces", label: "Smart Spaces", icon: Brain, shortcut: "2" },
    { path: "/search", label: "Search", icon: Search, shortcut: "3" },
    { path: "/recent", label: "Recent", icon: Clock },
    { path: "/favorites", label: "Favorites", icon: Star },
    { path: "/tags", label: "Tags", icon: Tag },
    { path: "/watched", label: "Watched Folders", icon: Folder },
    { path: "/insights", label: "Insights", icon: BarChart3 },
    { path: "/settings", label: "Settings", icon: Settings, shortcut: "," },
  ];

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-[100] bg-black/50 backdrop-blur-sm"
        onClick={close}
      />

      {/* Command dialog */}
      <div className="fixed inset-0 z-[101] flex items-start justify-center pt-[20vh]">
        <Command
          className="w-full max-w-lg rounded-xl border border-border-primary bg-bg-primary shadow-2xl overflow-hidden"
          shouldFilter={true}
          loop={true}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              close();
              setInputValue("");
            }
          }}
        >
          <Command.Input
            value={inputValue}
            onValueChange={setInputValue}
            placeholder="Type a command or search..."
            className="w-full border-b border-border-primary bg-transparent px-4 py-3 text-sm text-text-primary placeholder:text-text-tertiary outline-none"
          />

          <Command.List className="max-h-80 overflow-y-auto p-2">
            <Command.Empty className="py-6 text-center text-sm text-text-tertiary">
              No results found.
            </Command.Empty>

            {/* Navigation Group */}
            <Command.Group
              heading="Navigation"
              className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-text-tertiary"
            >
              {navItems.map((item) => (
                <Command.Item
                  key={item.path}
                  value={`navigate ${item.label}`}
                  onSelect={() => go(item.path)}
                  className="flex items-center gap-3 rounded-md px-2 py-2 text-sm text-text-secondary cursor-pointer data-[selected=true]:bg-bg-tertiary data-[selected=true]:text-text-primary"
                >
                  <item.icon size={16} />
                  <span className="flex-1">{item.label}</span>
                  {item.shortcut && (
                    <kbd className="rounded bg-bg-secondary px-1.5 py-0.5 text-[10px] font-mono text-text-tertiary">
                      Cmd+{item.shortcut}
                    </kbd>
                  )}
                </Command.Item>
              ))}
            </Command.Group>

            {/* Spaces Group */}
            {spaces && spaces.length > 0 && (
              <Command.Group
                heading="Spaces"
                className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-text-tertiary"
              >
                {spaces.slice(0, 8).map((space) => {
                  const SpaceIcon = resolveIcon(space.icon);
                  return (
                    <Command.Item
                      key={space.id}
                      value={`space ${space.name}`}
                      onSelect={() => go(`/spaces/${space.id}`)}
                      className="flex items-center gap-3 rounded-md px-2 py-2 text-sm text-text-secondary cursor-pointer data-[selected=true]:bg-bg-tertiary data-[selected=true]:text-text-primary"
                    >
                      <SpaceIcon size={16} style={{ color: space.color }} />
                      <span className="flex-1">{space.name}</span>
                      <span className="text-xs text-text-tertiary">
                        {space.documentCount} docs
                      </span>
                    </Command.Item>
                  );
                })}
              </Command.Group>
            )}

            {/* Search Results Group */}
            {searchResults && searchResults.length > 0 && (
              <Command.Group
                heading="Documents"
                className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-text-tertiary"
              >
                {searchResults.slice(0, 5).map((result) => (
                  <Command.Item
                    key={result.document.id}
                    value={`document ${result.document.name}`}
                    onSelect={() => go(`/document/${result.document.id}`)}
                    className="flex items-center gap-3 rounded-md px-2 py-2 text-sm text-text-secondary cursor-pointer data-[selected=true]:bg-bg-tertiary data-[selected=true]:text-text-primary"
                  >
                    <FileText size={16} />
                    <div className="flex-1 min-w-0">
                      <p className="truncate">{result.document.name}</p>
                      {result.matchedExcerpt && (
                        <p className="text-xs text-text-tertiary truncate">
                          {result.matchedExcerpt}
                        </p>
                      )}
                    </div>
                    <span className="text-xs text-text-tertiary">
                      {Math.round(result.score * 100)}%
                    </span>
                  </Command.Item>
                ))}
              </Command.Group>
            )}

            {/* Actions Group */}
            <Command.Group
              heading="Actions"
              className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-text-tertiary"
            >
              <Command.Item
                value="toggle theme dark light"
                onSelect={() =>
                  runAction(() =>
                    setTheme(theme === "dark" ? "light" : "dark"),
                  )
                }
                className="flex items-center gap-3 rounded-md px-2 py-2 text-sm text-text-secondary cursor-pointer data-[selected=true]:bg-bg-tertiary data-[selected=true]:text-text-primary"
              >
                {theme === "dark" ? <Sun size={16} /> : <Moon size={16} />}
                <span className="flex-1">
                  Switch to {theme === "dark" ? "Light" : "Dark"} Mode
                </span>
                <kbd className="rounded bg-bg-secondary px-1.5 py-0.5 text-[10px] font-mono text-text-tertiary">
                  Cmd+D
                </kbd>
              </Command.Item>
              <Command.Item
                value="toggle sidebar"
                onSelect={() => runAction(toggleSidebar)}
                className="flex items-center gap-3 rounded-md px-2 py-2 text-sm text-text-secondary cursor-pointer data-[selected=true]:bg-bg-tertiary data-[selected=true]:text-text-primary"
              >
                <PanelLeftClose size={16} />
                <span className="flex-1">Toggle Sidebar</span>
                <kbd className="rounded bg-bg-secondary px-1.5 py-0.5 text-[10px] font-mono text-text-tertiary">
                  Cmd+\
                </kbd>
              </Command.Item>
              <Command.Item
                value="restart onboarding setup"
                onSelect={() =>
                  runAction(() => {
                    useOnboardingStore.getState().reset();
                    navigate("/onboarding");
                  })
                }
                className="flex items-center gap-3 rounded-md px-2 py-2 text-sm text-text-secondary cursor-pointer data-[selected=true]:bg-bg-tertiary data-[selected=true]:text-text-primary"
              >
                <RefreshCw size={16} />
                <span className="flex-1">Restart Onboarding</span>
              </Command.Item>
              <Command.Item
                value="recluster spaces"
                onSelect={() => runAction(() => recluster.mutate())}
                className="flex items-center gap-3 rounded-md px-2 py-2 text-sm text-text-secondary cursor-pointer data-[selected=true]:bg-bg-tertiary data-[selected=true]:text-text-primary"
              >
                <RefreshCw size={16} />
                <span className="flex-1">Re-cluster Spaces</span>
              </Command.Item>
            </Command.Group>
          </Command.List>
        </Command>
      </div>
    </>
  );
}
