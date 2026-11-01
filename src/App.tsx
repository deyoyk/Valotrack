import { lazy, Suspense, useEffect, useState } from "react";
import {
  Activity,
  Crosshair,
  Grid3X3,
  Layers,
  Loader2,
  Map as MapIcon,
  Play,
  Radio,
  Settings as SettingsIcon,
  Swords,
  Trophy,
  Users,
  Wallet,
  WifiOff,
} from "lucide-react";
import { cn } from "./lib/format";

const Live = lazy(() => import("./pages/Live"));
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
const Settings = lazy(() => import("./pages/Settings"));

type Page =
  | "live"
  | "performance"
  | "encounters"
  | "collection"
  | "matchmaking"
  | "crosshairs"
  | "lineups"
  | "callouts"
  | "strategy"
  | "replays"
  | "sensitivity"
  | "ascii"
  | "offline"
  | "settings";

type NavItem = {
  id: Page;
  label: string;
  icon: typeof Radio;
};

const playNav: NavItem[] = [
  { id: "live", label: "Live", icon: Radio },
  { id: "performance", label: "Performance", icon: Activity },
  { id: "encounters", label: "Encounters", icon: Users },
  { id: "collection", label: "Collection", icon: Wallet },
  { id: "matchmaking", label: "Matchmaking", icon: Swords },
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
  { id: "offline", label: "Appear Offline", icon: WifiOff },
  { id: "settings", label: "Settings", icon: SettingsIcon },
];

function readPage(): Page {
  const raw = location.hash.replace("#", "");
  const all = [...playNav, ...toolsNav, ...systemNav];
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
    case "performance":
      return <Performance />;
    case "encounters":
      return <Encounters />;
    case "collection":
      return <Collection />;
    case "matchmaking":
      return <Matchmaking />;
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
    case "settings":
      return <Settings />;
    default:
      return <Live />;
  }
}

export default function App() {
  const [page, setPage] = useState<Page>(readPage);

  useEffect(() => {
    const onHash = () => setPage(readPage());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  function navigate(next: Page) {
    location.hash = next;
    setPage(next);
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <svg viewBox="0 0 64 64" aria-hidden="true">
              <path d="M8 12 L34 50 L56 50 L56 43 L38 43 L17 12 Z" fill="currentColor" />
              <path
                d="M46 12 L31 31 L37 39 L56 15 L56 12 Z"
                fill="currentColor"
                opacity="0.82"
              />
            </svg>
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
          <NavGroup label="Tools" items={toolsNav} active={page} onSelect={navigate} />
          <NavGroup label="System" items={systemNav} active={page} onSelect={navigate} />
        </nav>

        <div className="sidebar-foot">
          <span className="dot" />
          <span>all data stays local</span>
        </div>
      </aside>

      <main className="main">
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
          <Page page={page} />
        </Suspense>
      </main>
    </div>
  );
}
