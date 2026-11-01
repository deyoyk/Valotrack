import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
  Crown,
  Loader2,
  RefreshCw,
  Search,
  Trophy,
  Users,
} from "lucide-react";
import { api, LeaderboardSnapshot } from "../lib/api";

function PlayerRow({
  player,
  spotlight,
}: {
  player: LeaderboardSnapshot["players"][number];
  spotlight?: boolean;
}) {
  return (
    <div className={player.isSelf ? "leaderboard-row self" : "leaderboard-row"}>
      <span className={spotlight ? "leaderboard-rank podium" : "leaderboard-rank"}>
        {player.rank > 0 ? `#${player.rank}` : "-"}
      </span>
      <span className="leaderboard-avatar">
        {player.card ? <img src={player.card} alt="" loading="lazy" /> : <Users size={15} />}
      </span>
      <span className="leaderboard-player">
        <strong>
          {player.gameName || "Anonymous"}
          <em>#{player.tagLine}</em>
        </strong>
        <span>{player.title || "No title"}</span>
      </span>
      <span className="leaderboard-tier" style={{ color: player.tierColor }}>
        <img src={player.tierIcon} alt="" loading="lazy" />
        {player.tierName}
      </span>
      <span className="leaderboard-rr">{player.rr.toLocaleString()} RR</span>
      <span className="leaderboard-wins">{player.wins.toLocaleString()} wins</span>
    </div>
  );
}

export default function Leaderboard() {
  const [data, setData] = useState<LeaderboardSnapshot | null>(null);
  const [query, setQuery] = useState("");
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (search = "") => {
    setLoading(true);
    try {
      const snapshot = await api.leaderboard(search, 100);
      setData(snapshot);
      setQuery(search);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load("");
  }, [load]);

  const top = data?.players.slice(0, 3) ?? [];

  return (
    <div className="page leaderboard-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Trophy size={12} /> competitive
          </span>
          <h1>Climb the ladder.</h1>
          <p>
            Live Radiant leaderboard for the active act, with player search and your own
            placement highlighted.
          </p>
        </div>
        <div className="head-status">
          {data?.seasonLabel && (
            <span className="account-badge">
              <span className="dot" />
              {data.seasonLabel} · {data.region.toUpperCase()}
            </span>
          )}
          <button className="button small" onClick={() => load(query)} disabled={loading}>
            <RefreshCw size={13} className={loading ? "spin" : ""} /> Refresh
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <AlertTriangle size={16} />
          <div>
            <strong>Leaderboard unavailable</strong>
            <span>{error}</span>
          </div>
          <button className="button small" onClick={() => load(query)}>
            Retry
          </button>
        </div>
      )}

      <section className="leaderboard-toolbar">
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void load(input.trim());
          }}
        >
          <div className="search-box">
            <Search size={15} />
            <input
              value={input}
              onChange={(event) => setInput(event.target.value)}
              placeholder="Search Riot ID or tagline"
            />
          </div>
          <button className="button primary" type="submit" disabled={loading}>
            Search
          </button>
          {query && (
            <button
              className="button"
              type="button"
              onClick={() => {
                setInput("");
                void load("");
              }}
            >
              Clear
            </button>
          )}
        </form>
        <span className="leaderboard-count">
          {data ? `${data.players.length} of ${data.totalPlayers.toLocaleString()} players` : "-"}
        </span>
      </section>

      {loading && !data ? (
        <div className="center-state">
          <Loader2 className="spin" size={20} />
          <span>Fetching the ladder</span>
        </div>
      ) : data ? (
        <>
          {top.length === 3 && !query && (
            <section className="podium-grid">
              {top.map((player, index) => (
                <article className="podium-card" key={player.puuid}>
                  <span className="podium-place">
                    {index === 0 ? <Crown size={18} /> : `#${index + 1}`}
                  </span>
                  <span className="podium-avatar">
                    {player.card ? <img src={player.card} alt="" /> : <Users size={22} />}
                  </span>
                  <strong>
                    {player.gameName || "Anonymous"}
                    <em>#{player.tagLine}</em>
                  </strong>
                  <span style={{ color: player.tierColor }}>
                    {player.tierName} · {player.rr.toLocaleString()} RR
                  </span>
                </article>
              ))}
            </section>
          )}

          <section className="panel leaderboard-panel">
            <div className="leaderboard-head">
              <span>Rank</span>
              <span>Player</span>
              <span>Tier</span>
              <span>Rating</span>
              <span>Wins</span>
            </div>
            <div className="leaderboard-list">
              {data.players.length > 0 ? (
                data.players.map((player) => (
                  <PlayerRow
                    player={player}
                    spotlight={!query && player.rank <= 3}
                    key={player.puuid}
                  />
                ))
              ) : (
                <div className="owned-empty">
                  <Search size={17} />
                  <span>No players matched that search.</span>
                </div>
              )}
            </div>
          </section>
        </>
      ) : null}
    </div>
  );
}
