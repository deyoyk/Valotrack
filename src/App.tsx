import { lazy, Suspense, useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import {
  Activity,
  Crosshair,
  Download,
  Grid3X3,
  Layers,
  Loader2,
  Map as MapIcon,
  MessageSquare,
  Package,
  Play,
  Radio,
  RefreshCw,
  ScrollText,
  Settings as SettingsIcon,
  ShoppingBag,
  Smartphone,
  Swords,
  Trophy,
  UserRound,
  Users,
  Wallet,
  WifiOff,
  X,
} from "lucide-react";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { api } from "./lib/api";
import { cn } from "./lib/format";
import { useUpdater } from "./lib/updater";
import { displayVersion } from "./lib/version";

const Live = lazy(() => import("./pages/Live"));
const Social = lazy(() => import("./pages/Social"));
const Matchmaking = lazy(() => import("./pages/Matchmaking"));
const Performance = lazy(() => import("./pages/Performance"));
const Encounters = lazy(() => import("./pages/Encounters"));
const Collection = lazy(() => import("./pages/Collection"));
const Crosshairs = lazy(() => import("./pages/Crosshairs"));
const Lineups = lazy(() => import("./pages/Lineups"));
const Callouts = lazy(() => import("./pages/Callouts"));
const Strategy = lazy(() => import("./pages/Strategy"));
const Replays = lazy(() => import("./pages/Replays"));
const Sensitivity = lazy(() => import("./pages/Sensitivity"));
const AsciiStudio = lazy(() => import("./pages/AsciiStudio"));
const Offline = lazy(() => import("./pages/Offline"));
const Remote = lazy(() => import("./pages/Remote"));
const Settings = lazy(() => import("./pages/Settings"));
const Store = lazy(() => import("./pages/Store"));
const Loadout = lazy(() => import("./pages/Loadout"));
const Contracts = lazy(() => import("./pages/Contracts"));
const Leaderboard = lazy(() => import("./pages/Leaderboard"));
const Account = lazy(() => import("./pages/Account"));

type Page =
  | "live"
  | "social"
  | "performance"
  | "encounters"
  | "collection"
  | "matchmaking"
  | "store"
  | "loadout"
  | "contracts"
  | "leaderboard"
  | "account"
  | "crosshairs"
  | "lineups"
  | "callouts"
  | "strategy"
  | "replays"
  | "sensitivity"
  | "ascii"
  | "offline"
  | "remote"
  | "settings";

type NavItem = {
  id: Page;
  label: string;
  icon: typeof Radio;
};

const playNav: NavItem[] = [
  { id: "live", label: "Live", icon: Radio },
  { id: "social", label: "Social", icon: MessageSquare },
  { id: "performance", label: "Performance", icon: Activity },
  { id: "encounters", label: "Encounters", icon: Users },
  { id: "collection", label: "Collection", icon: Wallet },
  { id: "matchmaking", label: "Matchmaking", icon: Swords },
  { id: "offline", label: "Appear Offline", icon: WifiOff },
];

const armoryNav: NavItem[] = [
  { id: "store", label: "Store", icon: ShoppingBag },
  { id: "loadout", label: "Loadout", icon: Package },
  { id: "contracts", label: "Contracts", icon: ScrollText },
];

const intelNav: NavItem[] = [
  { id: "leaderboard", label: "Leaderboard", icon: Trophy },
  { id: "account", label: "Account", icon: UserRound },
];

const toolsNav: NavItem[] = [
  { id: "crosshairs", label: "Crosshairs", icon: Crosshair },
  { id: "lineups", label: "Lineups", icon: Layers },
  { id: "callouts", label: "Callouts", icon: MapIcon },
  { id: "strategy", label: "Strategy", icon: Grid3X3 },
  { id: "replays", label: "Replays", icon: Play },
  { id: "sensitivity", label: "Sensitivity", icon: Trophy },
  { id: "ascii", label: "ASCII Studio", icon: Radio },
];

const systemNav: NavItem[] = [
  { id: "remote", label: "Phone Remote", icon: Smartphone },
  { id: "settings", label: "Settings", icon: SettingsIcon },
];

function readPage(): Page {
  const raw = location.hash.replace("#", "");
  const all = [...playNav, ...armoryNav, ...intelNav, ...toolsNav, ...systemNav];
  return (all.find((item) => item.id === raw)?.id ?? "live") as Page;
}

function NavGroup({
  label,
  items,
  active,
  onSelect,
}: {
  label: string;
  items: NavItem[];
  active: Page;
  onSelect: (page: Page) => void;
}) {
  return (
    <div className="nav-group">
      <span className="nav-label">{label}</span>
      {items.map((item) => {
        const Icon = item.icon;
        return (
          <button
            key={item.id}
            className={cn("nav-item", active === item.id && "active")}
            onClick={() => onSelect(item.id)}
          >
            <Icon size={17} strokeWidth={1.8} />
            <span>{item.label}</span>
          </button>
        );
      })}
    </div>
  );
}

function Page({ page }: { page: Page }) {
  switch (page) {
    case "live":
      return <Live />;
    case "social":
      return <Social />;
    case "performance":
      return <Performance />;
    case "encounters":
      return <Encounters />;
    case "collection":
      return <Collection />;
    case "matchmaking":
      return <Matchmaking />;
    case "store":
      return <Store />;
    case "loadout":
      return <Loadout />;
    case "contracts":
      return <Contracts />;
    case "leaderboard":
      return <Leaderboard />;
    case "account":
      return <Account />;
    case "crosshairs":
      return <Crosshairs />;
    case "lineups":
      return <Lineups />;
    case "callouts":
      return <Callouts />;
    case "strategy":
      return <Strategy />;
    case "replays":
      return <Replays />;
    case "sensitivity":
      return <Sensitivity />;
    case "ascii":
      return <AsciiStudio />;
    case "offline":
      return <Offline />;
    case "remote":
      return <Remote />;
    case "settings":
      return <Settings />;
    default:
      return <Live />;
  }
}

function UpdateBanner() {
  const update = useUpdater();
  const [dismissed, setDismissed] = useState<string | null>(null);
  const visible =
    update.status === "available" ||
    update.status === "downloading" ||
    update.status === "ready";
  if (!visible || dismissed === update.version) return null;
  const downloading = update.status === "downloading";
  const ready = update.status === "ready";
  const line = downloading
    ? `Downloading update${update.progress ? ` · ${update.progress}%` : ""}`
    : ready
      ? "The update is installed. Restart Valotrack to finish."
      : update.notes?.split("\n")[0]?.trim() || "A new version is ready to install.";
  return (
    <div className="update-banner">
      <div className="update-mark">
        <Download size={17} />
      </div>
      <div className="update-copy">
        <strong>
          {ready ? "Update installed" : `Valotrack ${displayVersion(update.version)} is available`}
        </strong>
        <span>{line}</span>
        {downloading && (
          <div className="update-progress">
            <span style={{ width: `${update.progress ?? 0}%` }} />
          </div>
        )}
      </div>
      {ready ? (
        <button className="button primary" onClick={() => void update.restart()}>
          <RefreshCw size={15} /> Restart now
        </button>
      ) : (
        <button
          className="button primary"
          onClick={() => void update.install()}
          disabled={downloading}
        >
          {downloading ? (
            <Loader2 size={15} className="spin" />
          ) : (
            <Download size={15} />
          )}
          {downloading ? "Installing" : "Install update"}
        </button>
      )}
      <button
        className="icon-button"
        onClick={() => setDismissed(update.version ?? "")}
        aria-label="Dismiss update"
      >
        <X size={15} />
      </button>
    </div>
  );
}

export default function App() {
  const [page, setPage] = useState<Page>(readPage);
  const [appVersion, setAppVersion] = useState("");
  const [onlineCount, setOnlineCount] = useState<number | null>(null);

  useEffect(() => {
    void getVersion().then(setAppVersion).catch(() => void 0);
  }, []);

  useEffect(() => {
    let stopped = false;
    const load = async () => {
      if (document.hidden) return;
      try {
        const metrics = await api.concurrentUsers();
        if (!stopped) setOnlineCount(metrics.activeUsers);
      } catch {
        if (!stopped) setOnlineCount(null);
      }
    };
    void load();
    const timer = window.setInterval(() => void load(), 5 * 60 * 1000);
    const onVisibility = () => {
      if (!document.hidden) void load();
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      stopped = true;
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);

  useEffect(() => {
    const onHash = () => setPage(readPage());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  useEffect(() => {
    document.querySelector<HTMLElement>(".main")?.scrollTo({ top: 0 });
  }, [page]);

  function navigate(next: Page) {
    location.hash = next;
    setPage(next);
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <img src="/logo.svg" alt="Valotrack" />
          </div>
          <div>
            <div className="brand-name">
              VALO<span>TRACK</span>
            </div>
            <div className="brand-sub">local intelligence</div>
          </div>
        </div>

        <nav className="nav">
          <NavGroup label="Play" items={playNav} active={page} onSelect={navigate} />
          <NavGroup label="Armory" items={armoryNav} active={page} onSelect={navigate} />
          <NavGroup label="Intel" items={intelNav} active={page} onSelect={navigate} />
          <NavGroup label="Tools" items={toolsNav} active={page} onSelect={navigate} />
          <NavGroup label="System" items={systemNav} active={page} onSelect={navigate} />
        </nav>

        <div className="sidebar-foot">
          <span className="dot" />
          <span className="sidebar-version">{displayVersion(appVersion) || "Valotrack"}</span>
          <span className="sidebar-sep">·</span>
          <span
            className="sidebar-users"
            title="Concurrent users in the last 5 minutes"
          >
            {onlineCount === null ? "- online" : `${onlineCount} online`}
          </span>
        </div>
      </aside>

      <main className="main">
        <UpdateBanner />
        <Suspense
          fallback={
            <div className="page">
              <div className="center-state">
                <Loader2 size={18} className="spin" />
                <span>Loading</span>
              </div>
            </div>
          }
        >
          <ErrorBoundary key={page}>
            <Page page={page} />
          </ErrorBoundary>
        </Suspense>
      </main>
    </div>
  );
}
