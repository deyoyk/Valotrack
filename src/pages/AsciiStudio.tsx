import { useEffect, useMemo, useRef, useState } from "react";
import {
  Check,
  Copy,
  Eraser,
  Grid3X3,
  Heart,
  Maximize2,
  Paintbrush,
  Search,
  Sparkles,
  Type,
} from "lucide-react";
import asciiData from "../data/ascii.json";
import {
  artToChat,
  decodeGalleryArt,
  encodeForChat,
  STYLES,
  textToRows,
} from "../lib/ascii";
import { copyText } from "../lib/clipboard";
import { cn } from "../lib/format";
import { Modal } from "../components/Modal";

type Art = { t: string; g: string; a: string; s: string };

const GALLERY = asciiData as Art[];
const FAVORITES_KEY = "valotrack-ascii-favorites";
const GRID_WIDTH = 30;
const GRID_HEIGHT = 13;

function GalleryArt({ art }: { art: Art }) {
  const rows = useMemo(() => decodeGalleryArt(art.s), [art.s]);
  return (
    <pre className="ascii-art">
      {rows.map((row) => row.join("")).join("\n")}
    </pre>
  );
}

function CopyArt({ text, label = "Copy" }: { text: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      className="button small"
      onClick={async (event) => {
        event.stopPropagation();
        if (await copyText(text)) {
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

function DrawGrid({
  cells,
  setCells,
  brush,
  large,
}: {
  cells: string[];
  setCells: (cells: string[]) => void;
  brush: string;
  large?: boolean;
}) {
  const drawing = useRef(false);
  function paint(index: number) {
    const next = [...cells];
    next[index] = brush;
    setCells(next);
  }
  return (
    <div
      className={cn("draw-grid", large && "large")}
      onMouseUp={() => {
        drawing.current = false;
      }}
      onMouseLeave={() => {
        drawing.current = false;
      }}
    >
      {cells.map((cell, index) => (
        <button
          key={index}
          className={cell !== " " ? "filled" : ""}
          onMouseDown={() => {
            drawing.current = true;
            paint(index);
          }}
          onMouseEnter={() => {
            if (drawing.current) paint(index);
          }}
          aria-label={`cell ${index}`}
        >
          {cell}
        </button>
      ))}
    </div>
  );
}

function Creator() {
  const [mode, setMode] = useState<"text" | "draw">("text");
  const [text, setText] = useState("VALOTRACK");
  const [style, setStyle] = useState("solid");
  const [cells, setCells] = useState<string[]>(() =>
    Array(GRID_WIDTH * GRID_HEIGHT).fill(" "),
  );
  const [brush, setBrush] = useState("█");
  const [focus, setFocus] = useState(false);
  const selectedStyle = STYLES.find((item) => item.id === style) ?? STYLES[0];

  const textRows = useMemo(() => textToRows(text.slice(0, 18)), [text]);
  const rows = mode === "text" ? textRows : Array.from({ length: GRID_HEIGHT }, (_, row) =>
    cells.slice(row * GRID_WIDTH, (row + 1) * GRID_WIDTH),
  );
  const chatRows =
    mode === "text"
      ? textRows.map((row) =>
          row.map((cell) => (cell === "1" ? selectedStyle.filled : " ")),
        )
      : rows;
  const filledChat = encodeForChat(chatRows, selectedStyle.empty);

  function clear() {
    setCells(Array(GRID_WIDTH * GRID_HEIGHT).fill(" "));
  }

  return (
    <div className="ascii-creator">
      <div className="creator-toolbar">
        <div className="segmented">
          <button className={mode === "text" ? "active" : ""} onClick={() => setMode("text")}>
            <Type size={12} /> Text banner
          </button>
          <button className={mode === "draw" ? "active" : ""} onClick={() => setMode("draw")}>
            <Paintbrush size={12} /> Draw grid
          </button>
        </div>
        <label className="style-select">
          <span>Style</span>
          <select value={style} onChange={(event) => setStyle(event.target.value)}>
            {STYLES.map((item) => (
              <option value={item.id} key={item.id}>
                {item.label}
              </option>
            ))}
          </select>
        </label>
        <button className="button small" onClick={() => setFocus(true)}>
          <Maximize2 size={12} /> Focus canvas
        </button>
      </div>

      {mode === "text" ? (
        <label className="field">
          <span>Text</span>
          <input
            value={text}
            maxLength={18}
            onChange={(event) => setText(event.target.value)}
            placeholder="Type a word"
          />
          <small>Letters, numbers and basic punctuation. One line, up to 18 characters.</small>
        </label>
      ) : (
        <div className="draw-tools">
          <div className="brush-row">
            <span>Brush</span>
            {["█", "▓", "▒", "░", "#", "@", "●", "★", " "].map((item) => (
              <button
                key={item || "erase"}
                className={brush === item ? "active" : ""}
                onClick={() => setBrush(item)}
              >
                {item || "·"}
              </button>
            ))}
          </div>
          <button className="button small" onClick={clear}>
            <Eraser size={12} /> Clear
          </button>
        </div>
      )}

      <div className="creator-preview">
        {mode === "text" ? (
          <pre className="ascii-art generated">
            {rows
              .map((row) =>
                row
                  .map((cell) => (cell === "1" ? selectedStyle.filled : " "))
                  .join(""),
              )
              .join("\n")}
          </pre>
        ) : (
          <DrawGrid cells={cells} setCells={setCells} brush={brush} />
        )}
        {mode === "text" && (
          <div className="style-legend">
            {STYLES.map((item) => (
              <button
                key={item.id}
                className={style === item.id ? "active" : ""}
                onClick={() => setStyle(item.id)}
              >
                {item.filled}
                {item.empty}
                <span>{item.label}</span>
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="creator-output">
        <div>
          <span className="panel-label">Chat output</span>
          <p>
            One line, fixed width, spaces replaced with a visible background so VALORANT
            doesn't collapse it.
          </p>
        </div>
        <CopyArt text={filledChat} label="Copy chat art" />
      </div>

      {focus && (
        <Modal onClose={() => setFocus(false)} label="ASCII focus canvas" className="ascii-focus-modal">
          <div className="focus-head">
            <div>
              <span className="profile-kicker">Focus canvas</span>
              <h2>{mode === "text" ? text || "Banner" : "Your art"}</h2>
            </div>
            <CopyArt text={filledChat} label="Copy" />
          </div>
          {mode === "text" ? (
            <pre className="ascii-art focus">
              {rows
                .map((row) =>
                  row
                    .map((cell) => (cell === "1" ? selectedStyle.filled : selectedStyle.empty))
                    .join(""),
                )
                .join("\n")}
            </pre>
          ) : (
            <DrawGrid cells={cells} setCells={setCells} brush={brush} large />
          )}
        </Modal>
      )}
    </div>
  );
}

export default function AsciiStudio() {
  const [tab, setTab] = useState<"gallery" | "creator">("gallery");
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("All");
  const [favorites, setFavorites] = useState<string[]>(() => {
    try {
      const raw = JSON.parse(localStorage.getItem(FAVORITES_KEY) ?? "[]");
      return Array.isArray(raw) ? raw : [];
    } catch {
      return [];
    }
  });
  const [selected, setSelected] = useState<Art | null>(null);

  useEffect(() => {
    localStorage.setItem(FAVORITES_KEY, JSON.stringify(favorites));
  }, [favorites]);

  const categories = useMemo(
    () => ["All", ...Array.from(new Set(GALLERY.map((art) => art.g))).sort()],
    [],
  );
  const rows = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return GALLERY.filter((art) => {
      if (category !== "All" && art.g !== category) return false;
      if (!needle) return true;
      return (
        art.t.toLowerCase().includes(needle) ||
        art.a.toLowerCase().includes(needle) ||
        art.g.toLowerCase().includes(needle)
      );
    });
  }, [category, search]);

  return (
    <div className="page ascii-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Grid3X3 size={12} /> chat art studio
          </span>
          <h1>Say it in blocks.</h1>
          <p>
            {GALLERY.length} paste-ready pieces plus a text-to-banner maker and paint
            grid. Output is encoded so it survives the VALORANT chat box.
          </p>
        </div>
        <div className="head-status">
          <span className="checked">{favorites.length} favorites</span>
        </div>
      </header>

      <div className="ascii-tabs">
        <button className={tab === "gallery" ? "active" : ""} onClick={() => setTab("gallery")}>
          Gallery
        </button>
        <button className={tab === "creator" ? "active" : ""} onClick={() => setTab("creator")}>
          Creator
        </button>
      </div>

      {tab === "gallery" ? (
        <>
          <div className="ascii-toolbar">
            <div className="search-box ascii-search">
              <Search size={14} />
              <input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Search art, author or category"
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
          </div>
          <div className="ascii-grid">
            {rows.map((art) => {
              const favorite = favorites.includes(art.t);
              return (
                <button className="ascii-card" key={`${art.t}-${art.a}`} onClick={() => setSelected(art)}>
                  <div className="ascii-card-art">
                    <GalleryArt art={art} />
                  </div>
                  <div className="ascii-card-meta">
                    <span>
                      <strong>{art.t}</strong>
                      <em>
                        {art.g} · {art.a}
                      </em>
                    </span>
                    <span
                      className={cn("crosshair-fav", favorite && "active")}
                      onClick={(event) => {
                        event.stopPropagation();
                        setFavorites((current) =>
                          current.includes(art.t)
                            ? current.filter((value) => value !== art.t)
                            : [...current, art.t],
                        );
                      }}
                    >
                      <Heart size={12} fill={favorite ? "currentColor" : "none"} />
                    </span>
                  </div>
                </button>
              );
            })}
          </div>
          {!rows.length && (
            <div className="board-empty">
              <div className="empty-icon">
                <Search size={20} />
              </div>
              <h2>No art matches</h2>
              <p>Try another term or category.</p>
            </div>
          )}
        </>
      ) : (
        <Creator />
      )}

      {selected && (
        <Modal onClose={() => setSelected(null)} label={`${selected.t} ASCII art`} className="ascii-modal">
          <div className="ascii-detail-head">
            <div>
              <span className="profile-kicker">
                {selected.g} · by {selected.a}
              </span>
              <h2>{selected.t}</h2>
            </div>
            <CopyArt text={artToChat(selected.s)} label="Copy chat art" />
          </div>
          <div className="ascii-detail-art">
            <GalleryArt art={selected} />
          </div>
          <div className="ascii-detail-foot">
            <Sparkles size={13} />
            Empty cells are filled with a visible block when copied so the shape holds in
            chat.
          </div>
        </Modal>
      )}
    </div>
  );
}
