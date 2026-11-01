import { useDeferredValue, useMemo, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Crosshair,
  ExternalLink,
  Heart,
  Layers,
  Play,
  Search,
  Target,
  Users,
} from "lucide-react";
import lineupsData from "../data/lineups.json";
import { cn } from "../lib/format";

type Lineup = {
  id: string;
  stratsId: string;
  title: string;
  map: string;
  agent: string;
  ability: string;
  side: string;
  difficulty: string;
  creator: string;
  posted: string;
  views: number;
  startPin: { x: number; y: number };
  endPins: { x: number; y: number }[];
  video: string;
  poster: string;
  startSeconds: number;
};

const LINEUPS = lineupsData as Lineup[];
const MAPS = Array.from(new Set(LINEUPS.map((lineup) => lineup.map))).sort();
const FAVORITES_KEY = "valotrack-lineup-favorites";

function thumbnail(lineup: Lineup) {
  if (!lineup.poster) return null;
  return `https://cdn.strats.gg/cdn-cgi/image/width=520,quality=72,format=webp/valorant/lineups/images/${lineup.poster}.webp`;
}

function guideUrl(lineup: Lineup) {
  return `https://strats.gg/valorant/lineups/${lineup.stratsId}`;
}

export default function Lineups() {
  const [map, setMap] = useState("Ascent");
  const [side, setSide] = useState<"attack" | "defense">("attack");
  const [agent, setAgent] = useState("All");
  const [ability, setAbility] = useState("All");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Lineup | null>(null);
  const [favorites, setFavorites] = useState<string[]>(() => {
    try {
      const raw = JSON.parse(localStorage.getItem(FAVORITES_KEY) ?? "[]");
      return Array.isArray(raw) ? raw : [];
    } catch {
      return [];
    }
  });
  const deferredSearch = useDeferredValue(search);
  const mapSlug = map.toLowerCase();

  const mapLineups = useMemo(
    () => LINEUPS.filter((lineup) => lineup.map === map && lineup.side === side),
    [map, side],
  );
  const agents = useMemo(
    () => ["All", ...Array.from(new Set(mapLineups.map((lineup) => lineup.agent))).sort()],
    [mapLineups],
  );
  const abilities = useMemo(
    () => [
      "All",
      ...Array.from(
        new Set(
          mapLineups
            .filter((lineup) => agent === "All" || lineup.agent === agent)
            .map((lineup) => lineup.ability),
        ),
      ).sort(),
    ],
    [agent, mapLineups],
  );
  const rows = useMemo(() => {
    const needle = deferredSearch.trim().toLowerCase();
    return mapLineups
      .filter((lineup) => {
        if (agent !== "All" && lineup.agent !== agent) return false;
        if (ability !== "All" && lineup.ability !== ability) return false;
        if (!needle) return true;
        return (
          lineup.title.toLowerCase().includes(needle) ||
          lineup.agent.toLowerCase().includes(needle) ||
          lineup.ability.toLowerCase().includes(needle) ||
          lineup.creator.toLowerCase().includes(needle)
        );
      })
      .sort((a, b) => b.views - a.views);
  }, [ability, agent, deferredSearch, mapLineups]);

  const active = selected ?? rows[0] ?? null;
  const pins = rows.slice(0, 60);

  function toggleFavorite(id: string) {
    setFavorites((current) => {
      const next = current.includes(id)
        ? current.filter((value) => value !== id)
        : [...current, id];
      localStorage.setItem(FAVORITES_KEY, JSON.stringify(next));
      return next;
    });
  }

  return (
    <div className="page lineups-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Layers size={12} /> lineup lab
          </span>
          <h1>One pixel. One kill.</h1>
          <p>
            {LINEUPS.length.toLocaleString()} attributed lineup guides across {MAPS.length}{" "}
            maps. Filter by side, agent and ability, place the stand and landing markers,
            then open the guide.
          </p>
        </div>
        <div className="head-status">
          <span className="checked">{favorites.length} saved lineups</span>
        </div>
      </header>

      <div className="lineup-map-tabs">
        {MAPS.map((name) => (
          <button
            key={name}
            className={map === name ? "active" : ""}
            onClick={() => {
              setMap(name);
              setAgent("All");
              setAbility("All");
              setSelected(null);
            }}
          >
            {name}
            <em>{LINEUPS.filter((lineup) => lineup.map === name).length}</em>
          </button>
        ))}
      </div>

      <div className="lineups-grid">
        <section className="panel lineup-map-panel">
          <div className="lineup-map-head">
            <div className="segmented">
              <button className={side === "attack" ? "active" : ""} onClick={() => setSide("attack")}>
                Attack
              </button>
              <button
                className={side === "defense" ? "active" : ""}
                onClick={() => setSide("defense")}
              >
                Defense
              </button>
            </div>
            <span className="callout-hint">
              <Target size={12} /> {rows.length} lineups · top pins
            </span>
          </div>
          <div className="lineup-map">
            <img src={`/maps/${mapSlug}-${side}.svg`} alt={`${map} ${side}`} draggable={false} />
            {active && (
              <svg className="lineup-lines" viewBox="0 0 100 100" preserveAspectRatio="none">
                {active.endPins.map((pin, index) => (
                  <line
                    key={index}
                    x1={active.startPin.x}
                    y1={active.startPin.y}
                    x2={pin.x}
                    y2={pin.y}
                    stroke="rgba(255,70,85,0.55)"
                    strokeWidth="0.35"
                    strokeDasharray="1.4 1"
                  />
                ))}
              </svg>
            )}
            {pins.map((lineup) => (
              <button
                key={lineup.id}
                className={cn("lineup-pin", active?.id === lineup.id && "active")}
                style={{ left: `${lineup.startPin.x}%`, top: `${lineup.startPin.y}%` }}
                onClick={() => setSelected(lineup)}
                title={`${lineup.agent} · ${lineup.ability} · ${lineup.title}`}
              >
                <span>{lineup.agent.slice(0, 1)}</span>
              </button>
            ))}
            {active &&
              active.endPins.map((pin, index) => (
                <span
                  className="lineup-end"
                  key={index}
                  style={{ left: `${pin.x}%`, top: `${pin.y}%` }}
                />
              ))}
          </div>
          {active && (
            <div className="lineup-active-card">
              <div className="lineup-thumb">
                {thumbnail(active) ? (
                  <img src={thumbnail(active) ?? ""} alt="" loading="lazy" />
                ) : (
                  <Crosshair size={22} />
                )}
                <span>{active.side}</span>
              </div>
              <div className="lineup-active-info">
                <span className="profile-kicker">
                  {active.agent} · {active.ability}
                </span>
                <h3>{active.title}</h3>
                <p>
                  {active.creator} · {active.posted} · {active.views.toLocaleString()} views
                </p>
              </div>
              <div className="lineup-actions">
                <button
                  className={cn("button small", favorites.includes(active.id) && "favorited")}
                  onClick={() => toggleFavorite(active.id)}
                >
                  <Heart size={12} fill={favorites.includes(active.id) ? "currentColor" : "none"} />
                  {favorites.includes(active.id) ? "Saved" : "Save"}
                </button>
                <button className="button small primary" onClick={() => void openUrl(guideUrl(active))}>
                  <Play size={12} /> Watch guide
                </button>
              </div>
            </div>
          )}
        </section>

        <section className="panel lineup-list-panel">
          <div className="lineup-toolbar">
            <div className="search-box">
              <Search size={14} />
              <input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Search lineups"
              />
            </div>
            <select
              className="small-select"
              value={agent}
              onChange={(event) => {
                setAgent(event.target.value);
                setAbility("All");
              }}
            >
              {agents.map((item) => (
                <option value={item} key={item}>
                  {item === "All" ? "Every agent" : item}
                </option>
              ))}
            </select>
            <select
              className="small-select"
              value={ability}
              onChange={(event) => setAbility(event.target.value)}
            >
              {abilities.map((item) => (
                <option value={item} key={item}>
                  {item === "All" ? "Every ability" : item}
                </option>
              ))}
            </select>
          </div>
          <div className="lineup-list">
            {rows.slice(0, 300).map((lineup) => (
              <button
                className={cn("lineup-row", active?.id === lineup.id && "active")}
                key={lineup.id}
                onClick={() => setSelected(lineup)}
              >
                <span className="lineup-row-thumb">
                  {thumbnail(lineup) ? (
                    <img src={thumbnail(lineup) ?? ""} alt="" loading="lazy" />
                  ) : (
                    <Crosshair size={15} />
                  )}
                </span>
                <span className="lineup-row-info">
                  <strong>
                    {lineup.agent} · {lineup.ability}
                  </strong>
                  <em>{lineup.title}</em>
                  <small>
                    <Users size={9} /> {lineup.creator} · {lineup.views.toLocaleString()} views
                  </small>
                </span>
                <ExternalLink size={12} className="lineup-row-link" />
              </button>
            ))}
            {!rows.length && (
              <div className="chart-empty compact">
                <span>No lineups match these filters.</span>
              </div>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
