import { useEffect, useState } from "react";
import {
  Activity,
  Crosshair,
  Grid3X3,
  Layers,
  Map as MapIcon,
  Play,
  Radio,
  Settings as SettingsIcon,
  Swords,
  Trophy,
  Users,
  Wallet,
} from "lucide-react";
import { cn } from "./lib/format";
import Live from "./pages/Live";
import Matchmaking from "./pages/Matchmaking";
import Settings from "./pages/Settings";

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
  soon?: boolean;
};

const playNav: NavItem[] = [
  { id: "live", label: "Live", icon: Radio },
  { id: "performance", label: "Performance", icon: Activity, soon: true },
  { id: "encounters", label: "Encounters", icon: Users, soon: true },
  { id: "collection", label: "Collection", icon: Wallet, soon: true },
  { id: "matchmaking", label: "Matchmaking", icon: Swords },
];

const toolsNav: NavItem[] = [
  { id: "crosshairs", label: "Crosshairs", icon: Crosshair, soon: true },
  { id: "lineups", label: "Lineups", icon: Layers, soon: true },
  { id: "callouts", label: "Callouts", icon: MapIcon, soon: true },
  { id: "strategy", label: "Strategy", icon: Grid3X3, soon: true },
  { id: "replays", label: "Replays", icon: Play, soon: true },
  { id: "sensitivity", label: "Sensitivity", icon: Trophy, soon: true },
  { id: "ascii", label: "ASCII Studio", icon: Radio, soon: true },
];

const systemNav: NavItem[] = [
  { id: "offline", label: "Appear Offline", icon: Radio, soon: true },
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
            disabled={item.soon}
            title={item.soon ? "Coming online" : item.label}
          >
            <Icon size={17} strokeWidth={1.8} />
            <span>{item.label}</span>
            {item.soon && <span className="nav-soon">soon</span>}
          </button>
        );
      })}
    </div>
  );
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
        {page === "live" && <Live />}
        {page === "matchmaking" && <Matchmaking />}
        {page === "settings" && <Settings />}
      </main>
    </div>
  );
}
