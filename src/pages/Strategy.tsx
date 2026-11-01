import {
  type PointerEvent as ReactPointerEvent,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Circle,
  Download,
  Eraser,
  Hand,
  Layers,
  MousePointer2,
  MoveUpRight,
  Play,
  RotateCcw,
  Save,
  Square,
  Trash2,
  Type as TypeIcon,
  Upload,
  Users,
} from "lucide-react";
import lineupMeta from "../data/lineup-meta.json";
import abilityRadii from "../data/ability-radii.json";
import mapScale from "../data/map-scale.json";
import { copyText } from "../lib/clipboard";
import { cn } from "../lib/format";

type Ability = { name: string; icon: string; key: string };
type AgentKit = {
  name: string;
  slug: string;
  icon: string;
  role: string;
  kit: Ability[];
};
type Tool = "select" | "circle" | "wall" | "arrow" | "text" | "cone" | "erase";
type PlanObject = {
  id: string;
  type: Exclude<Tool, "select" | "erase">;
  x: number;
  y: number;
  x2: number;
  y2: number;
  text?: string;
  color: string;
  ability?: string;
  radius?: number;
  step: number;
};

type Plan = {
  map: string;
  side: "attack" | "defense";
  objects: PlanObject[];
  steps: number;
  name: string;
  savedAt: number;
};

const AGENTS = lineupMeta.X6 as AgentKit[];
const RADII = abilityRadii as Record<string, number>;
const SCALES = mapScale as Record<string, Record<"attack" | "defense", number>>;
const MAPS = Object.keys(SCALES).sort();
const TOOLS: { id: Tool; label: string; icon: typeof Circle; hint: string }[] = [
  { id: "select", label: "Move", icon: MousePointer2, hint: "Select or drag something" },
  { id: "circle", label: "Area", icon: Circle, hint: "Click to place a smoke or molly" },
  { id: "wall", label: "Wall", icon: Square, hint: "Drag to draw a wall" },
  { id: "arrow", label: "Arrow", icon: MoveUpRight, hint: "Drag to draw a rotation arrow" },
  { id: "cone", label: "Cone", icon: Users, hint: "Drag to aim a 90° vision cone" },
  { id: "text", label: "Text", icon: TypeIcon, hint: "Click to label the map" },
  { id: "erase", label: "Erase", icon: Eraser, hint: "Click an object to delete it" },
];
const COLORS = ["#f4f1e8", "#ff4655", "#4d8dff", "#ffd447", "#49d6a3"];
const STEPS = 10;
const STORAGE_KEY = "valotrack-strategy-library";

function uid() {
  return Math.random().toString(36).slice(2, 10);
}

function abilityRadius(name: string, scale: number) {
  const clean = name.split(" / ")[0].trim();
  const meters = RADII[clean] ?? 4;
  return (meters * scale * 100) / 1024;
}

export default function Strategy() {
  const [map, setMap] = useState("Ascent");
  const [side, setSide] = useState<"attack" | "defense">("attack");
  const [tool, setTool] = useState<Tool>("select");
  const [color, setColor] = useState(COLORS[1]);
  const [agentName, setAgentName] = useState(AGENTS[0].name);
  const [abilityName, setAbilityName] = useState(AGENTS[0].kit[0]?.name ?? "");
  const [objects, setObjects] = useState<PlanObject[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [draft, setDraft] = useState<PlanObject | null>(null);
  const [step, setStep] = useState(1);
  const [playing, setPlaying] = useState(false);
  const [library, setLibrary] = useState<Plan[]>(() => {
    try {
      const raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]");
      return Array.isArray(raw) ? raw : [];
    } catch {
      return [];
    }
  });
  const [shareCode, setShareCode] = useState("");
  const [showShare, setShowShare] = useState(false);
  const [message, setMessage] = useState("");
  const dragRef = useRef<{ x: number; y: number; object?: PlanObject } | null>(null);
  const mapRef = useRef<HTMLDivElement>(null);

  const agent = AGENTS.find((item) => item.name === agentName) ?? AGENTS[0];
  const scale = SCALES[map]?.[side] ?? 14;
  const image = `/maps/${map.toLowerCase()}-${side}.svg`;

  const visibleObjects = useMemo(() => {
    if (playing) return objects.filter((object) => object.step <= step);
    return objects;
  }, [objects, playing, step]);

  useEffect(() => {
    if (!playing) return;
    if (step >= STEPS) {
      setPlaying(false);
      return;
    }
    const timer = window.setTimeout(() => setStep((value) => value + 1), 1100);
    return () => window.clearTimeout(timer);
  }, [playing, step]);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Delete" || event.key === "Backspace") {
        if (selected) {
          setObjects((current) => current.filter((object) => object.id !== selected));
          setSelected(null);
        }
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") {
        setObjects((current) => current.slice(0, -1));
      }
      if (event.key === "Escape") {
        setDraft(null);
        setSelected(null);
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selected]);

  function point(event: ReactPointerEvent) {
    const rect = mapRef.current?.getBoundingClientRect();
    if (!rect) return { x: 0, y: 0 };
    return {
      x: ((event.clientX - rect.left) / rect.width) * 100,
      y: ((event.clientY - rect.top) / rect.height) * 100,
    };
  }

  function startDraw(event: ReactPointerEvent) {
    if (tool === "select" || tool === "erase") return;
    const start = point(event);
    if (tool === "circle" || tool === "text") return;
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = { ...start };
    setDraft({
      id: uid(),
      type: tool,
      x: start.x,
      y: start.y,
      x2: start.x,
      y2: start.y,
      color,
      step,
      text: "",
    });
  }

  function moveDraw(event: ReactPointerEvent) {
    if (!dragRef.current) return;
    const next = point(event);
    setDraft((current) =>
      current ? { ...current, x2: next.x, y2: next.y } : current,
    );
  }

  function endDraw(event: ReactPointerEvent) {
    if (!dragRef.current) return;
    const end = point(event);
    dragRef.current = null;
    setDraft((current) => {
      if (!current) return current;
      const distance = Math.hypot(end.x - current.x, end.y - current.y);
      if (distance < 0.5) {
        return current;
      }
      const next = { ...current, x2: end.x, y2: end.y };
      setObjects((items) => [...items, next]);
      setSelected(next.id);
      return null;
    });
  }

  function clickMap(event: ReactPointerEvent) {
    if (tool === "circle") {
      const start = point(event);
      const distance = abilityRadius(abilityName, scale);
      const object: PlanObject = {
        id: uid(),
        type: "circle",
        x: start.x,
        y: start.y,
        x2: start.x,
        y2: start.y,
        color,
        step,
        ability: abilityName,
        radius: distance,
      };
      setObjects((current) => [...current, object]);
      setSelected(object.id);
    }
    if (tool === "text") {
      const start = point(event);
      const text = window.prompt("Label");
      if (!text?.trim()) return;
      const object: PlanObject = {
        id: uid(),
        type: "text",
        x: start.x,
        y: start.y,
        x2: start.x,
        y2: start.y,
        color,
        step,
        text: text.trim().slice(0, 40),
      };
      setObjects((current) => [...current, object]);
      setSelected(object.id);
    }
    if (tool === "erase") {
      setSelected(null);
    }
  }

  function objectPointerDown(event: ReactPointerEvent, object: PlanObject) {
    event.stopPropagation();
    if (tool === "erase") {
      setObjects((current) => current.filter((item) => item.id !== object.id));
      setSelected(null);
      return;
    }
    if (tool !== "select") return;
    setSelected(object.id);
    const start = point(event);
    dragRef.current = { ...start, object };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function objectPointerMove(event: ReactPointerEvent) {
    if (!dragRef.current?.object) return;
    const next = point(event);
    const dx = next.x - dragRef.current.x;
    const dy = next.y - dragRef.current.y;
    const id = dragRef.current.object.id;
    setObjects((current) =>
      current.map((object) =>
        object.id === id
          ? {
              ...object,
              x: object.x + dx,
              y: object.y + dy,
              x2: object.x2 + dx,
              y2: object.y2 + dy,
            }
          : object,
      ),
    );
    dragRef.current.x = next.x;
    dragRef.current.y = next.y;
  }

  function stopObjectDrag() {
    dragRef.current = null;
  }

  function savePlan() {
    const name = window.prompt("Name this strat", `${map} ${side} ${agent.name}`);
    if (!name) return;
    const plan: Plan = {
      map,
      side,
      objects,
      steps: step,
      name: name.slice(0, 60),
      savedAt: Date.now(),
    };
    const next = [plan, ...library.filter((item) => item.name !== plan.name)].slice(0, 40);
    setLibrary(next);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    setMessage("Strat saved to your local library.");
  }

  function loadPlan(plan: Plan) {
    setMap(plan.map);
    setSide(plan.side);
    setObjects(plan.objects);
    setStep(Math.max(1, plan.steps));
    setMessage(`Loaded ${plan.name}.`);
  }

  function clear() {
    if (!objects.length) return;
    if (window.confirm("Clear this board?")) {
      setObjects([]);
      setSelected(null);
    }
  }

  async function share() {
    const payload = JSON.stringify({ map, side, objects, steps: step, name: `${map} ${side}` });
    const encoded = btoa(unescape(encodeURIComponent(payload)));
    setShareCode(encoded);
    setShowShare(true);
    await copyText(encoded);
    setMessage("Plan code copied. Send it to a teammate.");
  }

  async function importPlan() {
    const raw = window.prompt("Paste a Valotrack plan code");
    if (!raw) return;
    try {
      const parsed = JSON.parse(decodeURIComponent(escape(atob(raw.trim()))));
      if (!Array.isArray(parsed.objects)) throw new Error("invalid");
      setMap(MAPS.includes(parsed.map) ? parsed.map : map);
      setSide(parsed.side === "defense" ? "defense" : "attack");
      setObjects(parsed.objects);
      setMessage("Plan imported.");
    } catch {
      setMessage("That plan code could not be read.");
    }
  }

  const selectedObject = objects.find((object) => object.id === selected) ?? null;

  return (
    <div className="page strategy-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Layers size={12} /> strategy planner
          </span>
          <h1>Draw the round.</h1>
          <p>
            Real-scale maps, ability radii from published values, walls, arrows, cones
            and up to ten steps. Everything stays local and autosaves to your library.
          </p>
        </div>
        <div className="head-status">
          <div className="head-buttons">
            <button className="button small" onClick={importPlan}>
              <Upload size={13} /> Import
            </button>
            <button className="button small" onClick={share}>
              <Download size={13} /> Share code
            </button>
            <button className="button small primary" onClick={savePlan}>
              <Save size={13} /> Save strat
            </button>
          </div>
        </div>
      </header>

      {message && (
        <div className="action-result ok">
          <span>{message}</span>
          <button className="icon-button" onClick={() => setMessage("")}>
            ×
          </button>
        </div>
      )}

      <div className="strategy-layout">
        <aside className="strategy-side panel">
          <div className="strategy-group">
            <span className="panel-label">Map</span>
            <select value={map} onChange={(event) => setMap(event.target.value)}>
              {MAPS.map((name) => (
                <option key={name}>{name}</option>
              ))}
            </select>
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
          </div>

          <div className="strategy-group">
            <span className="panel-label">Tool</span>
            <div className="tool-grid">
              {TOOLS.map((item) => {
                const Icon = item.icon;
                return (
                  <button
                    key={item.id}
                    className={tool === item.id ? "active" : ""}
                    onClick={() => setTool(item.id)}
                    title={item.hint}
                  >
                    <Icon size={14} />
                    <span>{item.label}</span>
                  </button>
                );
              })}
            </div>
            <p className="tool-hint">{TOOLS.find((item) => item.id === tool)?.hint}</p>
          </div>

          <div className="strategy-group">
            <span className="panel-label">Agent</span>
            <div className="agent-select-row">
              <img src={agent.icon} alt="" />
              <select
                value={agentName}
                onChange={(event) => {
                  const next = AGENTS.find((item) => item.name === event.target.value) ?? AGENTS[0];
                  setAgentName(next.name);
                  setAbilityName(next.kit[0]?.name ?? "");
                }}
              >
                {AGENTS.map((item) => (
                  <option key={item.name}>{item.name}</option>
                ))}
              </select>
            </div>
            <div className="ability-list">
              {agent.kit.map((ability) => (
                <button
                  key={ability.name}
                  className={abilityName === ability.name ? "active" : ""}
                  onClick={() => {
                    setAbilityName(ability.name);
                    setTool("circle");
                  }}
                >
                  <img src={ability.icon} alt="" />
                  <span>{ability.name}</span>
                  <em>{ability.key}</em>
                </button>
              ))}
            </div>
          </div>

          <div className="strategy-group">
            <span className="panel-label">Color</span>
            <div className="color-row">
              <div className="swatches">
                {COLORS.map((item) => (
                  <button
                    key={item}
                    className={color === item ? "active" : ""}
                    style={{ background: item }}
                    onClick={() => setColor(item)}
                  />
                ))}
              </div>
            </div>
          </div>

          <div className="strategy-group">
            <span className="panel-label">Steps</span>
            <div className="step-row">
              {Array.from({ length: STEPS }, (_, index) => index + 1).map((value) => (
                <button
                  key={value}
                  className={step === value ? "active" : ""}
                  onClick={() => {
                    setStep(value);
                    setPlaying(false);
                  }}
                >
                  {value}
                </button>
              ))}
            </div>
            <div className="step-actions">
              <button
                className="button small"
                onClick={() => {
                  setStep(1);
                  setPlaying(true);
                }}
              >
                <Play size={12} /> Play
              </button>
              <button
                className="button small"
                onClick={() => {
                  setPlaying(false);
                  setStep(1);
                }}
              >
                <RotateCcw size={12} /> Reset
              </button>
            </div>
          </div>
        </aside>

        <section className="strategy-board-panel panel">
          <div className="strategy-board-head">
            <div>
              <span className="panel-label">
                {map} · {side}
              </span>
              <h2>Round plan</h2>
            </div>
            <div className="strategy-board-actions">
              {selectedObject && (
                <button
                  className="button small danger"
                  onClick={() => {
                    setObjects((current) =>
                      current.filter((object) => object.id !== selectedObject.id),
                    );
                    setSelected(null);
                  }}
                >
                  <Trash2 size={12} /> Remove
                </button>
              )}
              <button className="button small" onClick={clear}>
                Clear board
              </button>
            </div>
          </div>
          <div
            className="strategy-map"
            ref={mapRef}
            onPointerDown={(event) => {
              clickMap(event);
              startDraw(event);
            }}
            onPointerMove={moveDraw}
            onPointerUp={endDraw}
          >
            <img src={image} alt={`${map} ${side}`} draggable={false} />
            <svg className="strategy-svg" viewBox="0 0 100 100" preserveAspectRatio="none">
              {visibleObjects.map((object) => {
                const active = selected === object.id;
                const opacity = playing && object.step !== step ? 0.15 : 1;
                if (object.type === "circle") {
                  return (
                    <g key={object.id} opacity={opacity}>
                      <circle
                        cx={object.x}
                        cy={object.y}
                        r={object.radius ?? 5}
                        fill={object.color}
                        fillOpacity="0.14"
                        stroke={object.color}
                        strokeWidth={active ? 0.6 : 0.35}
                        onPointerDown={(event) => objectPointerDown(event, object)}
                        onPointerMove={objectPointerMove}
                        onPointerUp={stopObjectDrag}
                        className="strategy-object"
                      />
                      {object.ability && (
                        <text
                          x={object.x}
                          y={object.y - (object.radius ?? 5) - 1}
                          fill={object.color}
                          fontSize="2.2"
                          textAnchor="middle"
                          className="strategy-label"
                        >
                          {object.ability}
                        </text>
                      )}
                    </g>
                  );
                }
                if (object.type === "wall") {
                  return (
                    <line
                      key={object.id}
                      x1={object.x}
                      y1={object.y}
                      x2={object.x2}
                      y2={object.y2}
                      stroke={object.color}
                      strokeWidth={active ? 2 : 1.4}
                      strokeLinecap="round"
                      opacity={opacity}
                      onPointerDown={(event) => objectPointerDown(event, object)}
                      onPointerMove={objectPointerMove}
                      onPointerUp={stopObjectDrag}
                      className="strategy-object"
                    />
                  );
                }
                if (object.type === "arrow") {
                  return (
                    <g key={object.id} opacity={opacity}>
                      <line
                        x1={object.x}
                        y1={object.y}
                        x2={object.x2}
                        y2={object.y2}
                        stroke={object.color}
                        strokeWidth={active ? 1.2 : 0.8}
                        markerEnd="url(#arrowhead)"
                        onPointerDown={(event) => objectPointerDown(event, object)}
                        onPointerMove={objectPointerMove}
                        onPointerUp={stopObjectDrag}
                        className="strategy-object"
                      />
                    </g>
                  );
                }
                if (object.type === "cone") {
                  const angle = Math.atan2(object.y2 - object.y, object.x2 - object.x);
                  const radius = Math.hypot(object.x2 - object.x, object.y2 - object.y);
                  const spread = Math.PI / 4;
                  const x1 = object.x + Math.cos(angle - spread) * radius;
                  const y1 = object.y + Math.sin(angle - spread) * radius;
                  const x2 = object.x + Math.cos(angle + spread) * radius;
                  const y2 = object.y + Math.sin(angle + spread) * radius;
                  return (
                    <path
                      key={object.id}
                      d={`M ${object.x} ${object.y} L ${x1} ${y1} A ${radius} ${radius} 0 0 1 ${x2} ${y2} Z`}
                      fill={object.color}
                      fillOpacity="0.1"
                      stroke={object.color}
                      strokeWidth={active ? 0.5 : 0.3}
                      opacity={opacity}
                      onPointerDown={(event) => objectPointerDown(event, object)}
                      onPointerMove={objectPointerMove}
                      onPointerUp={stopObjectDrag}
                      className="strategy-object"
                    />
                  );
                }
                if (object.type === "text") {
                  return (
                    <text
                      key={object.id}
                      x={object.x}
                      y={object.y}
                      fill={object.color}
                      fontSize="2.6"
                      fontWeight="700"
                      textAnchor="middle"
                      opacity={opacity}
                      onPointerDown={(event) => objectPointerDown(event, object)}
                      onPointerMove={objectPointerMove}
                      onPointerUp={stopObjectDrag}
                      className="strategy-object strategy-label"
                    >
                      {object.text}
                    </text>
                  );
                }
                return null;
              })}
              {draft && (
                <g opacity="0.8">
                  {draft.type === "wall" && (
                    <line
                      x1={draft.x}
                      y1={draft.y}
                      x2={draft.x2}
                      y2={draft.y2}
                      stroke={draft.color}
                      strokeWidth="1.4"
                    />
                  )}
                  {draft.type === "arrow" && (
                    <line
                      x1={draft.x}
                      y1={draft.y}
                      x2={draft.x2}
                      y2={draft.y2}
                      stroke={draft.color}
                      strokeWidth="0.8"
                      markerEnd="url(#arrowhead)"
                    />
                  )}
                  {draft.type === "cone" && (
                    <line
                      x1={draft.x}
                      y1={draft.y}
                      x2={draft.x2}
                      y2={draft.y2}
                      stroke={draft.color}
                      strokeWidth="0.5"
                    />
                  )}
                </g>
              )}
              <defs>
                <marker
                  id="arrowhead"
                  markerWidth="6"
                  markerHeight="6"
                  refX="4"
                  refY="3"
                  orient="auto"
                >
                  <path d="M0,0 L6,3 L0,6 Z" fill={color} />
                </marker>
              </defs>
            </svg>
          </div>
          <div className="strategy-foot">
            <span>
              <Hand size={12} /> Ctrl + Z to undo · Delete removes selection
            </span>
            <span>{objects.length} objects · step {step}</span>
          </div>
        </section>

        <aside className="strategy-library panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Library</span>
              <h2>Saved strats</h2>
            </div>
            <Save size={17} className="panel-icon" />
          </div>
          {library.length ? (
            <div className="strat-list">
              {library.map((plan) => (
                <div className={cn("strat-row", plan.map === map && plan.side === side && "active")} key={`${plan.name}-${plan.savedAt}`}>
                  <button className="strat-load" onClick={() => loadPlan(plan)}>
                    <strong>{plan.name}</strong>
                    <span>
                      {plan.map} · {plan.side} · {plan.objects.length} objects
                    </span>
                  </button>
                  <button
                    className="icon-button"
                    onClick={() => {
                      const next = library.filter((item) => item.savedAt !== plan.savedAt);
                      setLibrary(next);
                      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
                    }}
                    aria-label="Delete strat"
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <p className="quiet">No saved strats yet. Build a round and save it here.</p>
          )}
          {showShare && (
            <div className="share-box">
              <span className="panel-label">Share code</span>
              <code>{shareCode}</code>
              <button className="button small" onClick={() => void copyText(shareCode)}>
                Copy
              </button>
            </div>
          )}
        </aside>
      </div>
    </div>
  );
}
