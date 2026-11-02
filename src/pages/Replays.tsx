import {
  type CSSProperties,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  AlertTriangle,
  Crosshair,
  FileVideo2,
  Layers,
  Loader2,
  Pause,
  Play,
  RotateCcw,
  SkipBack,
  SkipForward,
  Upload,
} from "lucide-react";
import { api, Agent } from "../lib/api";
import replayMaps from "../data/replay-maps.json";
import agentCodenames from "../data/agent-codenames.json";
import { cn } from "../lib/format";

type ReplayPlayer = {
  id: string;
  name: string;
  agent: string;
  team: string;
  rankTier?: number;
  puuid?: string;
  stats?: Record<string, number | string | boolean | Record<string, number>>;
};

type ReplayEvent = {
  t: number;
  type: string;
  player?: string;
  victim?: string;
  killer?: string;
  assisters?: string[];
  x?: number;
  y?: number;
  site?: string;
  headshot?: boolean;
  weapon?: string;
  amount?: number;
};

type ReplayRound = {
  round: number;
  startMs: number;
  endMs: number;
  winner?: string;
  result?: string;
  score?: Record<string, number>;
  events: ReplayEvent[];
  tracks: Record<string, [number, number, number, number?][]>;
  states?: Record<string, { hp?: [number, number, number][] }>;
  utility?: { player: string; slot: string; name: string; t: number; end: number; x: number; y: number }[];
  stats?: Record<string, Record<string, number | boolean>>;
};

type Replay = {
  matchId: string;
  mapUrl: string;
  lengthMs: number;
  build?: string;
  matchInfo?: { mode?: string; startTime?: string; gameVersion?: string };
  players: ReplayPlayer[];
  rounds: ReplayRound[];
};

type MapMeta = {
  name: string;
  mapUrl: string;
  displayIcon: string;
  xMultiplier: number;
  yMultiplier: number;
  xScalarToAdd: number;
  yScalarToAdd: number;
  callouts: [string, number, number][];
};

const MAP_META = (replayMaps as unknown as { Kn: MapMeta[] }).Kn;
const CODENAMES = agentCodenames as Record<string, string>;
const TEAM_COLORS: Record<string, string> = { Red: "#ff4655", Blue: "#58a6ff" };

function clamp(value: number) {
  return Math.min(1, Math.max(0, value));
}

function trackAt(
  tracks: [number, number, number, number?][] | undefined,
  time: number,
): [number, number, number] | null {
  if (!tracks?.length) return null;
  if (time < tracks[0][0]) return [tracks[0][1], tracks[0][2], tracks[0][3] ?? 0];
  let low = 1;
  let high = tracks.length - 1;
  while (low < high) {
    const mid = (low + high) >> 1;
    if (tracks[mid][0] < time) low = mid + 1;
    else high = mid;
  }
  const next = tracks[low];
  const prev = tracks[low - 1];
  const span = next[0] - prev[0];
  const ratio = span > 0 ? (time - prev[0]) / span : 0;
  return [
    prev[1] + (next[1] - prev[1]) * ratio,
    prev[2] + (next[2] - prev[2]) * ratio,
    (prev[3] ?? 0) + ((((next[3] ?? 0) - (prev[3] ?? 0) + 540) % 360) - 180) * ratio,
  ];
}

function hpAt(states: ReplayRound["states"], id: string, time: number) {
  const rows = states?.[id]?.hp;
  if (!rows?.length) return 100;
  let value = rows[0][1];
  for (const row of rows) {
    if (row[0] > time) break;
    value = row[1];
  }
  return value;
}

function worldToPercent(x: number, y: number, meta: MapMeta) {
  return {
    left: clamp(y * meta.xMultiplier + meta.xScalarToAdd) * 100,
    top: clamp(x * meta.yMultiplier + meta.yScalarToAdd) * 100,
  };
}

function formatClock(ms: number) {
  const total = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
}

function mapName(url: string) {
  return MAP_META.find((meta) => meta.mapUrl === url)?.name ?? url.split("/").filter(Boolean).pop() ?? "Unknown";
}

function agentName(codename: string) {
  return CODENAMES[codename] ?? codename ?? "Agent";
}

function modeLabel(mode?: string) {
  if (!mode) return "Replay";
  return mode
    .replace(/GameMode$/, "")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/^Bomb$/, "Standard");
}

function teamScore(replay: Replay, index: number) {
  const rounds = replay.rounds.slice(0, index + 1);
  const score: Record<string, number> = {};
  for (const round of rounds) {
    if (round.winner) score[round.winner] = (score[round.winner] ?? 0) + 1;
  }
  return score;
}

function parseError(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

function statText(player: ReplayPlayer, key: string) {
  const value = player.stats?.[key];
  return typeof value === "number" || typeof value === "string" ? value : "-";
}

export default function Replays() {
  const [replay, setReplay] = useState<Replay | null>(null);
  const [fileName, setFileName] = useState("");
  const [progress, setProgress] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [roundIndex, setRoundIndex] = useState(0);
  const [time, setTime] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [showCallouts, setShowCallouts] = useState(false);
  const [tab, setTab] = useState<"overview" | "scoreboard">("overview");
  const workerRef = useRef<Worker | null>(null);
  const requestRef = useRef(0);
  const frameRef = useRef<number>(0);
  const lastFrame = useRef(0);

  useEffect(() => {
    api.agents().then((catalog) => setAgents(catalog.agents)).catch(() => void 0);
    return () => {
      workerRef.current?.terminate();
      cancelAnimationFrame(frameRef.current);
    };
  }, []);

  const round = replay?.rounds[roundIndex];
  const meta = replay ? MAP_META.find((item) => item.mapUrl === replay.mapUrl) : undefined;

  const agentMap = useMemo(() => {
    const map = new Map<string, Agent>();
    for (const agent of agents) map.set(agent.name, agent);
    return map;
  }, [agents]);

  const loadReplay = useCallback((parsed: Replay, name: string) => {
    if (!parsed?.rounds?.length) {
      setError("No rounds were found in that replay.");
      return;
    }
    setReplay(parsed);
    setFileName(name);
    setRoundIndex(0);
    setTime(parsed.rounds[0].startMs);
    setPlaying(false);
    setError(null);
  }, []);

  function chooseFile(file: File) {
    setError(null);
    setLoading(true);
    setProgress(0);
    setReplay(null);
    const request = ++requestRef.current;
    if (file.name.toLowerCase().endsWith(".json")) {
      file
        .text()
        .then((text) => {
          if (request !== requestRef.current) return;
          loadReplay(JSON.parse(text) as Replay, file.name);
        })
        .catch((cause) => setError(parseError(cause)))
        .finally(() => setLoading(false));
      return;
    }
    if (!workerRef.current) {
      workerRef.current = new Worker("/replay-parser/worker.js", { type: "module" });
    }
    const worker = workerRef.current;
    worker.onmessage = (event) => {
      if (request !== requestRef.current) return;
      const data = event.data ?? {};
      if (data.ev === "progress") setProgress(data.fraction ?? 0);
      if (data.ev === "done") {
        try {
          const parsed = JSON.parse(data.json) as Replay;
          loadReplay(parsed, file.name);
        } catch (cause) {
          setError(parseError(cause));
        } finally {
          setLoading(false);
        }
      }
      if (data.ev === "error") {
        setError(data.message || "The replay parser failed.");
        setLoading(false);
      }
      if (data.ev === "bootError") {
        setError(`Replay parser failed to start: ${data.message}`);
        setLoading(false);
      }
    };
    worker.onerror = (event) => {
      setError(event.message || "The replay worker crashed.");
      setLoading(false);
    };
    file
      .arrayBuffer()
      .then((buffer) => {
        if (request !== requestRef.current) return;
        worker.postMessage({ id: request, cmd: "parse", vrf: new Uint8Array(buffer) });
      })
      .catch((cause) => {
        setError(parseError(cause));
        setLoading(false);
      });
  }

  useEffect(() => {
    if (!playing || !round) return;
    lastFrame.current = performance.now();
    function tick(now: number) {
      const delta = (now - lastFrame.current) * speed;
      lastFrame.current = now;
      setTime((current) => {
        const next = current + delta;
        if (next >= round!.endMs) {
          setPlaying(false);
          return round!.endMs;
        }
        return next;
      });
      frameRef.current = requestAnimationFrame(tick);
    }
    frameRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frameRef.current);
  }, [playing, round, speed]);

  const positions = useMemo(() => {
    if (!round || !meta) return [];
    return replay!.players.map((player) => {
      const track = trackAt(round.tracks[player.id], time);
      if (!track) return null;
      return {
        player,
        ...worldToPercent(track[0], track[1], meta),
        angle: track[2],
        hp: hpAt(round.states, player.id, time),
      };
    }).filter(Boolean) as {
      player: ReplayPlayer;
      left: number;
      top: number;
      angle: number;
      hp: number;
    }[];
  }, [meta, replay, round, time]);

  const utility = useMemo(() => {
    if (!round || !meta) return [];
    return (round.utility ?? [])
      .filter((item) => item.t <= time && item.end >= time)
      .map((item) => ({
        ...item,
        ...worldToPercent(item.x, item.y, meta),
      }));
  }, [meta, round, time]);

  const recentEvents = useMemo(() => {
    if (!round) return [];
    return round.events
      .filter((event) => event.t <= time && ["death", "plant", "defuse", "explode"].includes(event.type))
      .slice(-7)
      .reverse();
  }, [round, time]);

  const currentScore =
    replay && round
      ? teamScore(replay, time >= round.endMs ? roundIndex : roundIndex - 1)
      : {};
  const playersByTeam = useMemo(() => {
    const groups: Record<string, ReplayPlayer[]> = {};
    for (const player of replay?.players ?? []) {
      groups[player.team ?? "Red"] = [...(groups[player.team ?? "Red"] ?? []), player];
    }
    return groups;
  }, [replay]);

  function openInPlanner() {
    if (!round || !meta || !replay) return;
    const objects = [
      ...positions.map((position) => ({
        id: `replay-player-${position.player.id}`,
        type: "text" as const,
        x: position.left,
        y: position.top,
        x2: position.left,
        y2: position.top,
        color: TEAM_COLORS[position.player.team] ?? "#f4f0e6",
        step: 1,
        text: agentName(position.player.agent),
        team: position.player.team === "Blue" ? ("ally" as const) : ("enemy" as const),
      })),
      ...utility.map((item, index) => ({
        id: `replay-utility-${index}-${item.t}`,
        type: "circle" as const,
        x: item.left,
        y: item.top,
        x2: item.left,
        y2: item.top,
        color: /smoke|cloud|nebula|pit|cove/i.test(item.name) ? "#8892a6" : "#ff4655",
        step: 1,
        radius: /smoke|cloud|nebula|pit|cove/i.test(item.name) ? 5 : 2.5,
        ability: item.name,
        team: "enemy" as const,
      })),
    ];
    const payload = {
      map: mapName(replay.mapUrl),
      side: "attack",
      objects,
      steps: 1,
      name: `${mapName(replay.mapUrl)} replay round ${roundIndex + 1}`,
    };
    localStorage.setItem("valotrack-strategy-import", JSON.stringify(payload));
    location.hash = "strategy";
  }

  return (
    <div className="page replays-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <FileVideo2 size={12} /> replay viewer
          </span>
          <h1>Watch it from above.</h1>
          <p>
            Drop a VALORANT .vrf replay and step through every round top-down:
            movement, health, kills, utility and the spike. Parsed locally.
          </p>
        </div>
        {replay && (
          <div className="head-status">
            <span className="checked">{replay.rounds.length} rounds · {formatClock(replay.lengthMs)}</span>
            <button
              className="button small"
              onClick={() => {
                setReplay(null);
                setFileName("");
                setError(null);
              }}
            >
              <RotateCcw size={13} /> New replay
            </button>
          </div>
        )}
      </header>

      {!replay && (
        <label
          className="replay-drop"
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => {
            event.preventDefault();
            const file = event.dataTransfer.files[0];
            if (file) chooseFile(file);
          }}
        >
          <input
            type="file"
            accept=".vrf,.json"
            onChange={(event) => {
              const file = event.target.files?.[0];
              if (file) chooseFile(file);
              event.target.value = "";
            }}
          />
          {loading ? (
            <>
              <Loader2 size={30} className="spin" />
              <h2>Parsing replay</h2>
              <p>{Math.round(progress * 100)}% · {fileName}</p>
              <div className="progress-bar">
                <i style={{ width: `${progress * 100}%` }} />
              </div>
            </>
          ) : (
            <>
              <Upload size={30} />
              <h2>Drop a .vrf replay</h2>
              <p>
                VALORANT saves replays in{" "}
                <code>%LOCALAPPDATA%\VALORANT\Saved\Demos</code>. Choose one or drop it
                here. Nothing is uploaded.
              </p>
              <span className="button primary">Choose replay file</span>
            </>
          )}
        </label>
      )}

      {error && (
        <div className="error-banner">
          <AlertTriangle size={16} />
          <div>
            <strong>Replay unavailable</strong>
            <span>{error}</span>
          </div>
        </div>
      )}

      {replay && round && (
        <>
          <div className="replay-head">
            <div className="replay-map-meta">
              {meta?.displayIcon && <img src={meta.displayIcon} alt="" />}
              <div>
                <span className="profile-kicker">
                  {modeLabel(replay.matchInfo?.mode)} · {replay.build ?? "replay"}
                </span>
                <h2>{mapName(replay.mapUrl)}</h2>
                <p>
                  {replay.matchInfo?.startTime
                    ? new Date(replay.matchInfo.startTime).toLocaleDateString()
                    : fileName}{" "}
                  · round {roundIndex + 1} of {replay.rounds.length}
                </p>
              </div>
            </div>
            <div className="replay-score">
              {["Red", "Blue"].map((team) => (
                <div key={team} style={{ color: TEAM_COLORS[team] }}>
                  <span>{team}</span>
                  <strong>{currentScore[team] ?? 0}</strong>
                </div>
              ))}
              <em>round {roundIndex + 1}</em>
            </div>
            <div className="replay-tabs">
              <button className={tab === "overview" ? "active" : ""} onClick={() => setTab("overview")}>
                Map
              </button>
              <button className={tab === "scoreboard" ? "active" : ""} onClick={() => setTab("scoreboard")}>
                Scoreboard
              </button>
            </div>
          </div>

          {tab === "overview" ? (
            <div className="replay-layout">
              <section className="panel replay-board-panel">
                <div className="replay-board">
                  {meta?.displayIcon && <img src={meta.displayIcon} alt={mapName(replay.mapUrl)} draggable={false} />}
                  {showCallouts &&
                    meta?.callouts.map(([name, x, y]) => (
                      <span key={name} className="replay-callout" style={worldToPercent(x, y, meta)}>
                        {name}
                      </span>
                    ))}
                  {utility.map((item, index) => (
                    <span
                      key={`${item.t}-${index}`}
                      className={cn("replay-utility", /smoke|cloud|nebula|pit|cove/i.test(item.name) && "smoke")}
                      style={{ left: `${item.left}%`, top: `${item.top}%` }}
                      title={item.name}
                    />
                  ))}
                  {positions.map(({ player, left, top, angle, hp }) => {
                    const agent = agentMap.get(agentName(player.agent));
                    return (
                      <span
                        key={player.id}
                        className={cn("replay-player", hp <= 0 && "dead")}
                        style={{
                          left: `${left}%`,
                          top: `${top}%`,
                          "--team": TEAM_COLORS[player.team] ?? "#fff",
                        } as CSSProperties}
                        title={`${agentName(player.agent)} · ${Math.round(hp)} HP`}
                      >
                        {agent?.portrait ? (
                          <img src={agent.portrait} alt={agentName(player.agent)} />
                        ) : (
                          <span className="replay-player-letter">{agentName(player.agent).slice(0, 2)}</span>
                        )}
                        <i style={{ transform: `rotate(${angle}deg)` }} />
                        {hp > 0 && <b style={{ width: `${Math.max(0, Math.min(100, hp))}%` }} />}
                      </span>
                    );
                  })}
                  {recentEvents
                    .filter((event) => event.x !== undefined && event.y !== undefined)
                    .slice(0, 4)
                    .map((event, index) => (
                      <span
                        key={`${event.t}-${index}`}
                        className={cn("replay-event", event.type)}
                        style={worldToPercent(event.x ?? 0, event.y ?? 0, meta!)}
                        title={`${event.type}${event.site ? ` at ${event.site}` : ""}`}
                      />
                    ))}
                  {!meta && (
                    <div className="replay-no-map">
                      <AlertTriangle size={18} />
                      <span>Map not recognised - positions use world coordinates only.</span>
                    </div>
                  )}
                  <div className="replay-map-overlay">
                    <span>{formatClock(time - round.startMs)}</span>
                    <label>
                      <input
                        type="checkbox"
                        checked={showCallouts}
                        onChange={(event) => setShowCallouts(event.target.checked)}
                      />
                      callouts
                    </label>
                  </div>
                </div>

                <div className="replay-controls">
                  <button
                    className="icon-button"
                    onClick={() => {
                      setRoundIndex((index) => Math.max(0, index - 1));
                      setTime(replay.rounds[Math.max(0, roundIndex - 1)].startMs);
                    }}
                    disabled={roundIndex === 0}
                  >
                    <SkipBack size={14} />
                  </button>
                  <button
                    className="replay-play"
                    onClick={() => {
                      if (!playing && time >= round.endMs) setTime(round.startMs);
                      setPlaying((value) => !value);
                    }}
                  >
                    {playing ? <Pause size={16} /> : <Play size={16} />}
                  </button>
                  <button
                    className="icon-button"
                    onClick={() => {
                      const next = Math.min(replay.rounds.length - 1, roundIndex + 1);
                      setRoundIndex(next);
                      setTime(replay.rounds[next].startMs);
                    }}
                    disabled={roundIndex >= replay.rounds.length - 1}
                  >
                    <SkipForward size={14} />
                  </button>
                  <input
                    className="replay-scrub"
                    type="range"
                    min={round.startMs}
                    max={round.endMs}
                    value={time}
                    onChange={(event) => {
                      setPlaying(false);
                      setTime(Number(event.target.value));
                    }}
                  />
                  <span className="replay-time">
                    {formatClock(time - round.startMs)} / {formatClock(round.endMs - round.startMs)}
                  </span>
                  <select
                    className="small-select speed-select"
                    value={speed}
                    onChange={(event) => setSpeed(Number(event.target.value))}
                  >
                    <option value={0.5}>0.5×</option>
                    <option value={1}>1×</option>
                    <option value={2}>2×</option>
                    <option value={4}>4×</option>
                  </select>
                  <button className="button small" onClick={openInPlanner}>
                    <Layers size={12} /> Planner
                  </button>
                </div>
              </section>

              <aside className="replay-side">
                <section className="panel">
                  <div className="panel-heading compact-heading">
                    <div>
                      <span className="panel-label">Round</span>
                      <h2>{roundIndex + 1}</h2>
                    </div>
                    <span className="checked">{round.result ?? "in progress"}</span>
                  </div>
                  <div className="round-list">
                    {replay.rounds.map((item, index) => {
                      const winner = item.winner;
                      return (
                        <button
                          key={item.round}
                          className={cn("round-select", index === roundIndex && "active")}
                          onClick={() => {
                            setRoundIndex(index);
                            setTime(item.startMs);
                            setPlaying(false);
                          }}
                          style={{
                            borderColor: winner ? `${TEAM_COLORS[winner]}55` : undefined,
                          }}
                        >
                          <span>{index + 1}</span>
                          {winner && <i style={{ background: TEAM_COLORS[winner] }} />}
                        </button>
                      );
                    })}
                  </div>
                  <div className="replay-player-list">
                    {Object.entries(playersByTeam).map(([team, players]) => (
                      <div key={team}>
                        <span className="team-tag" style={{ color: TEAM_COLORS[team] }}>{team}</span>
                        {players.map((player) => {
                          const position = positions.find((item) => item.player.id === player.id);
                          const agent = agentMap.get(agentName(player.agent));
                          return (
                            <div className="replay-player-row" key={player.id}>
                              <span className="replay-agent-mini">
                                {agent?.portrait ? <img src={agent.portrait} alt="" /> : null}
                              </span>
                              <span>
                                <strong>{agentName(player.agent)}</strong>
                                <em>{Math.round(position?.hp ?? 0)} HP</em>
                              </span>
                              <b style={{ color: TEAM_COLORS[team] }}>
                                {position && position.hp > 0 ? "alive" : "down"}
                              </b>
                            </div>
                          );
                        })}
                      </div>
                    ))}
                  </div>
                </section>

                <section className="panel">
                  <div className="panel-heading compact-heading">
                    <div>
                      <span className="panel-label">Feed</span>
                      <h2>Round events</h2>
                    </div>
                  </div>
                  <div className="event-feed">
                    {recentEvents.length ? (
                      recentEvents.map((event, index) => (
                        <div className="event-row" key={`${event.t}-${index}`}>
                          <span className={cn("event-pip", event.type)} />
                          <span>
                            <strong>
                              {event.type === "death"
                                ? `${agentName(
                                    replay.players.find((player) => player.id === event.killer)?.agent ?? "",
                                  )} killed ${agentName(
                                    replay.players.find((player) => player.id === event.victim)?.agent ?? "",
                                  )}`
                                : event.type === "plant"
                                  ? `Spike planted${event.site ? ` at ${event.site}` : ""}`
                                  : event.type === "defuse"
                                    ? "Spike defused"
                                    : "Spike detonated"}
                            </strong>
                            <em>{formatClock(event.t - round.startMs)}</em>
                          </span>
                          {event.headshot && <b>HS</b>}
                        </div>
                      ))
                    ) : (
                      <p className="quiet">No events yet at this timestamp.</p>
                    )}
                  </div>
                </section>
              </aside>
            </div>
          ) : (
            <section className="panel replay-scoreboard">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Full match</span>
                  <h2>Scoreboard</h2>
                </div>
                <Crosshair size={19} className="panel-icon" />
              </div>
              <div className="replay-scoreboard-grid">
                {Object.entries(playersByTeam).map(([team, players]) => (
                  <div key={team}>
                    <h3 style={{ color: TEAM_COLORS[team] }}>{team}</h3>
                    <div className="scoreboard-columns">
                      <span>Player</span>
                      <span>K</span>
                      <span>D</span>
                      <span>A</span>
                      <span>ACS</span>
                      <span>ADR</span>
                      <span>HS</span>
                      <span>FK</span>
                      <span>ACE</span>
                    </div>
                    {players.map((player) => (
                      <div className="scoreboard-replay-row" key={player.id}>
                        <span>{agentName(player.agent)}</span>
                        <span>{statText(player, "kills")}</span>
                        <span>{statText(player, "deaths")}</span>
                        <span>{statText(player, "assists")}</span>
                        <span>{statText(player, "acs")}</span>
                        <span>{statText(player, "adr")}</span>
                        <span>{statText(player, "hsPct")}</span>
                        <span>{statText(player, "firstKills")}</span>
                        <span>{statText(player, "aces")}</span>
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            </section>
          )}
        </>
      )}
    </div>
  );
}
