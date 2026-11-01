import { useMemo, useState } from "react";
import { Crosshair, Gauge, MousePointer2, Search, Trophy } from "lucide-react";
import sensitivityData from "../data/sensitivity.json";
import { cn } from "../lib/format";

type Pro = {
  team: string;
  player: string;
  country: string;
  dpi: number;
  sensitivity: number;
  edpi: number;
  scopedSensitivity: number;
};

const PROS = sensitivityData as Pro[];

const GAMES = [
  { id: "valorant", name: "VALORANT", yaw: 0.07 },
  { id: "cs2", name: "CS2 / CS:GO", yaw: 0.022 },
  { id: "apex", name: "Apex Legends", yaw: 0.022 },
  { id: "overwatch", name: "Overwatch 2", yaw: 0.0066 },
  { id: "r6", name: "Rainbow Six Siege", yaw: 0.00573 },
  { id: "cod", name: "Call of Duty", yaw: 0.0066 },
  { id: "fortnite", name: "Fortnite", yaw: 0.00555 },
  { id: "destiny", name: "Destiny 2", yaw: 0.0066 },
];

function cm360(sensitivity: number, dpi: number, yaw: number) {
  if (sensitivity <= 0 || dpi <= 0) return 0;
  return (360 / (sensitivity * dpi * yaw)) * 2.54;
}

export default function Sensitivity() {
  const [dpi, setDpi] = useState(800);
  const [sensitivity, setSensitivity] = useState(0.4);
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<"edpi" | "name" | "team">("edpi");
  const [converter, setConverter] = useState("cs2");
  const [sourceSens, setSourceSens] = useState(1.0);

  const edpi = Math.round(sensitivity * dpi);
  const median = useMemo(() => {
    const values = PROS.map((pro) => pro.edpi).sort((a, b) => a - b);
    return values[Math.floor(values.length / 2)];
  }, []);
  const percentile = useMemo(
    () => Math.round((PROS.filter((pro) => pro.edpi <= edpi).length / PROS.length) * 100),
    [edpi],
  );
  const closest = useMemo(
    () =>
      [...PROS]
        .sort((a, b) => Math.abs(a.edpi - edpi) - Math.abs(b.edpi - edpi))
        .slice(0, 5),
    [edpi],
  );
  const rows = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return PROS.filter(
      (pro) =>
        !needle ||
        pro.player.toLowerCase().includes(needle) ||
        pro.team.toLowerCase().includes(needle) ||
        pro.country.toLowerCase().includes(needle),
    ).sort((a, b) => {
      if (sort === "name") return a.player.localeCompare(b.player);
      if (sort === "team") return a.team.localeCompare(b.team);
      return a.edpi - b.edpi;
    });
  }, [search, sort]);

  const buckets = useMemo(() => {
    const ranges = [
      [0, 100],
      [100, 150],
      [150, 200],
      [200, 250],
      [250, 300],
      [300, 400],
      [400, 500],
      [500, 700],
      [700, 10000],
    ];
    return ranges.map(([min, max]) => ({
      label: max === 10000 ? `${min}+` : `${min}–${max}`,
      count: PROS.filter((pro) => pro.edpi >= min && pro.edpi < max).length,
      active: edpi >= min && edpi < max,
    }));
  }, [edpi]);
  const maxBucket = Math.max(...buckets.map((bucket) => bucket.count), 1);

  const game = GAMES.find((item) => item.id === converter) ?? GAMES[1];
  const converted = useMemo(() => {
    const degrees = sourceSens * game.yaw;
    return degrees / 0.07;
  }, [game, sourceSens]);
  const physical = cm360(sensitivity, dpi, 0.07);

  return (
    <div className="page sensitivity-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <MousePointer2 size={12} /> sensitivity lab
          </span>
          <h1>Dial it in.</h1>
          <p>
            {PROS.length} pro settings, eDPI percentile, closest-pro matcher and
            physical distance converted across eight shooters.
          </p>
        </div>
        <div className="head-status">
          <span className="checked">median pro eDPI {median}</span>
        </div>
      </header>

      <div className="sens-hero-grid">
        <section className="panel sens-input-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Your settings</span>
              <h2>eDPI matcher</h2>
            </div>
            <Gauge size={20} className="panel-icon" />
          </div>
          <div className="sens-inputs">
            <label>
              <span>DPI</span>
              <input
                type="number"
                min={100}
                max={32000}
                step={50}
                value={dpi}
                onChange={(event) => setDpi(Number(event.target.value))}
              />
            </label>
            <label>
              <span>In-game sensitivity</span>
              <input
                type="number"
                min={0.01}
                max={10}
                step={0.01}
                value={sensitivity}
                onChange={(event) => setSensitivity(Number(event.target.value))}
              />
            </label>
          </div>
          <div className="edpi-readout">
            <div>
              <span>Your eDPI</span>
              <strong>{edpi.toLocaleString()}</strong>
            </div>
            <div>
              <span>Percentile</span>
              <strong>{percentile}%</strong>
            </div>
            <div>
              <span>cm / 360°</span>
              <strong>{physical > 0 ? physical.toFixed(1) : "—"}</strong>
            </div>
          </div>
          <div className="distribution">
            <span className="panel-label">Where you land</span>
            {buckets.map((bucket) => (
              <div className={cn("dist-row", bucket.active && "active")} key={bucket.label}>
                <span>{bucket.label}</span>
                <div className="dist-bar">
                  <i style={{ width: `${(bucket.count / maxBucket) * 100}%` }} />
                </div>
                <em>{bucket.count}</em>
              </div>
            ))}
          </div>
        </section>

        <section className="panel sens-convert-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Converter</span>
              <h2>From another game</h2>
            </div>
            <Crosshair size={20} className="panel-icon" />
          </div>
          <label className="field">
            <span>Game</span>
            <select value={converter} onChange={(event) => setConverter(event.target.value)}>
              {GAMES.filter((item) => item.id !== "valorant").map((item) => (
                <option value={item.id} key={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>{game.name} sensitivity</span>
            <input
              type="number"
              min={0.01}
              step={0.01}
              value={sourceSens}
              onChange={(event) => setSourceSens(Number(event.target.value))}
            />
          </label>
          <div className="conversion-result">
            <span>VALORANT sensitivity</span>
            <strong>{converted.toFixed(3)}</strong>
            <em>at {dpi} DPI · {cm360(sourceSens, dpi, game.yaw).toFixed(1)} cm/360</em>
          </div>
          <button
            className="button primary"
            onClick={() => setSensitivity(Number(converted.toFixed(4)))}
          >
            Use this sensitivity
          </button>
          <p className="quiet">
            Conversions use each game's published yaw rate and are rounded for practical
            use. Always confirm in the range.
          </p>
        </section>
      </div>

      <div className="sens-columns">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Closest</span>
              <h2>Pros near your eDPI</h2>
            </div>
            <Trophy size={19} className="panel-icon" />
          </div>
          <div className="closest-list">
            {closest.map((pro) => (
              <div className="closest-row" key={`${pro.player}-${pro.team}`}>
                <span className="closest-avatar">{pro.player.slice(0, 2).toUpperCase()}</span>
                <div>
                  <strong>{pro.player}</strong>
                  <span>
                    {pro.team} · {pro.country}
                  </span>
                </div>
                <div className="closest-values">
                  <strong>{pro.edpi} eDPI</strong>
                  <span>
                    {pro.dpi} DPI · {pro.sensitivity}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Database</span>
              <h2>All pro settings</h2>
            </div>
          </div>
          <div className="sens-toolbar">
            <div className="search-box">
              <Search size={14} />
              <input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Search player, team or country"
              />
            </div>
            <select
              className="small-select"
              value={sort}
              onChange={(event) => setSort(event.target.value as typeof sort)}
            >
              <option value="edpi">eDPI</option>
              <option value="name">Player</option>
              <option value="team">Team</option>
            </select>
          </div>
          <div className="pro-table">
            <div className="pro-columns">
              <span>Player</span>
              <span>Team</span>
              <span>DPI</span>
              <span>Sens</span>
              <span>eDPI</span>
              <span>Scoped</span>
            </div>
            <div className="pro-list">
              {rows.map((pro) => (
                <div className="pro-row" key={`${pro.player}-${pro.team}-${pro.dpi}`}>
                  <span className="pro-player">
                    <strong>{pro.player}</strong>
                    <em>{pro.country}</em>
                  </span>
                  <span className="pro-team">{pro.team}</span>
                  <span className="tnum">{pro.dpi}</span>
                  <span className="tnum">{pro.sensitivity}</span>
                  <span className="tnum strong">{pro.edpi}</span>
                  <span className="tnum">{pro.scopedSensitivity}</span>
                </div>
              ))}
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
