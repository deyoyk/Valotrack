import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
  CalendarDays,
  Check,
  Clock3,
  Globe2,
  Loader2,
  MailCheck,
  Phone,
  RefreshCw,
  ShieldCheck,
  Sparkles,
  Trophy,
  UserRound,
  Wallet,
} from "lucide-react";
import { AccountOverview, api } from "../lib/api";
import { rankColor, rankIcon, rankName } from "../lib/rank";

const SOURCES: Record<string, string> = {
  "time-played": "Time played",
  "match-win": "Match win",
  "first-win-of-the-day": "First win",
};

function formatDate(value: string | number | null) {
  if (!value) return "Not available";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  return date.toLocaleString();
}

function formatRelative(value: string | null) {
  if (!value) return "Not available";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const diff = date.getTime() - Date.now();
  if (diff <= 0) return "Available now";
  const hours = Math.floor(diff / 3_600_000);
  const minutes = Math.floor((diff % 3_600_000) / 60_000);
  return `${hours}h ${minutes}m`;
}

export default function Account() {
  const [data, setData] = useState<AccountOverview | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setData(await api.accountOverview());
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (!data && !error) {
    return (
      <div className="page">
        <div className="center-state">
          <Loader2 className="spin" size={20} />
          <span>Reading your account</span>
        </div>
      </div>
    );
  }

  const rank = data?.rank;
  const identity = data?.identity;
  const totalXp = data?.history.reduce((sum, row) => sum + row.xp, 0) ?? 0;

  return (
    <div className="page account-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <UserRound size={12} /> account
          </span>
          <h1>Your footprint.</h1>
          <p>
            RSO identity, account level, rank, wallet, first-win timer and restrictions in
            one place.
          </p>
        </div>
        <div className="head-status">
          <button className="button small" onClick={() => load()}>
            <RefreshCw size={13} /> Refresh
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <AlertTriangle size={16} />
          <div>
            <strong>Account unavailable</strong>
            <span>{error}</span>
          </div>
          <button className="button small" onClick={() => load()}>
            Retry
          </button>
        </div>
      )}

      {data && (
        <>
          <section className="account-hero">
            <div className="account-identity">
              <span className="account-avatar">
                <UserRound size={38} />
              </span>
              <div>
                <span className="profile-kicker">Signed-in Riot account</span>
                <h2>
                  {identity?.gameName ?? "Riot account"}
                  <em>#{identity?.tagLine ?? "-"}</em>
                </h2>
                <div className="account-badges">
                  <span>
                    <Globe2 size={12} /> {identity?.country ?? "-"} · {identity?.locale ?? "-"}
                  </span>
                  <span>
                    <CalendarDays size={12} /> {formatDate(identity?.createdAt ?? null)}
                  </span>
                  <span>
                    <ShieldCheck size={12} /> {identity?.state ?? "-"}
                  </span>
                </div>
              </div>
            </div>
            <div className="account-level">
              <span>Account level</span>
              <strong>{data.progress.level}</strong>
              <em>{data.progress.xp.toLocaleString()} XP this level</em>
            </div>
          </section>

          <div className="account-grid">
            <section className="panel rank-panel">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Competitive</span>
                  <h2>Current rank</h2>
                </div>
                <Trophy size={19} className="panel-icon" />
              </div>
              <div className="rank-hero" style={{ color: rankColor(rank?.tier ?? 0) }}>
                <img src={rankIcon(rank?.tier ?? 0)} alt="" />
                <div>
                  <strong>{rankName(rank?.tier ?? 0)}</strong>
                  <span>{(rank?.rr ?? 0).toLocaleString()} RR</span>
                </div>
              </div>
              <dl className="account-stats">
                <div>
                  <dt>Peak</dt>
                  <dd>{rankName(rank?.peak ?? 0)}</dd>
                </div>
                <div>
                  <dt>Win rate</dt>
                  <dd>{rank?.winRate ?? 0}%</dd>
                </div>
                <div>
                  <dt>Games</dt>
                  <dd>{rank?.games ?? 0}</dd>
                </div>
                <div>
                  <dt>Leaderboard</dt>
                  <dd>{rank?.leaderboard ? `#${rank.leaderboard}` : "-"}</dd>
                </div>
              </dl>
            </section>

            <section className="panel">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Wallet</span>
                  <h2>Balances</h2>
                </div>
                <Wallet size={19} className="panel-icon" />
              </div>
              <div className="account-wallet">
                <div>
                  <i style={{ background: "#ff4655" }} />
                  <span>VALORANT Points</span>
                  <strong>{data.wallet.vp.toLocaleString()}</strong>
                </div>
                <div>
                  <i style={{ background: "#38d6b0" }} />
                  <span>Radianite</span>
                  <strong>{data.wallet.rad.toLocaleString()}</strong>
                </div>
                <div>
                  <i style={{ background: "#e8b45a" }} />
                  <span>Kingdom Credits</span>
                  <strong>{data.wallet.kc.toLocaleString()}</strong>
                </div>
              </div>
              <div className="first-win">
                <Clock3 size={15} />
                <div>
                  <span>Next first-win bonus</span>
                  <strong>{formatRelative(data.progress.nextFirstWin)}</strong>
                </div>
                <em>{data.progress.lastFirstWin ? "Received" : "Not yet"}</em>
              </div>
            </section>

            <section className="panel">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Verification</span>
                  <h2>Identity checks</h2>
                </div>
                <ShieldCheck size={19} className="panel-icon" />
              </div>
              <div className="verification-list">
                <div className={identity?.emailVerified ? "verified" : ""}>
                  {identity?.emailVerified ? <Check size={15} /> : <AlertTriangle size={15} />}
                  <span>Email verified</span>
                </div>
                <div className={identity?.phoneVerified ? "verified" : ""}>
                  {identity?.phoneVerified ? <Phone size={15} /> : <Phone size={15} />}
                  <span>Phone verified</span>
                </div>
                <div className={identity?.accountVerified ? "verified" : ""}>
                  {identity?.accountVerified ? <ShieldCheck size={15} /> : <ShieldCheck size={15} />}
                  <span>Account verified</span>
                </div>
                <div className={identity?.providers.length ? "verified" : ""}>
                  <MailCheck size={15} />
                  <span>{(identity?.providers ?? []).join(" · ") || "No linked providers"}</span>
                </div>
              </div>
            </section>

            <section className="panel">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Restrictions</span>
                  <h2>Penalties</h2>
                </div>
                <AlertTriangle size={19} className="panel-icon" />
              </div>
              {data.penalties.length > 0 ? (
                <pre className="penalty-json">{JSON.stringify(data.penalties, null, 2)}</pre>
              ) : (
                <div className="verified-empty">
                  <ShieldCheck size={20} />
                  <strong>Clean record</strong>
                  <span>No active matchmaking or communication penalties.</span>
                </div>
              )}
            </section>
          </div>

          <section className="panel xp-panel">
            <div className="panel-heading">
              <div>
                <span className="panel-label">Account XP</span>
                <h2>Recent progress</h2>
              </div>
              <Sparkles size={19} className="panel-icon" />
            </div>
            <div className="xp-summary">
              <span>
                <strong>{totalXp.toLocaleString()}</strong> XP across the last{" "}
                {data.history.length} matches
              </span>
              <em>
                {data.region.toUpperCase()} · {data.clientVersion || "client version unknown"}
              </em>
            </div>
            {data.history.length > 0 ? (
              <div className="account-xp-list">
                {data.history.map((row) => (
                  <div className="account-xp-row" key={row.matchId}>
                    <span className="xp-match">{row.matchId.slice(0, 8)}</span>
                    <span className="xp-date">{formatDate(row.startedAt)}</span>
                    <span className="xp-sources">
                      {row.sources.map((source) => (
                        <em key={source.id}>
                          {SOURCES[source.id] ?? source.id} +{source.amount}
                        </em>
                      ))}
                    </span>
                    <strong>+{row.xp.toLocaleString()} XP</strong>
                  </div>
                ))}
              </div>
            ) : (
              <div className="owned-empty">
                <Sparkles size={17} />
                <span>No account XP history returned yet.</span>
              </div>
            )}
          </section>
        </>
      )}
    </div>
  );
}
