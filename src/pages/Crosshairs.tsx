import { useEffect, useMemo, useState } from "react";
import {
  Check,
  Copy,
  Crosshair as CrosshairIcon,
  Heart,
  Search,
  SlidersHorizontal,
  X,
} from "lucide-react";
import crosshairsData from "../data/crosshairs.json";
import {
  buildCrosshair,
  CrosshairSettings,
  defaultSettings,
  parseCrosshair,
} from "../lib/crosshair";
import { copyText } from "../lib/clipboard";
import { cn } from "../lib/format";
import { CrosshairPreview, SCENES } from "../components/CrosshairPreview";
import { Modal } from "../components/Modal";

type Entry = {
  id: number;
  name: string;
  code: string;
  cat: string;
  copied: number;
};

const ALL = crosshairsData as Entry[];
const FAVORITES_KEY = "valotrack-crosshair-favorites";
const CUSTOM_KEY = "valotrack-custom-crosshairs";

function loadCustom(): Entry[] {
  try {
    const raw = JSON.parse(localStorage.getItem(CUSTOM_KEY) ?? "[]");
    return Array.isArray(raw)
      ? raw.filter(
          (entry) =>
            entry &&
            typeof entry.id === "number" &&
            typeof entry.name === "string" &&
            typeof entry.code === "string",
        )
      : [];
  } catch {
    return [];
  }
}

function useFavorites() {
  const [favorites, setFavorites] = useState<number[]>(() => {
    try {
      const raw = JSON.parse(localStorage.getItem(FAVORITES_KEY) ?? "[]");
      return Array.isArray(raw) ? raw.filter((id) => typeof id === "number") : [];
    } catch {
      return [];
    }
  });
  useEffect(() => {
    localStorage.setItem(FAVORITES_KEY, JSON.stringify(favorites));
  }, [favorites]);
  return {
    favorites,
    toggle: (id: number) =>
      setFavorites((current) =>
        current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
      ),
    isFavorite: (id: number) => favorites.includes(id),
  };
}

function CopyButton({ code, label = "Copy" }: { code: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      className="button small"
      onClick={async (event) => {
        event.stopPropagation();
        if (await copyText(code)) {
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1300);
        }
      }}
    >
      {copied ? <Check size={12} /> : <Copy size={12} />}
      {copied ? "Copied" : label}
    </button>
  );
}

function PreviewControls({
  scene,
  setScene,
  state,
  setState,
}: {
  scene: string;
  setScene: (scene: string) => void;
  state: "resting" | "moving" | "firing";
  setState: (state: "resting" | "moving" | "firing") => void;
}) {
  return (
    <div className="preview-controls">
      <div className="scene-tabs">
        {SCENES.map((item) => (
          <button
            key={item.id}
            className={scene === item.id ? "active" : ""}
            onClick={() => setScene(item.id)}
          >
            {item.name}
          </button>
        ))}
      </div>
      <div className="segmented">
        {(["resting", "moving", "firing"] as const).map((item) => (
          <button
            key={item}
            className={state === item ? "active" : ""}
            onClick={() => setState(item)}
          >
            {item}
          </button>
        ))}
      </div>
    </div>
  );
}

function DetailModal({
  entry,
  onClose,
  onBuild,
  favorite,
  toggleFavorite,
}: {
  entry: Entry;
  onClose: () => void;
  onBuild: (settings: CrosshairSettings) => void;
  favorite: boolean;
  toggleFavorite: () => void;
}) {
  const [scene, setScene] = useState("ascent");
  const [state, setState] = useState<"resting" | "moving" | "firing">("resting");
  const settings = useMemo(() => parseCrosshair(entry.code), [entry.code]);
  return (
    <Modal onClose={onClose} label={`${entry.name} crosshair`} className="crosshair-modal">
      <div className="crosshair-detail">
        <div className="crosshair-detail-preview">
          <CrosshairPreview settings={settings} size={330} scene={scene} state={state} />
          <PreviewControls
            scene={scene}
            setScene={setScene}
            state={state}
            setState={setState}
          />
        </div>
        <div className="crosshair-detail-info">
          <span className="profile-kicker">
            {entry.cat} · {entry.copied.toLocaleString()} copies
          </span>
          <h2>{entry.name}</h2>
          <p className="quiet">
            Pixel-accurate preview generated from the import code. Paste the code under
            Settings → Crosshair → Import Profile Code.
          </p>
          <code className="code-box">{entry.code}</code>
          <div className="detail-actions">
            <CopyButton code={entry.code} label="Copy code" />
            <button className={cn("button small", favorite && "favorited")} onClick={toggleFavorite}>
              <Heart size={12} fill={favorite ? "currentColor" : "none"} />
              {favorite ? "Saved" : "Favorite"}
            </button>
            <button className="button small" onClick={() => onBuild(settings)}>
              <SlidersHorizontal size={12} /> Edit in builder
            </button>
          </div>
          <div className="crosshair-specs">
            <div>
              <span>Inner</span>
              <strong>
                {settings.inner.length}L · {settings.inner.thickness}T · {settings.inner.offset}O
              </strong>
            </div>
            <div>
              <span>Outer</span>
              <strong>
                {settings.outer.length}L · {settings.outer.thickness}T · {settings.outer.offset}O
              </strong>
            </div>
            <div>
              <span>Dot</span>
              <strong>{settings.dot ? `on · ${settings.dotSize}` : "off"}</strong>
            </div>
            <div>
              <span>Outlines</span>
              <strong>{settings.outlines ? "on" : "off"}</strong>
            </div>
          </div>
        </div>
      </div>
    </Modal>
  );
}

function BuilderModal({
  initial,
  onClose,
  onSaveFavorite,
}: {
  initial: CrosshairSettings;
  onClose: () => void;
  onSaveFavorite: (code: string) => void;
}) {
  const [settings, setSettings] = useState<CrosshairSettings>(initial);
  const [scene, setScene] = useState("ascent");
  const [state, setState] = useState<"resting" | "moving" | "firing">("resting");
  const [importCode, setImportCode] = useState("");
  const code = buildCrosshair(settings);
  const [copied, setCopied] = useState(false);

  function update(patch: Partial<CrosshairSettings>) {
    setSettings((current) => ({ ...current, ...patch }));
  }
  function updateLine(which: "inner" | "outer", key: string, value: number | boolean) {
    setSettings((current) => ({
      ...current,
      [which]: { ...current[which], [key]: value },
    }));
  }

  return (
    <Modal onClose={onClose} label="Crosshair builder" className="builder-modal">
      <div className="builder-head">
        <div>
          <span className="profile-kicker">Crosshair studio</span>
          <h2>Build your own</h2>
        </div>
        <button
          className="button primary"
          onClick={async () => {
            if (await copyText(code)) {
              setCopied(true);
              window.setTimeout(() => setCopied(false), 1300);
            }
          }}
        >
          {copied ? <Check size={13} /> : <Copy size={13} />} {copied ? "Copied" : "Copy code"}
        </button>
      </div>
      <div className="builder-layout">
        <div className="builder-preview">
          <CrosshairPreview settings={settings} size={300} scene={scene} state={state} />
          <PreviewControls scene={scene} setScene={setScene} state={state} setState={setState} />
          <code className="code-box builder-code">{code}</code>
          <button className="button small" onClick={() => onSaveFavorite(code)}>
            <Heart size={12} /> Save to library
          </button>
        </div>
        <div className="builder-controls">
          <div className="builder-group">
            <span className="panel-label">Color</span>
            <div className="color-row">
              <div className="swatches">
                {[
                  "#FFFFFF",
                  "#00FF00",
                  "#BFFF00",
                  "#FFFF00",
                  "#00FFFF",
                  "#FF1493",
                  "#FF0000",
                  "#B565B5",
                  "#FFD700",
                ].map((color, index) => (
                  <button
                    key={color}
                    className={settings.colorIndex === index ? "active" : ""}
                    style={{ background: color }}
                    onClick={() =>
                      update({ colorIndex: index, customColor: color })
                    }
                    aria-label={`Color ${color}`}
                  />
                ))}
              </div>
              <input
                type="color"
                value={settings.customColor}
                onChange={(event) =>
                  update({ colorIndex: 8, customColor: event.target.value })
                }
                title="Custom color"
              />
            </div>
            <div className="inline-controls">
              <label>
                <span>Outlines</span>
                <input
                  type="checkbox"
                  checked={settings.outlines}
                  onChange={(event) => update({ outlines: event.target.checked })}
                />
              </label>
              <label>
                <span>Center dot</span>
                <input
                  type="checkbox"
                  checked={settings.dot}
                  onChange={(event) => update({ dot: event.target.checked })}
                />
              </label>
              <label className="wide">
                <span>Outline opacity</span>
                <input
                  type="range"
                  min={0}
                  max={1}
                  step={0.05}
                  value={settings.outlineOpacity}
                  onChange={(event) => update({ outlineOpacity: Number(event.target.value) })}
                />
              </label>
              <label className="wide">
                <span>Dot size</span>
                <input
                  type="range"
                  min={0}
                  max={10}
                  step={1}
                  value={settings.dotSize}
                  onChange={(event) => update({ dotSize: Number(event.target.value) })}
                />
              </label>
            </div>
          </div>

          {(["inner", "outer"] as const).map((which) => (
            <div className="builder-group" key={which}>
              <span className="panel-label">{which} lines</span>
              <div className="slider-grid">
                {[
                  ["thickness", "Thickness", 0, 10, 1],
                  ["length", "Length", 0, 20, 1],
                  ["offset", "Offset", 0, 20, 1],
                  ["opacity", "Opacity", 0, 1, 0.05],
                ].map(([key, label, min, max, step]) => (
                  <label key={key as string}>
                    <span>{label as string}</span>
                    <input
                      type="range"
                      min={min as number}
                      max={max as number}
                      step={step as number}
                      value={settings[which][key as "thickness"]}
                      onChange={(event) =>
                        updateLine(which, key as string, Number(event.target.value))
                      }
                    />
                    <strong>{settings[which][key as "thickness"]}</strong>
                  </label>
                ))}
              </div>
              <div className="inline-controls">
                <label>
                  <span>Movement error</span>
                  <input
                    type="checkbox"
                    checked={settings[which].movement}
                    onChange={(event) => updateLine(which, "movement", event.target.checked)}
                  />
                </label>
                <label>
                  <span>Firing error</span>
                  <input
                    type="checkbox"
                    checked={settings[which].firing}
                    onChange={(event) => updateLine(which, "firing", event.target.checked)}
                  />
                </label>
              </div>
            </div>
          ))}

          <div className="builder-group import-group">
            <span className="panel-label">Import a code</span>
            <div className="import-row">
              <input
                value={importCode}
                onChange={(event) => setImportCode(event.target.value)}
                placeholder="Paste a VALORANT crosshair code"
              />
              <button
                className="button small"
                onClick={() => {
                  if (importCode.includes(";")) setSettings(parseCrosshair(importCode));
                }}
              >
                Import
              </button>
            </div>
          </div>
        </div>
      </div>
    </Modal>
  );
}

function CompareModal({
  entries,
  onClose,
}: {
  entries: Entry[];
  onClose: () => void;
}) {
  const [left, setLeft] = useState(entries[0]?.id ?? ALL[0].id);
  const [right, setRight] = useState(entries[1]?.id ?? ALL[1].id);
  const leftEntry = entries.find((entry) => entry.id === left) ?? entries[0] ?? ALL[0];
  const rightEntry = entries.find((entry) => entry.id === right) ?? entries[1] ?? ALL[1];
  return (
    <Modal onClose={onClose} label="Compare crosshairs" className="compare-modal">
      <div className="builder-head">
        <div>
          <span className="profile-kicker">Side by side</span>
          <h2>Compare crosshairs</h2>
        </div>
      </div>
      <div className="compare-grid">
        {[
          [leftEntry, left, setLeft] as const,
          [rightEntry, right, setRight] as const,
        ].map(([entry, value, setValue]) => (
          <div className="compare-card" key={entry.id}>
            <CrosshairPreview settings={parseCrosshair(entry.code)} size={260} />
            <select
              value={value}
              onChange={(event) => setValue(Number(event.target.value))}
            >
              {entries.map((row) => (
                <option value={row.id} key={row.id}>
                  {row.name} · {row.cat}
                </option>
              ))}
            </select>
            <CopyButton code={entry.code} />
          </div>
        ))}
      </div>
    </Modal>
  );
}

export default function Crosshairs() {
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("All");
  const [sort, setSort] = useState<"popular" | "name">("popular");
  const [onlyFavorites, setOnlyFavorites] = useState(false);
  const [detail, setDetail] = useState<Entry | null>(null);
  const [builder, setBuilder] = useState<CrosshairSettings | null>(null);
  const [compare, setCompare] = useState(false);
  const [custom, setCustom] = useState<Entry[]>(loadCustom);
  const { favorites, toggle, isFavorite } = useFavorites();
  const entries = useMemo(() => [...ALL, ...custom], [custom]);

  const categories = useMemo(
    () => ["All", ...Array.from(new Set(entries.map((entry) => entry.cat))).sort()],
    [entries],
  );

  const rows = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return entries.filter((entry) => {
      if (category !== "All" && entry.cat !== category) return false;
      if (onlyFavorites && !favorites.includes(entry.id)) return false;
      if (!needle) return true;
      return entry.name.toLowerCase().includes(needle) || entry.code.includes(needle);
    }).sort((a, b) => {
      if (sort === "name") return a.name.localeCompare(b.name);
      return b.copied - a.copied;
    });
  }, [category, entries, favorites, onlyFavorites, search, sort]);

  const proCount = useMemo(() => ALL.filter((entry) => entry.cat === "Pro").length, []);

  return (
    <div className="page crosshairs-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <CrosshairIcon size={12} /> crosshair library
          </span>
          <h1>Find your dot.</h1>
          <p>
            {ALL.length} verified import codes, {proCount} from pros, rendered exactly as
            VALORANT draws them. Test movement and firing states, favorite, compare or
            build from scratch.
          </p>
        </div>
        <div className="head-status">
          <span className="checked">
            {favorites.length} favorites · {rows.length} shown
          </span>
          <div className="head-buttons">
            <button
              className={cn("button small", compare && "active")}
              onClick={() => setCompare(true)}
            >
              <SlidersHorizontal size={13} /> Compare
            </button>
            <button
              className="button small primary"
              onClick={() => setBuilder(defaultSettings())}
            >
              <CrosshairIcon size={13} /> Builder
            </button>
          </div>
        </div>
      </header>

      <div className="crosshair-toolbar">
        <div className="search-box crosshair-search">
          <Search size={14} />
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search name or code"
          />
        </div>
        <div className="filter-tabs">
          {categories.map((item) => (
            <button
              key={item}
              className={category === item ? "active" : ""}
              onClick={() => setCategory(item)}
            >
              {item}
            </button>
          ))}
        </div>
        <div className="toolbar-right">
          <button
            className={cn("icon-text-button", onlyFavorites && "active")}
            onClick={() => setOnlyFavorites((value) => !value)}
          >
            <Heart size={13} fill={onlyFavorites ? "currentColor" : "none"} /> Favorites
          </button>
          <select
            className="small-select"
            value={sort}
            onChange={(event) => setSort(event.target.value as "popular" | "name")}
          >
            <option value="popular">Most copied</option>
            <option value="name">Name A–Z</option>
          </select>
        </div>
      </div>

      <div className="crosshair-grid">
        {rows.map((entry) => {
          const settings = parseCrosshair(entry.code);
          const favorite = isFavorite(entry.id);
          return (
            <button
              className="crosshair-card"
              key={entry.id}
              onClick={() => setDetail(entry)}
            >
              <div className="crosshair-card-preview">
                <CrosshairPreview settings={settings} size={178} scene="bind" />
                <span className="crosshair-cat">{entry.cat}</span>
                <span
                  className={cn("crosshair-fav", favorite && "active")}
                  onClick={(event) => {
                    event.stopPropagation();
                    void toggle(entry.id);
                  }}
                  role="button"
                  aria-label="Favorite"
                >
                  <Heart size={13} fill={favorite ? "currentColor" : "none"} />
                </span>
              </div>
              <div className="crosshair-meta">
                <div>
                  <strong>{entry.name}</strong>
                  <span>{entry.copied.toLocaleString()} copies</span>
                </div>
                <CopyButton code={entry.code} />
              </div>
            </button>
          );
        })}
      </div>

      {!rows.length && (
        <div className="board-empty">
          <div className="empty-icon">
            <X size={20} />
          </div>
          <h2>No codes match</h2>
          <p>Try another name, category or clear your favorites filter.</p>
        </div>
      )}

      {detail && (
        <DetailModal
          entry={detail}
          onClose={() => setDetail(null)}
          onBuild={(settings) => {
            setDetail(null);
            setBuilder(settings);
          }}
          favorite={isFavorite(detail.id)}
          toggleFavorite={() => toggle(detail.id)}
        />
      )}

      {builder && (
        <BuilderModal
          initial={builder}
          onClose={() => setBuilder(null)}
          onSaveFavorite={(code) => {
            const name = window.prompt("Name this crosshair", "Custom crosshair");
            if (!name?.trim()) return;
            const entry: Entry = {
              id: -Date.now(),
              name: name.trim().slice(0, 60),
              code,
              cat: "Custom",
              copied: 0,
            };
            const next = [entry, ...custom].slice(0, 60);
            setCustom(next);
            localStorage.setItem(CUSTOM_KEY, JSON.stringify(next));
            setBuilder(null);
          }}
        />
      )}

      {compare && (
        <CompareModal
          entries={
            favorites.length
              ? entries.filter((entry) => favorites.includes(entry.id))
              : entries.slice(0, 2)
          }
          onClose={() => setCompare(false)}
        />
      )}
    </div>
  );
}
