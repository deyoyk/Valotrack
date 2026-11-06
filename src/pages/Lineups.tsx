import { useDeferredValue, useEffect, useMemo, useState } from "react";
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
  X,
} from "lucide-react";
import lineupsData from "../data/lineups.json";
import lineupMeta from "../data/lineup-meta.json";
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

type AgentKit = { name: string; icon: string; key: string };
type AgentMeta = { name: string; slug: string; icon: string; role: string; kit: AgentKit[] };
type MapMeta = { name: string; slug: string; count: number };
type Meta = { Kn: MapMeta[]; X6: AgentMeta[] };

type Spot = { key: string; x: number; y: number; count: number };

const LINEUPS = lineupsData as Lineup[];
const META = lineupMeta as unknown as Meta;
const FAVORITES_KEY = "valotrack-lineup-favorites";
const FEED_PAGE = 30;
const MAX_SPOTS = 48;
const BOOT_TIME = Date.now();
const MEDIA_BASE = "https://cdn.strats.gg";

const AGENT_META = new Map(META.X6.map((agent) => [agent.name.toLowerCase(), agent]));
const MAP_SLUGS = new Map(META.Kn.map((map) => [map.name, map.slug]));
const MAP_COUNTS = new Map<string, number>();
for (const lineup of LINEUPS) {
  MAP_COUNTS.set(lineup.map, (MAP_COUNTS.get(lineup.map) ?? 0) + 1);
}
const MAPS = [...MAP_COUNTS.entries()].sort((a, b) => b[1] - a[1]).map(([name]) => name);
const POSTED_AT = new Map(
  LINEUPS.map((lineup) => [lineup.id, Date.parse(`${lineup.posted}T00:00:00Z`)]),
);
const SCORE = new Map(
  LINEUPS.map((lineup) => {
    const posted = POSTED_AT.get(lineup.id) ?? 0;
    const ageDays = Math.max(0, (BOOT_TIME - posted) / 86_400_000);
    return [lineup.id, lineup.views / (1 + ageDays / 365)];
  }),
);

function normalise(value: string) {
  return value.replace(/\s+/g, " ").trim().toLowerCase();
}

function agentMeta(agent: string) {
  return AGENT_META.get(agent.toLowerCase()) ?? null;
}

function abilityIcon(agent: string, ability: string) {
  const meta = agentMeta(agent);
  if (!meta) return null;
  const target = normalise(ability);
  const kit = meta.kit.find((item) => normalise(item.name) === target);
  return kit?.icon ?? null;
}

function mapArt(map: string, side: string) {
  const slug = MAP_SLUGS.get(map) ?? map.toLowerCase();
  return `/maps/${slug}-${side}.svg`;
}

function posterUrl(lineup: Lineup) {
  return lineup.poster ? `${MEDIA_BASE}/valorant/lineups/images/${lineup.poster}.webp` : "";
}

function videoUrl(lineup: Lineup) {
  return `${MEDIA_BASE}/valorant/lineups/videos/${lineup.video}.mp4`;
}

function guideUrl(lineup: Lineup) {
  return `https://strats.gg/valorant/lineups/${lineup.stratsId}`;
}

function viewCount(value: number) {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}m`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(value >= 10_000 ? 0 : 1)}k`;
  return String(value);
}

function postedLabel(value: string) {
  const date = new Date(`${value}T00:00:00Z`);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function spotKeyOf(lineup: Lineup) {
  const pin = lineup.endPins[0];
  return pin ? `${Math.round(pin.x / 2)}:${Math.round(pin.y / 2)}` : "";
}

function buildSpots(lineups: Lineup[]): Spot[] {
  const buckets = new Map<string, { x: number; y: number; count: number }>();
  for (const lineup of lineups) {
    const pin = lineup.endPins[0];
    if (!pin) continue;
    const key = spotKeyOf(lineup);
    const bucket = buckets.get(key);
    if (bucket) {
      bucket.x += pin.x;
      bucket.y += pin.y;
      bucket.count += 1;
    } else {
      buckets.set(key, { x: pin.x, y: pin.y, count: 1 });
    }
  }
  return [...buckets.entries()]
    .map(([key, bucket]) => ({
      key,
      x: bucket.x / bucket.count,
      y: bucket.y / bucket.count,
      count: bucket.count,
    }))
    .sort((a, b) => b.count - a.count);
}

export default function Lineups() {
  const [map, setMap] = useState(MAPS[0]);
  const [side, setSide] = useState<"attack" | "defense">("attack");
  const [agent, setAgent] = useState("All");
  const [ability, setAbility] = useState("All");
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<"recommended" | "newest" | "views">("recommended");
  const [playbookOnly, setPlaybookOnly] = useState(false);
  const [spotKey, setSpotKey] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [visible, setVisible] = useState(FEED_PAGE);
  const [favorites, setFavorites] = useState<string[]>(() => {
    try {
      const raw = JSON.parse(localStorage.getItem(FAVORITES_KEY) ?? "[]");
      return Array.isArray(raw) ? raw : [];
    } catch {
      return [];
    }
  });
  const deferredSearch = useDeferredValue(search);

  const mapLineups = useMemo(
    () => LINEUPS.filter((lineup) => lineup.map === map && lineup.side === side),
    [map, side],
  );

  const agentRail = useMemo(() => {
    const counts = new Map<string, number>();
    for (const lineup of mapLineups) {
      counts.set(lineup.agent, (counts.get(lineup.agent) ?? 0) + 1);
    }
    return [...counts.entries()]
      .map(([name, count]) => ({ name, count, icon: agentMeta(name)?.icon ?? "" }))
      .sort((a, b) => b.count - a.count);
  }, [mapLineups]);

  const abilities = useMemo(() => {
    const names = new Set<string>();
    for (const lineup of mapLineups) {
      if (agent !== "All" && lineup.agent !== agent) continue;
      names.add(lineup.ability);
    }
    return ["All", ...[...names].sort()];
  }, [agent, mapLineups]);

  const pool = useMemo(() => {
    const needle = deferredSearch.trim().toLowerCase();
    return mapLineups.filter((lineup) => {
      if (playbookOnly && !favorites.includes(lineup.id)) return false;
      if (agent !== "All" && lineup.agent !== agent) return false;
      if (ability !== "All" && lineup.ability !== ability) return false;
      if (!needle) return true;
      return (
        lineup.title.toLowerCase().includes(needle) ||
        lineup.agent.toLowerCase().includes(needle) ||
        lineup.ability.toLowerCase().includes(needle) ||
        lineup.creator.toLowerCase().includes(needle)
      );
    });
  }, [ability, agent, deferredSearch, favorites, mapLineups, playbookOnly]);

  const spots = useMemo(() => buildSpots(pool), [pool]);

  const rows = useMemo(() => {
    const scoped = spotKey ? pool.filter((lineup) => spotKeyOf(lineup) === spotKey) : pool;
    return [...scoped].sort((a, b) => {
      if (sort === "newest") return (POSTED_AT.get(b.id) ?? 0) - (POSTED_AT.get(a.id) ?? 0);
      if (sort === "views") return b.views - a.views;
      return (SCORE.get(b.id) ?? 0) - (SCORE.get(a.id) ?? 0);
    });
  }, [pool, sort, spotKey]);

  const active = useMemo(() => {
    if (selectedId) {
      const hit = rows.find((lineup) => lineup.id === selectedId);
      if (hit) return hit;
    }
    return rows[0] ?? null;
  }, [rows, selectedId]);

  const activeAgent = active ? agentMeta(active.agent) : null;
  const activeAbilityIcon = active ? abilityIcon(active.agent, active.ability) : null;

  useEffect(() => {
    setVisible(FEED_PAGE);
  }, [map, side, agent, ability, deferredSearch, sort, playbookOnly, spotKey]);

  function changeMap(next: string) {
    setMap(next);
    setAgent("All");
    setAbility("All");
    setSpotKey(null);
    setSelectedId(null);
  }

  function changeSide(next: "attack" | "defense") {
    setSide(next);
    setSpotKey(null);
    setSelectedId(null);
  }

  function changeAgent(next: string) {
    setAgent(next);
    setAbility("All");
    setSpotKey(null);
    setSelectedId(null);
  }

  function toggleSpot(next: string) {
    setSpotKey((current) => (current === next ? null : next));
    setSelectedId(null);
  }

  function toggleFavorite(id: string) {
    setFavorites((current) => {
      const next = current.includes(id)
        ? current.filter((value) => value !== id)
        : [...current, id];
      localStorage.setItem(FAVORITES_KEY, JSON.stringify(next));
      return next;
    });
  }

  const pinnedSpots = spots.slice(0, MAX_SPOTS);
  const activeSpot = active
    ? pinnedSpots.find((spot) => spot.key === spotKeyOf(active))
    : undefined;

  return (
    <div className="page lineups-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Layers size={12} /> lineup lab
          </span>
          <h1>One pixel. One kill.</h1>
          <p>
            {LINEUPS.length.toLocaleString()} attributed lineup guides across {MAPS.length} maps.
            Pick a landing spot, watch the clip without leaving the app, and keep the ones you
            trust in your playbook.
          </p>
        </div>
        <div className="head-status">
          <span className="status-pill">
            <Play size={11} /> {pool.length.toLocaleString()} in view
          </span>
          <span className="checked">
            <Heart size={11} /> {favorites.length} saved lineups
          </span>
        </div>
      </header>

      <div className="lineup-map-tabs">
        {MAPS.map((name) => (
          <button key={name} className={map === name ? "active" : ""} onClick={() => changeMap(name)}>
            {name}
            <em>{(MAP_COUNTS.get(name) ?? 0).toLocaleString()}</em>
          </button>
        ))}
      </div>

      <div className="lineup-filters">
        <div className="lineup-agent-rail">
          <button
            className={cn("lineup-agent lineup-agent-all", agent === "All" && "active")}
            onClick={() => changeAgent("All")}
            title="Every agent"
          >
            <span>All</span>
            <em>{mapLineups.length}</em>
          </button>
          {agentRail.map((item) => (
            <button
              key={item.name}
              className={cn("lineup-agent", agent === item.name && "active")}
              onClick={() => changeAgent(item.name)}
              title={`${item.name} · ${item.count} lineups`}
            >
              {item.icon ? <img src={item.icon} alt={item.name} loading="lazy" /> : <span>{item.name}</span>}
              <em>{item.count}</em>
            </button>
          ))}
        </div>

        <div className="lineup-toolbar">
          <div className="segmented">
            <button className={side === "attack" ? "active" : ""} onClick={() => changeSide("attack")}>
              Attack
            </button>
            <button className={side === "defense" ? "active" : ""} onClick={() => changeSide("defense")}>
              Defense
            </button>
          </div>
          <div className="search-box">
            <Search size={14} />
            <input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search lineups, agents, creators"
            />
          </div>
          <select
            className="small-select"
            value={ability}
            onChange={(event) => setAbility(event.target.value)}
            aria-label="Filter by ability"
          >
            {abilities.map((item) => (
              <option value={item} key={item}>
                {item === "All" ? "Every ability" : item}
              </option>
            ))}
          </select>
          <button
            className={cn("button small", playbookOnly && "active")}
            onClick={() => setPlaybookOnly((value) => !value)}
          >
            <Heart size={12} fill={playbookOnly ? "currentColor" : "none"} /> Playbook
          </button>
          <select
            className="small-select"
            value={sort}
            onChange={(event) => setSort(event.target.value as "recommended" | "newest" | "views")}
            aria-label="Sort lineups"
          >
            <option value="recommended">Recommended</option>
            <option value="newest">Newest</option>
            <option value="views">Most viewed</option>
          </select>
        </div>
      </div>

      <div className="lineups-grid">
        <section className="panel lineup-board">
          <div className="lineup-map-head">
            <div>
              <span className="panel-label">Stand &amp; landing</span>
              <h2>{map}</h2>
            </div>
            <div className="lineup-legend">
              <span>
                <i className="stand" /> Stand
              </span>
              <span>
                <i className="land" /> Landing
              </span>
            </div>
          </div>

          <div className="lineup-map">
            <img src={mapArt(map, side)} alt={`${map} ${side} map`} draggable={false} />

            {active && (
              <svg className="lineup-lines" viewBox="0 0 100 100" preserveAspectRatio="none">
                {active.endPins.map((pin, index) => (
                  <line
                    key={index}
                    x1={active.startPin.x}
                    y1={active.startPin.y}
                    x2={pin.x}
                    y2={pin.y}
                    stroke="rgba(255,70,85,0.45)"
                    strokeWidth="0.28"
                    strokeDasharray="1.1 0.85"
                  />
                ))}
              </svg>
            )}

            {pinnedSpots.map((spot) => (
              <button
                key={spot.key}
                className={cn("lineup-spot", spot.count > 9 && "big", spotKey === spot.key && "active")}
                style={{ left: `${spot.x}%`, top: `${spot.y}%` }}
                onClick={() => toggleSpot(spot.key)}
                title={`${spot.count} lineup${spot.count === 1 ? "" : "s"} land here`}
              >
                {spot.count}
              </button>
            ))}

            {active && (
              <span
                className="lineup-stand"
                style={{ left: `${active.startPin.x}%`, top: `${active.startPin.y}%` }}
                title="Where you stand"
              >
                <Crosshair size={14} />
              </span>
            )}

            {active?.endPins.map((pin, index) => (
              <span key={index} className="lineup-land" style={{ left: `${pin.x}%`, top: `${pin.y}%` }} />
            ))}

            {!rows.length && <span className="lineup-map-empty">No lineups for this filter</span>}
          </div>

          <div className="lineup-map-foot">
            <span>
              {rows.length.toLocaleString()} lineup{rows.length === 1 ? "" : "s"} ·{" "}
              {side === "attack" ? "attacking" : "defending"} · {spots.length} landing spots
            </span>
            {spotKey && (
              <button className="lineup-spot-clear" onClick={() => toggleSpot(spotKey)}>
                <X size={11} /> Clear landing spot
              </button>
            )}
          </div>
        </section>

        <section className="panel lineup-side">
          {active ? (
            <>
              <div className="lineup-stage">
                <video
                  key={active.id}
                  src={videoUrl(active)}
                  poster={posterUrl(active) || undefined}
                  controls
                  playsInline
                  preload="metadata"
                />
                <span className="lineup-stage-tag">
                  {active.map} · {active.side}
                </span>
              </div>

              <div className="lineup-detail-body">
                <span className="lineup-detail-kicker">
                  {activeAgent?.icon && <img src={activeAgent.icon} alt="" />}
                  <span>
                    {active.agent} · {active.ability}
                  </span>
                  {activeAbilityIcon && <img src={activeAbilityIcon} alt="" />}
                </span>
                <h2>{active.title}</h2>

                <div className="lineup-chips">
                  {activeAgent?.role && <span className="lineup-chip">{activeAgent.role}</span>}
                  <span className="lineup-chip">
                    <Users size={11} /> {viewCount(active.views)} views
                  </span>
                  <span className="lineup-chip">
                    <Target size={11} /> {active.endPins.length} landing
                  </span>
                  {activeSpot && <span className="lineup-chip">Spot {activeSpot.count}</span>}
                </div>

                <p className="lineup-byline">
                  {active.creator} · {postedLabel(active.posted)}
                </p>

                <div className="lineup-actions">
                  <button
                    className={cn("button small", favorites.includes(active.id) && "favorited")}
                    onClick={() => toggleFavorite(active.id)}
                  >
                    <Heart size={12} fill={favorites.includes(active.id) ? "currentColor" : "none"} />
                    {favorites.includes(active.id) ? "Saved" : "Save"}
                  </button>
                  <button className="button small primary" onClick={() => void openUrl(guideUrl(active))}>
                    <ExternalLink size={12} /> Open on Strats.gg
                  </button>
                </div>
              </div>
            </>
          ) : (
            <div className="lineup-detail-empty">
              <Crosshair size={22} />
              <strong>No lineup selected</strong>
              <span>Clear the filters or pick another landing spot.</span>
            </div>
          )}

          <div className="lineup-list-head">
            <span className="panel-label">{spotKey ? "Landing here" : "Lineups"}</span>
            <span>{rows.length.toLocaleString()} guides</span>
          </div>

          <div className="lineup-list">
            {rows.slice(0, visible).map((lineup) => {
              const icon = agentMeta(lineup.agent)?.icon ?? "";
              const said = abilityIcon(lineup.agent, lineup.ability);
              return (
                <button
                  key={lineup.id}
                  className={cn("lineup-row", active?.id === lineup.id && "active")}
                  onClick={() => setSelectedId(lineup.id)}
                >
                  <span className="lineup-row-icon">
                    {icon && <img src={icon} alt="" loading="lazy" />}
                  </span>
                  <span className="lineup-row-body">
                    <strong>{lineup.title}</strong>
                    <small>
                      {lineup.agent} · {lineup.ability}
                    </small>
                  </span>
                  <span className="lineup-row-side">
                    {said && <img src={said} alt="" loading="lazy" />}
                    <em>{viewCount(lineup.views)}</em>
                  </span>
                </button>
              );
            })}

            {!rows.length && (
              <div className="lineup-list-empty">
                <span>No lineups match these filters.</span>
              </div>
            )}
          </div>

          {rows.length > visible && (
            <button className="button small lineup-more" onClick={() => setVisible((value) => value + FEED_PAGE)}>
              Show {Math.min(FEED_PAGE, rows.length - visible)} more
            </button>
          )}
        </section>
      </div>
    </div>
  );
}
