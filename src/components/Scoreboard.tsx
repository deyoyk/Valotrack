import { AlertTriangle, Check, Loader2, RefreshCw } from "lucide-react";
import { api, Player, TeamStats } from "../lib/api";
import { cn, num, pct } from "../lib/format";

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="team-stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function TeamHeader({
  name,
  stats,
  side,
  score,
  isSelf,
}: {
  name: string;
  stats?: TeamStats;
  side?: string | null;
  score?: number | null;
  isSelf: boolean;
}) {
  return (
    <div className={cn("team-head", isSelf && "self")}>
      <div className="team-title">
        <span className={cn("team-tag", isSelf ? "ally" : "enemy")}>
          {isSelf ? "Your team" : "Enemy team"}
        </span>
        <strong>{name}</strong>
        {side && <span className="side-pill">{side}</span>}
      </div>
      <div className="team-metrics">
        {score !== null && score !== undefined && <span className="team-score">{score}</span>}
        <Stat label="Avg rank" value={stats?.avgRank ?? "—"} />
        <Stat label="K/D" value={num(stats?.avgKd)} />
        <Stat label="Win rate" value={pct(stats?.avgWinRate)} />
        {!!stats?.smurfCount && (
          <span className="smurf-count">
            <AlertTriangle size={12} /> {stats.smurfCount} smurf
            {stats.smurfCount === 1 ? "" : "s"}
          </span>
        )}
      </div>
    </div>
  );
}

function Row({ player, rank }: { player: Player; rank: number }) {
  const acc = player.kd === null ? null : Math.min(100, Math.max(8, (player.kd / 2.2) * 100));
  return (
    <button className={cn("player-row", player.isSelf && "self", player.smurf && "flagged")}>
      <span
        className="party-rail"
        style={{ background: player.party?.color ?? "transparent" }}
        title={player.party ? `Party ${player.party.number}` : "Solo"}
      />
      <span className="rank-no tnum">{rank}</span>
      <span className="agent-cell">
        <span className="agent-frame">
          {player.agentPortrait ? (
            <img src={player.agentPortrait} alt={player.agent ?? "Agent"} loading="lazy" />
          ) : (
            <span className="agent-empty">?</span>
          )}
        </span>
        <span className="agent-name">{player.agent ?? "Lobby"}</span>
      </span>
      <span className="player-main">
        <span className="player-name-line">
          <strong>{player.name}</strong>
          {player.isSelf && <em>you</em>}
          {player.smurf && (
            <span className="smurf-badge" title={player.smurfReasons.join(" · ")}>
              smurf
            </span>
          )}
          {player.selection === "locked" && <span className="lock-badge">locked</span>}
        </span>
        <span className="player-sub">
          {player.title ? <span className="title-text">{player.title}</span> : null}
          {player.level > 0 && (
            <span>
              lvl {player.level}
              {player.levelHidden ? "+" : ""}
            </span>
          )}
          {player.form.length > 0 && (
            <span className="form-dots" title="Recent form">
              {player.form.slice(0, 5).map((result, index) => (
                <i key={index} className={result === "W" ? "w" : "l"} />
              ))}
            </span>
          )}
        </span>
      </span>
      <span className="rank-cell">
        <span className="rank-line">
          {player.rankIcon && <img src={player.rankIcon} alt="" loading="lazy" />}
          <strong style={{ color: player.rankColor }}>{player.rank}</strong>
        </span>
        <span className="rank-sub">
          {player.rr > 0 ? `${player.rr} RR` : "unrated"}
          {player.peakRankTier > player.rankTier && (
            <span className="peak">
              peak {player.peakRank}
              {player.peakAct ? ` · ${player.peakAct}` : ""}
            </span>
          )}
        </span>
      </span>
      <span className="metric-cell tnum">
        <strong>{num(player.kd)}</strong>
        <span>K/D</span>
        <i style={{ width: `${acc ?? 0}%` }} />
      </span>
      <span className="metric-cell tnum">
        <strong>{pct(player.hsPct)}</strong>
        <span>HS</span>
      </span>
      <span className="metric-cell tnum">
        <strong>{pct(player.winRate)}</strong>
        <span>{player.games ? `${player.games} games` : "win rate"}</span>
      </span>
      <span className="skin-cell">
        {player.skin?.icon && (
          <img src={player.skin.icon} alt={player.skin.name} loading="lazy" title={player.skin.name} />
        )}
      </span>
    </button>
  );
}

export function Scoreboard({
  board,
  onRefresh,
  refreshing,
}: {
  board: NonNullable<Awaited<ReturnType<typeof api.liveBoard>>>;
  onRefresh: () => void;
  refreshing: boolean;
}) {
  const teams = Object.keys(board.teams ?? {});
  const selfTeam = board.selfTeam || teams[0];
  const ordered = [selfTeam, ...teams.filter((team) => team !== selfTeam)].filter(
    (team) => (board.teams?.[team]?.length ?? 0) > 0,
  );
  const isLobby = board.state === "MENUS";

  return (
    <section className="board">
      <header className="board-head">
        {board.mapSplash && (
          <div
            className="board-splash"
            style={{ backgroundImage: `url(${board.mapSplash})` }}
          />
        )}
        <div className="board-head-inner">
          <div>
            <div className="board-kicker">
              <span className="live-dot" />
              {board.stateLabel}
              <span className="slash">/</span>
              {board.mode}
              {board.matchId && !isLobby && (
                <span className="match-id">{board.matchId.slice(0, 8)}</span>
              )}
            </div>
            <h1>{isLobby ? "Your lobby" : board.map || "Match"}</h1>
            {board.score && (
              <div className="match-score">
                <span>{board.score.ally}</span>
                <em>:</em>
                <span>{board.score.enemy}</span>
                {board.score.round !== null && board.score.round !== undefined && (
                  <small>round {board.score.round}</small>
                )}
              </div>
            )}
          </div>
          <div className="board-actions">
            {board.lockProgress && (
              <div className="lock-progress">
                <span>
                  {board.lockProgress.locked}/{board.lockProgress.total} locked
                </span>
                <div className="bar">
                  <i
                    style={{
                      width: `${(board.lockProgress.locked / board.lockProgress.total) * 100}%`,
                    }}
                  />
                </div>
              </div>
            )}
            <button className="icon-button" onClick={onRefresh} title="Refresh scoreboard">
              <RefreshCw size={16} className={refreshing ? "spin" : ""} />
            </button>
          </div>
        </div>
        {board.winProb !== null && (
          <div className="win-prob">
            <span className="wp-label">
              <strong>{board.winProb}%</strong> win probability
            </span>
            <div className="wp-bar">
              <i style={{ width: `${board.winProb}%` }} />
              <b style={{ left: `${board.winProb}%` }} />
            </div>
            <span className="wp-note">rank + recent form</span>
          </div>
        )}
      </header>

      {board.notice && !board.players?.length && (
        <div className="notice">
          <AlertTriangle size={16} />
          <span>{board.notice.message}</span>
        </div>
      )}

      {ordered.map((team) => {
        const players = board.teams?.[team] ?? [];
        return (
          <div className="team-block" key={team}>
            <TeamHeader
              name={isLobby ? "Lobby" : team}
              stats={board.teamStats?.[team]}
              side={board.side}
              score={board.score ? (team === selfTeam ? board.score.ally : board.score.enemy) : null}
              isSelf={team === selfTeam}
            />
            <div className="player-list">
              <div className="player-columns">
                <span />
                <span>#</span>
                <span>Agent</span>
                <span>Player</span>
                <span>Rank</span>
                <span>K/D</span>
                <span>HS</span>
                <span>WR</span>
                <span>Skin</span>
              </div>
              {players.map((player, index) => (
                <Row player={player} rank={index + 1} key={player.puuid} />
              ))}
              {players.some((player) => player.statsPending) && (
                <div className="stats-pending">
                  <Loader2 size={13} className="spin" />
                  loading recent form from Riot
                </div>
              )}
            </div>
          </div>
        );
      })}

      {!ordered.length && (
        <div className="board-empty">
          <div className="empty-icon">
            <Check size={20} />
          </div>
          <h2>Client connected</h2>
          <p>The scoreboard fills in once you join a lobby, agent select or a match.</p>
        </div>
      )}
    </section>
  );
}
