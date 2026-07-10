import "./global.css";

import { Toaster } from "@/components/ui/toaster";
import { createRoot } from "react-dom/client";
import { Toaster as Sonner } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { AppShell } from "./components/layout/AppShell";
import Index from "./pages/Index";
import OnboardingPage from "./pages/OnboardingPage";
import SpacesPage from "./pages/SpacesPage";
import SpaceDetailPage from "./pages/SpaceDetailPage";
import SearchPage from "./pages/SearchPage";
import ChatPage from "./pages/ChatPage";
import InsightsPage from "./pages/InsightsPage";
import RecentPage from "./pages/RecentPage";
import FavoritesPage from "./pages/FavoritesPage";
import TagsPage from "./pages/TagsPage";
import WatchedPage from "./pages/WatchedPage";
import SettingsPage from "./pages/SettingsPage";
import DocumentPage from "./pages/DocumentPage";
import EntitiesPage from "./pages/EntitiesPage";
import EntityDetailPage from "./pages/EntityDetailPage";
import EntityDetailPage11 from "./pages/EntityDetailPage11";
import OwnershipPage from "./pages/OwnershipPage";
import NotFound from "./pages/NotFound";
import { ThemeProvider } from "next-themes";

const queryClient = new QueryClient();

const App = () => (
  <ThemeProvider attribute="class" defaultTheme="dark" enableSystem>
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <Toaster />
        <Sonner />
        <BrowserRouter>
          <Routes>
            {/* Onboarding renders fullscreen without AppShell layout (BREAK 6 fix) */}
            <Route path="/onboarding" element={<OnboardingPage />} />
            <Route element={<AppShell />}>
              <Route path="/" element={<Index />} />
              <Route path="/spaces" element={<SpacesPage />} />
              <Route path="/spaces/:id" element={<SpaceDetailPage />} />
              <Route path="/search" element={<SearchPage />} />
              <Route path="/chat" element={<ChatPage />} />
              <Route path="/recent" element={<RecentPage />} />
              <Route path="/favorites" element={<FavoritesPage />} />
              <Route path="/tags" element={<TagsPage />} />
              <Route path="/watched" element={<WatchedPage />} />
              <Route path="/insights" element={<InsightsPage />} />
              <Route path="/settings" element={<SettingsPage />} />
              <Route path="/document/:id" element={<DocumentPage />} />
              <Route path="/entities" element={<EntitiesPage />} />
              <Route path="/entities/:id" element={<EntityDetailPage />} />
              {/* Phase 11: entity detail by class+value (additive — Phase 6 /entities/:id preserved) */}
              <Route path="/entity/:class/:value" element={<EntityDetailPage11 />} />
              {/* Phase 11.5: "all my assets" ownership view grouped by AssetType */}
              <Route path="/ownership/:personId" element={<OwnershipPage />} />
              {/* ADD ALL CUSTOM ROUTES ABOVE THE CATCH-ALL "*" ROUTE */}
            </Route>
            <Route path="*" element={<NotFound />} />
          </Routes>
        </BrowserRouter>
      </TooltipProvider>
    </QueryClientProvider>
  </ThemeProvider>
);

createRoot(document.getElementById("root")!).render(<App />);
