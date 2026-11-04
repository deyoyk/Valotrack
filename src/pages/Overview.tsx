import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  ArrowRight,
  CalendarClock,
  Crosshair,
  LayoutDashboard,
  Loader2,
  Radio,
  ShoppingBag,
  Swords,
  Trophy,
  Wallet,
} from "lucide-react";
import {
  api,
  Health,
  Inventory,
  LiveBoard,
  Performance,
  SessionList,
  StoreSnapshot,
} from "../lib/api";
import { cn, pct } from "../lib/format";
import { MatchDetailModal } from "../components/MatchDetailModal";

function formStats(form: string[]) {
  return {
    wins: form.filter((result) => result === "Victory").length,
    losses: form.filter((result) => result === "Defeat").length,
    draws: form.filter((result) => result === "Draw").length,
  };
}

function remaining(seconds: number | null | undefined) {
  if (!seconds) return "-";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${minutes}m`;
}

export default function Overview() {
  const [health, setHealth] = useState<Health | null>(null);
  const [performance, setPerformance] = useState<Performance | null>(null);
  const [inventory, setInventory] = useState<Inventory | null>(null);
  const [sessions, setSessions] = useState<SessionList | null>(null);
  const [store, setStore] = useState<StoreSnapshot | null>(null);
  const [board, setBoard] = useState<LiveBoard | null>(null);
  const [selectedMatch, setSelectedMatch] = useState<{ id: string; title: string } | null>(
    null,
  );
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    Promise.allSettled([
      api.health(),
      api.performance(),
      api.inventory(),
      api.sessions(),
      api.storeSnapshot(),
      api.liveBoard(),
    ]).then(([healthResult, performanceResult, inventoryResult, sessionsResult, storeResult, boardResult]) => {
      if (cancelled) return;
      if (healthResult.status === "fulfilled") setHealth(healthResult.value);
      if (performanceResult.status === "fulfilled") setPerformance(performanceResult.value);
      if (inventoryResult.status === "fulfilled") setInventory(inventoryResult.value);
      if (sessionsResult.status === "fulfilled") setSessions(sessionsResult.value);
      if (storeResult.status === "fulfilled") setStore(storeResult.value);
      if (boardResult.status === "fulfilled") setBoard(boardResult.value);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const recent = performance?.recent.slice(0, 6) ?? [];
  const form = recent.map((match) => match.result).reverse();
  const formTally = formStats(form);
  const draws = performance?.summary.draws ?? 0;
  const currentSession = sessions?.active ?? null;
  const liveLabel =
    board?.map && board.state !== "MENUS"
      ? `${board.map} · ${board.score ? `${board.score.ally}:${board.score.enemy}` : board.stateLabel}`
      : health?.clientAvailable
        ? "Client online"
        : "Client offline";

  const greeting = useMemo(() => {
    const hour = new Date().getHours();
    if (hour < 5) return "Late night climb.";
    if (hour < 12) return "Morning warm-up.";
    if (hour < 18) return "Ready for a round.";
    return "One more game.";
  }, []);

  return (
    <div className="page overview-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <LayoutDashboard size={12} /> overview
          </span>
          <h1>{greeting}</h1>
          <p>
            Your rank, recent matches, session and collection in one place. Everything is
            read from your own Riot client and kept on this machine.
          </p>
        </div>
        <div className="head-status">
          <span className={health?.clientAvailable ? "status-pill online" : "status-pill"}>
            <span className="dot" />
            {liveLabel}
          </span>
          <span className="checked">{store?.account ?? "local account"}</span>
        </div>
      </header>

      {loading ? (
        <div className="center-state">
          <Loader2 className="spin" size={20} />
          <span>Reading your dashboard</span>
        </div>
      ) : (
        <>
          <div className="overview-grid">
            <div className="panel overview-kpi">
              <span>Current rank</span>
              <strong style={{ color: performance?.rankColor }}>
                {performance?.rank ?? "Unranked"}
              </strong>
              <em>
                {performance?.rr ?? 0} RR · {performance?.summary.games ?? 0} ranked games
              </em>
            </div>
            <div className="panel overview-kpi">
              <span>Recent win rate</span>
              <strong>{pct(performance?.summary.winRate ?? 0)}</strong>
              <em>
                {performance?.summary.wins ?? 0}W · {performance?.summary.losses ?? 0}L
                {draws > 0 ? ` · ${draws}D` : ""}
              </em>
            </div>
            <div className="panel overview-kpi">
              <span>Collection value</span>
              <strong>{(inventory?.totalVp ?? 0).toLocaleString()}</strong>
              <em>VP · about ${(inventory?.usdApprox ?? 0).toLocaleString()}</em>
            </div>
            <div className="panel overview-kpi">
              <span>Session</span>
              <strong>{currentSession ? `${currentSession.games}g` : "None"}</strong>
              <em>
                {currentSession
                  ? `${currentSession.wins}W ${currentSession.losses}L · ${currentSession.netRr > 0 ? "+" : ""}${currentSession.netRr} RR`
                  : "Set a target on Performance"}
              </em>
            </div>
          </div>

          <div className="overview-columns">
            <section className="panel">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Match history</span>
                  <h2>Recent ranked</h2>
                </div>
                <button className="button small" onClick={() => (location.hash = "performance")}>
                  All matches <ArrowRight size={13} />
                </button>
              </div>

              {recent.length ? (
                <div className="overview-list">
                  {recent.map((match) => (
                    <button
                      className="overview-match"
                      key={match.matchId}
                      onClick={() =>
                        setSelectedMatch({ id: match.matchId, title: `${match.map} · ${match.result}` })
                      }
                    >
                      <span
                        className={cn(
                          "result-dot",
                          match.result === "Victory"
                            ? "win"
                            : match.result === "Defeat"
                              ? "loss"
                              : "draw",
                        )}
                        title={match.result}
                      >
                        {match.result === "Victory" ? "W" : match.result === "Defeat" ? "L" : "D"}
                      </span>
                      <span>
                        <strong>{match.map}</strong>
                        <span>{new Date(match.ts * 1000).toLocaleDateString()}</span>
                      </span>
                      <span className="tnum">
                        {match.tier > 0 ? `${match.rr} RR` : "Unranked"}
                      </span>
                      <span className={cn("rr-delta", match.delta >= 0 ? "up" : "down")}>
                        {match.delta > 0 ? "+" : ""}
                        {match.delta} RR
                      </span>
                      <ArrowRight size={14} />
                    </button>
                  ))}
                </div>
              ) : (
                <div className="chart-empty">
                  <Trophy size={20} />
                  <span>No ranked matches recorded yet.</span>
                </div>
              )}

              <div className="overview-form">
                <span className="panel-label">Recent form</span>
                <div className="overview-form-dots">
                  {form.length ? (
                    form.map((result, index) => (
                      <i
                        key={index}
                        className={
                          result === "Victory"
                            ? "win"
                            : result === "Defeat"
                              ? "loss"
                              : "draw"
                        }
                        title={result}
                      >
                        {result === "Victory" ? "W" : result === "Defeat" ? "L" : "D"}
                      </i>
                    ))
                  ) : (
                    <em>No form yet</em>
                  )}
                </div>
                <span className="overview-form-note">
                  {form.length
                    ? `${formTally.wins}W · ${formTally.losses}L${
                        formTally.draws > 0 ? ` · ${formTally.draws}D` : ""
                      } in the last ${form.length}`
                    : "Play a ranked match to build form"}
                </span>
              </div>
            </section>

            <aside className="overview-side">
              <section className="panel">
                <div className="panel-heading">
                  <div>
                    <span className="panel-label">Quick actions</span>
                    <h2>Jump back in</h2>
                  </div>
                  <Swords size={19} className="panel-icon" />
                </div>
                <div className="overview-quick">
                  <button onClick={() => (location.hash = "live")}>
                    <Radio size={18} />
                    <strong>Live lobby</strong>
                    <span>Ranks, parties and smurf flags</span>
                  </button>
                  <button onClick={() => (location.hash = "matchmaking")}>
                    <Swords size={18} />
                    <strong>Matchmaking</strong>
                    <span>Queue, instalock and side check</span>
                  </button>
                  <button onClick={() => (location.hash = "crosshairs")}>
                    <Crosshair size={18} />
                    <strong>Crosshairs</strong>
                    <span>209 pro and community codes</span>
                  </button>
                  <button onClick={() => (location.hash = "collection")}>
                    <Wallet size={18} />
                    <strong>Collection</strong>
                    <span>{inventory?.counts.skins ?? 0} skins owned</span>
                  </button>
                </div>
              </section>

              <section className="panel">
                <div className="panel-heading">
                  <div>
                    <span className="panel-label">Today</span>
                    <h2>At a glance</h2>
                  </div>
                  <Activity size={19} className="panel-icon" />
                </div>
                <div className="overview-facts">
                  <div>
                    <span>Wallet</span>
                    <strong>{(inventory?.wallet.vp ?? 0).toLocaleString()} VP</strong>
                    <em>{(inventory?.wallet.kc ?? 0).toLocaleString()} KC</em>
                  </div>
                  <div>
                    <span>Daily store</span>
                    <strong className="inline-icon">
                      <ShoppingBag size={14} /> {store?.daily.offers.length ?? 0} offers
                    </strong>
                    <em>{remaining(store?.daily.remaining)}</em>
                  </div>
                  <div>
                    <span>Streak</span>
                    <strong>{performance?.summary.streak ? `${performance.summary.streak.count}${performance.summary.streak.type}` : "-"}</strong>
                    <em>{performance?.summary.streak ? "current run" : "no active streak"}</em>
                  </div>
                  <div>
                    <span>Maps tracked</span>
                    <strong>{performance?.summary.maps.length ?? 0}</strong>
                    <em>this season</em>
                  </div>
                </div>
                <button className="button small overview-calendar" onClick={() => (location.hash = "performance")}>
                  <CalendarClock size={13} /> Open rank history
                </button>
              </section>
            </aside>
          </div>
        </>
      )}

      {selectedMatch && (
        <MatchDetailModal
          matchId={selectedMatch.id}
          title={selectedMatch.title}
          onClose={() => setSelectedMatch(null)}
        />
      )}
    </div>
  );
}
