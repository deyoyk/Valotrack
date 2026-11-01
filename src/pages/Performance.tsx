import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  AlertTriangle,
  ArrowDownRight,
  ArrowUpRight,
  BarChart3,
  CalendarDays,
  Flag,
  Flame,
  Loader2,
  Play,
  RefreshCw,
  ShieldCheck,
  Square,
  Target,
  Trash2,
  Trophy,
} from "lucide-react";
import { api, type Performance as PerformanceData } from "../lib/api";
import { cn, formatDate, formatDateTime, num, pct } from "../lib/format";
import { Modal } from "../components/Modal";

function TrendChart({ points }: { points: PerformanceData["points"] }) {
  const rows = useMemo(() => (points ?? []).slice(-80), [points]);
  if (rows.length < 2) {
    return (
      <div className="chart-empty">
        <BarChart3 size={22} />
        <span>Two ranked games are needed to draw the trend.</span>
      </div>
    );
  }
  const values = rows.map((point) => point.rr ?? 0);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = Math.max(30, max - min);
  const width = 820;
  const height = 200;
  const pad = 14;
  const x = (index: number) => pad + (index / (rows.length - 1)) * (width - pad * 2);
  const y = (value: number) =>
    height - pad - ((value - (min - span * 0.1)) / (span * 1.2)) * (height - pad * 2);
  const line = rows.map((row, index) => `${index ? "L" : "M"}${x(index).toFixed(1)},${y(row.rr ?? 0).toFixed(1)}`).join(" ");
  const area = `${line} L${x(rows.length - 1)},${height} L${x(0)},${height} Z`;
  const last = values[values.length - 1];
  const first = values[0];
  const delta = last - first;

  return (
    <div className="trend-chart">
      <div className="trend-summary">
        <div>
          <span>Current RR</span>
          <strong>{last}</strong>
        </div>
        <div>
          <span>Window change</span>
          <strong className={delta >= 0 ? "pos" : "neg"}>
            {delta > 0 ? "+" : ""}
            {delta} RR
          </strong>
        </div>
        <div>
          <span>Games</span>
          <strong>{rows.length}</strong>
        </div>
      </div>
      <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" role="img" aria-label="RR trend">
        <defs>
          <linearGradient id="rrFill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#ff4655" stopOpacity="0.25" />
            <stop offset="100%" stopColor="#ff4655" stopOpacity="0" />
          </linearGradient>
        </defs>
        {[0.25, 0.5, 0.75].map((ratio) => (
          <line
            key={ratio}
            x1={pad}
            x2={width - pad}
            y1={height * ratio}
            y2={height * ratio}
            stroke="rgba(255,255,255,0.045)"
            strokeWidth="1"
          />
        ))}
        <path d={area} fill="url(#rrFill)" />
        <path d={line} fill="none" stroke="#ff4655" strokeWidth="1.8" vectorEffect="non-scaling-stroke" />
        {rows.map((row, index) => (
          <circle
            key={row.matchId}
            cx={x(index)}
            cy={y(row.rr ?? 0)}
            r={row.result === "Victory" ? 2 : 1.4}
            fill={row.result === "Victory" ? "#49d6a3" : "#5b6270"}
            opacity={index === rows.length - 1 ? 1 : 0.6}
          />
        ))}
      </svg>
      <div className="trend-axis">
        <span>{formatDate(rows[0].ts)}</span>
        <span>{formatDate(rows[rows.length - 1].ts)}</span>
      </div>
    </div>
  );
}

export default function Performance() {
  const [data, setData] = useState<PerformanceData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [startOpen, setStartOpen] = useState(false);
  const [goalType, setGoalType] = useState("games");
  const [goalTarget, setGoalTarget] = useState(5);
  const [sessionBusy, setSessionBusy] = useState(false);

  async function load(quiet = false) {
    if (!quiet) setRefreshing(true);
    try {
      const next = await api.performance();
      setData(next);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }

  useEffect(() => {
    void load(true);
  }, []);

  if (loading) {
    return (
      <div className="page">
        <div className="center-state">
          <Loader2 className="spin" size={20} />
          <span>Reading rank history</span>
        </div>
      </div>
    );
  }

  const summary = data?.summary;
  const tilt = summary?.tilt;
  const hasGames = (summary?.games ?? 0) > 0;
  const manual = data?.manualSessions;
  const activeSession = manual?.active ?? null;

  async function startSession() {
    setSessionBusy(true);
    try {
      await api.sessionStart(goalType, goalTarget);
      setStartOpen(false);
      await load(true);
    } finally {
      setSessionBusy(false);
    }
  }

  async function endSession() {
    setSessionBusy(true);
    try {
      await api.sessionEnd();
      await load(true);
    } finally {
      setSessionBusy(false);
    }
  }

  return (
    <div className="page performance-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Activity size={12} /> performance
          </span>
          <h1>Your climb.</h1>
          <p>
            Ranked RR history, streaks, map splits and tilt signals stored on this
            machine and refreshed from your competitive updates.
          </p>
        </div>
        <div className="head-status">
          {data?.owner ? (
            <div className="rank-badge" style={{ color: data.rankColor }}>
              <Trophy size={13} />
              <strong>{data.rank}</strong>
              <span>{data.rr} RR</span>
            </div>
          ) : (
            <div className="status-pill">
              <span className="dot" />
              no account saved
            </div>
          )}
          <button className="button small" onClick={() => load()} disabled={refreshing}>
            <RefreshCw size={13} className={refreshing ? "spin" : ""} /> Refresh
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <AlertTriangle size={16} />
          <div>
            <strong>Could not refresh performance</strong>
            <span>{error}</span>
          </div>
        </div>
      )}

      <section className={cn("session-banner", activeSession && "active")}>
        {activeSession ? (
          <>
            <div className="session-banner-icon">
              <Flag size={18} />
            </div>
            <div className="session-banner-main">
              <span className="profile-kicker">session in progress</span>
              <h2>
                {activeSession.current} / {activeSession.target}{" "}
                {activeSession.goalType === "wins"
                  ? "wins"
                  : activeSession.goalType === "rr"
                    ? "RR"
                    : "games"}
              </h2>
              <div className="session-progress">
                <i style={{ width: `${activeSession.progress}%` }} />
              </div>
              <p>
                {activeSession.games} games · {activeSession.wins}W {activeSession.losses}L ·{" "}
                <span className={activeSession.netRr >= 0 ? "pos" : "neg"}>
                  {activeSession.netRr > 0 ? "+" : ""}
                  {activeSession.netRr} RR
                </span>
              </p>
            </div>
            <button className="button small danger" onClick={endSession} disabled={sessionBusy}>
              <Square size={12} /> End session
            </button>
          </>
        ) : (
          <>
            <div className="session-banner-icon">
              <Flag size={18} />
            </div>
            <div className="session-banner-main">
              <span className="profile-kicker">session tracker</span>
              <h2>Set a target before you queue.</h2>
              <p>Track wins, games or RR this session and see the progress live.</p>
            </div>
            <button className="button small primary" onClick={() => setStartOpen(true)}>
              <Play size={12} /> Start session
            </button>
          </>
        )}
      </section>

      {summary && (
        <div className="perf-stat-row">
          <div className="perf-stat">
            <span>Games</span>
            <strong>{summary.games}</strong>
            <em>ranked matches</em>
          </div>
          <div className="perf-stat">
            <span>Win rate</span>
            <strong>{pct(summary.winRate)}</strong>
            <em>
              {summary.wins}W · {summary.losses}L
            </em>
          </div>
          <div className="perf-stat">
            <span>Net RR</span>
            <strong className={summary.netRr >= 0 ? "pos" : "neg"}>
              {summary.netRr > 0 ? "+" : ""}
              {summary.netRr}
            </strong>
            <em>{num(summary.avgDelta, 1)} avg / game</em>
          </div>
          <div className="perf-stat">
            <span>Streak</span>
            <strong className={summary.streak?.type === "Victory" ? "pos" : summary.streak ? "neg" : ""}>
              {summary.streak ? `${summary.streak.count}${summary.streak.type === "Victory" ? "W" : "L"}` : "-"}
            </strong>
            <em>{summary.streak?.type === "Victory" ? "winning run" : summary.streak ? "losing run" : "no streak"}</em>
          </div>
          <div className={cn("perf-stat tilt", tilt?.active && "active")}>
            <span>Mental</span>
            <strong>
              {tilt?.active ? <Flame size={18} /> : <ShieldCheck size={18} />}
              {tilt?.level === "high" ? "Tilted" : tilt?.active ? "Watch it" : "Steady"}
            </strong>
            <em>{tilt?.message}</em>
          </div>
        </div>
      )}

      <div className="perf-grid">
        <section className="panel chart-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Ranked rating</span>
              <h2>RR trend</h2>
            </div>
            <span className="chart-range">last {Math.min(80, data?.points?.length ?? 0)} games</span>
          </div>
          {hasGames ? <TrendChart points={data?.points ?? []} /> : (
            <div className="chart-empty">
              <Target size={22} />
              <strong>No ranked games yet</strong>
              <span>Play a competitive match with VALORANT open to start the history.</span>
            </div>
          )}
        </section>

        <section className="panel insight-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Patterns</span>
              <h2>Insights</h2>
            </div>
            <BarChart3 size={19} className="panel-icon" />
          </div>
          {data?.insights?.length ? (
            <div className="insight-list">
              {data.insights.map((insight) => (
                <div className={cn("insight", insight.tone)} key={insight.title}>
                  <strong>{insight.title}</strong>
                  <span>{insight.text}</span>
                </div>
              ))}
            </div>
          ) : (
            <div className="chart-empty compact">
              <span>Insights unlock after 20 ranked games.</span>
            </div>
          )}
          {summary?.maps?.length ? (
            <div className="map-splits">
              <span className="panel-label">Map splits</span>
              {summary.maps.slice(0, 6).map((map) => (
                <div className="map-split" key={map.map}>
                  <span>{map.map}</span>
                  <div className="mini-bar">
                    <i
                      className={map.winRate >= 50 ? "good" : "bad"}
                      style={{ width: `${map.winRate}%` }}
                    />
                  </div>
                  <strong>{map.winRate}%</strong>
                  <em>{map.delta > 0 ? "+" : ""}{map.delta}</em>
                </div>
              ))}
            </div>
          ) : null}
        </section>

        <section className="panel sessions-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Runs</span>
              <h2>Sessions</h2>
            </div>
            <CalendarDays size={19} className="panel-icon" />
          </div>
          {data?.sessions?.length ? (
            <div className="session-list">
              {data.sessions.slice(0, 6).map((session) => (
                <div className="session-row" key={session.id}>
                  <div className="session-date">
                    <strong>{formatDate(session.start)}</strong>
                    <span>{formatDateTime(session.start).split(", ")[1] ?? ""}</span>
                  </div>
                  <div className="session-bar">
                    <span>{session.games} games</span>
                    <div className="mini-bar">
                      <i
                        className={session.winRate >= 50 ? "good" : "bad"}
                        style={{ width: `${session.winRate}%` }}
                      />
                    </div>
                  </div>
                  <strong className={session.netRr >= 0 ? "pos" : "neg"}>
                    {session.netRr > 0 ? "+" : ""}
                    {session.netRr} RR
                  </strong>
                </div>
              ))}
            </div>
          ) : (
            <div className="chart-empty compact">
              <span>Sessions group automatically when you play.</span>
            </div>
          )}
          {manual?.sessions?.length ? (
            <div className="manual-sessions">
              <span className="panel-label">Tracked sessions</span>
              {manual.sessions.slice(0, 6).map((session) => (
                <div className="manual-session-row" key={session.id}>
                  <span>
                    <strong>
                      {session.current}/{session.target}{" "}
                      {session.goalType === "wins"
                        ? "wins"
                        : session.goalType === "rr"
                          ? "RR"
                          : "games"}
                    </strong>
                    <em>{formatDate(session.startedAt)}</em>
                  </span>
                  <span className={session.netRr >= 0 ? "pos" : "neg"}>
                    {session.netRr > 0 ? "+" : ""}
                    {session.netRr} RR
                  </span>
                  <button
                    className="icon-button"
                    onClick={async () => {
                      await api.sessionDelete(session.id);
                      await load(true);
                    }}
                    aria-label="Delete session"
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              ))}
            </div>
          ) : null}
        </section>

        <section className="panel recent-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Latest</span>
              <h2>Recent ranked</h2>
            </div>
          </div>
          {data?.recent?.length ? (
            <div className="recent-list">
              {data.recent.slice(0, 10).map((point) => (
                <div className="recent-row" key={point.matchId}>
                  <span
                    className={cn(
                      "result-dot",
                      point.result === "Victory" ? "win" : point.result === "Defeat" ? "loss" : "draw",
                    )}
                  >
                    {point.result === "Victory" ? "W" : point.result === "Defeat" ? "L" : "D"}
                  </span>
                  <div>
                    <strong>{point.map}</strong>
                    <span>{formatDate(point.ts)}</span>
                  </div>
                  <span className="recent-rr">{point.rr} RR</span>
                  <span className={cn("recent-delta", point.delta >= 0 ? "pos" : "neg")}>
                    {point.delta > 0 ? <ArrowUpRight size={11} /> : <ArrowDownRight size={11} />}
                    {point.delta > 0 ? "+" : ""}
                    {point.delta}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <div className="chart-empty compact">
              <span>Recent ranked matches appear here.</span>
            </div>
          )}
        </section>
      </div>

      {startOpen && (
        <Modal onClose={() => setStartOpen(false)} label="Start session" className="session-modal">
          <div className="builder-head">
            <div>
              <span className="profile-kicker">Session tracker</span>
              <h2>Set the target</h2>
            </div>
          </div>
          <div className="session-form">
            <label className="field">
              <span>Goal</span>
              <select value={goalType} onChange={(event) => setGoalType(event.target.value)}>
                <option value="games">Play games</option>
                <option value="wins">Win games</option>
                <option value="rr">Earn RR</option>
              </select>
            </label>
            <label className="field">
              <span>Target</span>
              <input
                type="number"
                min={1}
                max={100}
                value={goalTarget}
                onChange={(event) => setGoalTarget(Number(event.target.value))}
              />
            </label>
            <div className="builder-actions">
              <button className="button" onClick={() => setStartOpen(false)}>
                Cancel
              </button>
              <button className="button primary" onClick={startSession} disabled={sessionBusy}>
                {sessionBusy ? <Loader2 size={13} className="spin" /> : <Play size={13} />}
                Start tracking
              </button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
