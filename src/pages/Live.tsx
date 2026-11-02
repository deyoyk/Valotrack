import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
  Clock3,
  Cpu,
  Gamepad2,
  Loader2,
  Radio,
  RefreshCw,
  ShieldCheck,
  WifiOff,
} from "lucide-react";
import { api, Agent, CareerMatch, Health, LiveBoard, Player } from "../lib/api";
import { buildDemoBoard } from "../lib/demo";
import { cn } from "../lib/format";
import { usePoll } from "../lib/usePoll";
import { Scoreboard } from "../components/Scoreboard";
import { ProfileModal } from "../components/ProfileModal";
import { MatchDetailModal } from "../components/MatchDetailModal";

export default function Live() {
  const [health, setHealth] = useState<Health | null>(null);
  const [board, setBoard] = useState<LiveBoard | null>(null);
  const [selected, setSelected] = useState<Player | null>(null);
  const [selectedMatch, setSelectedMatch] = useState<CareerMatch | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [checkedAt, setCheckedAt] = useState<number>(Date.now());
  const [demo, setDemo] = useState(false);
  const [agents, setAgents] = useState<Agent[]>([]);

  const refresh = useCallback(async (quiet = false) => {
    if (!quiet) setRefreshing(true);
    try {
      const nextHealth = await api.health();
      setHealth(nextHealth);
      if (nextHealth.clientAvailable) {
        const nextBoard = await api.liveBoard();
        setBoard(nextBoard);
        setError(null);
      } else {
        setBoard(null);
        setError(null);
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
      setRefreshing(false);
      setCheckedAt(Date.now());
    }
  }, []);

  useEffect(() => {
    void refresh(true);
  }, [refresh]);

  useEffect(() => {
    api.agents().then((catalog) => setAgents(catalog.agents)).catch(() => void 0);
  }, []);

  usePoll(() => refresh(true), (health?.clientAvailable ? 4 : 6) * 1000);

  const connected = health?.clientAvailable ?? false;
  const demoBoard = demo ? buildDemoBoard(agents) : null;
  const shownBoard = demoBoard ?? board;

  return (
    <div className="page live-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Radio size={12} /> live intelligence
          </span>
          <h1>Know the lobby.</h1>
          <p>
            Ranks, parties, recent form and smurf flags pulled from your own Riot
            client. Nothing leaves this machine.
          </p>
        </div>
        <div className="head-status">
          <div className={connected ? "status-pill online" : "status-pill"}>
            <span className="dot" />
            {connected ? "Riot client online" : "Riot client offline"}
          </div>
          <span className="checked">
            <Clock3 size={12} />
            checked {new Date(checkedAt).toLocaleTimeString()}
          </span>
          <div className="head-buttons">
            <button
              className={cn("button small", demo && "active")}
              onClick={() => setDemo((value) => !value)}
            >
              {demo ? "Exit demo" : "Preview demo lobby"}
            </button>
          </div>
        </div>
      </header>

      {loading && !health && (
        <div className="center-state">
          <Loader2 size={22} className="spin" />
          <span>Connecting to the local client</span>
        </div>
      )}

      {error && (
        <div className="error-banner">
          <AlertTriangle size={17} />
          <div>
            <strong>Could not read game data</strong>
            <span>{error}</span>
          </div>
          <button className="button small" onClick={() => refresh()}>
            Retry
          </button>
        </div>
      )}

      {!connected && health && !demo && (
        <section className="offline-grid">
          <div className="connect-card">
            <div className="connect-glow" />
            <WifiOff size={28} />
            <span className="eyebrow">waiting for valorant</span>
            <h2>Open the game to go live.</h2>
            <p>
              Valotrack reads the Riot Client locally. Once VALORANT is signed in,
              your lobby and match scoreboard appear here automatically.
            </p>
            <button className="button primary" onClick={() => refresh()}>
              <RefreshCw size={15} /> Check again
            </button>
            <div className="connect-steps">
              <div>
                <span>1</span>
                Launch VALORANT
              </div>
              <div>
                <span>2</span>
                Sign in and reach the lobby
              </div>
              <div>
                <span>3</span>
                Queue up - intel loads itself
              </div>
            </div>
          </div>

          <div className="client-card">
            <span className="panel-label">Client detection</span>
            <div className="client-row">
              <Gamepad2 size={16} />
              <div>
                <strong>Riot Client</strong>
                <span>{connected ? "running" : "not running"}</span>
              </div>
              <i className={connected ? "state-dot on" : "state-dot"} />
            </div>
            <div className="client-row">
              <Cpu size={16} />
              <div>
                <strong>Region</strong>
                <span>{health.region ?? "detected from game logs"}</span>
              </div>
            </div>
            <div className="client-row">
              <ShieldCheck size={16} />
              <div>
                <strong>Data path</strong>
                <span className="path">{health.lockfilePath}</span>
              </div>
            </div>
            <div className="privacy-note">
              <ShieldCheck size={14} />
              Local reads only. No account, no cloud, no telemetry.
            </div>
          </div>
        </section>
      )}

      {shownBoard && (
        <Scoreboard
          board={shownBoard}
          onRefresh={() => refresh()}
          refreshing={refreshing}
          onPlayer={setSelected}
        />
      )}

      {selected && (
        <ProfileModal
          player={selected}
          onClose={() => setSelected(null)}
          onMatch={setSelectedMatch}
          demo={Boolean(demoBoard)}
          agents={agents}
        />
      )}

      {selectedMatch && (
        <MatchDetailModal
          matchId={selectedMatch.matchId}
          subject={selected?.puuid}
          title={`${selectedMatch.map} · ${selectedMatch.mode}`}
          onClose={() => setSelectedMatch(null)}
          demo={Boolean(demoBoard)}
          agents={agents}
        />
      )}

      {!shownBoard && !loading && connected && (
        <div className="center-state">
          <Loader2 size={20} className="spin" />
          <span>Waiting for the client session</span>
        </div>
      )}
    </div>
  );
}
