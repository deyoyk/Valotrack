import { useState, type CSSProperties } from "react";
import {
  AlertTriangle,
  Check,
  Loader2,
  RefreshCw,
  Share2,
  ShieldCheck,
  Swords,
  Users,
} from "lucide-react";
import { api, Player, TeamStats } from "../lib/api";
import { copyText } from "../lib/clipboard";
import { cn, num, pct } from "../lib/format";

function oppositeSide(side: string | null | undefined) {
  if (!side) return null;
  const label = side.toLowerCase();
  if (label.includes("attack")) return side.replace(/attack/i, "Defense");
  if (label.includes("defense") || label.includes("defence")) {
    return side.replace(/defen[cs]e/i, "Attack");
  }
  return null;
}

function FormDots({ form }: { form: string[] }) {
  if (!form.length) return null;
  return (
    <span className="duel-form" title="Recent form">
      {form.slice(0, 5).map((result, index) => (
        <i key={index} className={result === "W" ? "w" : "l"} />
      ))}
    </span>
  );
}

function Kpi({
  label,
  value,
  color,
  icon,
  smurfs = false,
}: {
  label: string;
  value: string;
  color?: string;
  icon?: string | null;
  smurfs?: boolean;
}) {
  return (
    <span className={cn("duel-kpi", smurfs && "smurfs")}>
      {icon ? <img src={icon} alt="" loading="lazy" /> : null}
      {smurfs ? <AlertTriangle size={12} /> : null}
      <strong style={color ? { color } : undefined}>{value}</strong>
      <em>{label}</em>
    </span>
  );
}

function PlayerCard({
  player,
  index,
  showAgent,
  onSelect,
}: {
  player: Player;
  index: number;
  showAgent: boolean;
  onSelect?: (player: Player) => void;
}) {
  return (
    <button
      className={cn(
        "duel-card",
        player.isSelf && "self",
        player.smurf && "flagged",
        !showAgent && "no-agent",
      )}
      onClick={() => onSelect?.(player)}
      style={{ "--row-index": index } as CSSProperties}
      title={`Open ${player.name}'s profile`}
    >
      <span
        className="duel-rail"
        style={{ background: player.party?.color ?? "transparent" }}
        title={player.party ? `Party ${player.party.number}` : "Solo"}
      />
      <span className="duel-rank">
        <span className="duel-rank-line">
          {player.rankIcon ? (
            <img src={player.rankIcon} alt="" loading="lazy" />
          ) : (
            <span className="duel-rank-empty" />
          )}
          <strong style={{ color: player.rankColor }}>{player.rank}</strong>
        </span>
        <span className="duel-rank-sub">
          {player.rr > 0 ? `${player.rr} RR` : "unrated"}
          {player.rrEarned ? (
            <b className={player.rrEarned > 0 ? "up" : "down"}>
              {player.rrEarned > 0 ? "+" : ""}
              {player.rrEarned}
            </b>
          ) : null}
          {player.leaderboard > 0 ? <b>#{player.leaderboard.toLocaleString()}</b> : null}
        </span>
        {player.peakRankTier > 0 && (
          <span className="duel-rank-peak">
            peak {player.peakRank}
            {player.peakAct ? ` · ${player.peakAct}` : ""}
          </span>
        )}
      </span>

      <span className="duel-identity">
        <span className="duel-name-line">
          <strong>{player.name}</strong>
          {player.isSelf && <em className="duel-you">you</em>}
          {player.smurf && (
            <em className="duel-smurf" title={player.smurfReasons.join(" · ")}>
              smurf
            </em>
          )}
          {player.selection === "locked" && <em className="duel-locked">locked</em>}
        </span>
        <span className="duel-meta-line">
          {player.level > 0 && (
            <span>
              lvl {player.level}
              {player.levelHidden ? "+" : ""}
            </span>
          )}
          {player.title ? <span className="duel-title-text">{player.title}</span> : null}
          {player.form.length > 0 && <FormDots form={player.form} />}
        </span>
        {player.smurf && (
          <span className="duel-flag-line">
            <AlertTriangle size={10} />
            {player.smurfReasons.slice(0, 2).join(" · ") || "performance signals detected"}
          </span>
        )}
        {(player.mapWinRate || player.previousRank) && (
          <span className="duel-meta-line subtle">
            {player.mapWinRate && (
              <span>
                map {player.mapWinRate.winRate}% · {player.mapWinRate.games}g
              </span>
            )}
            {player.previousRank && <span>prev {player.previousRank}</span>}
          </span>
        )}
      </span>

      <span className="duel-stats">
        <span>
          <strong>{num(player.kd)}</strong>
          <em>K/D</em>
        </span>
        <span>
          <strong>{pct(player.hsPct)}</strong>
          <em>HS</em>
        </span>
        <span title={player.games ? `${player.games} games this act` : "win rate"}>
          <strong>{pct(player.winRate)}</strong>
          <em>WIN</em>
        </span>
      </span>

      {showAgent && (
        <span className="duel-agent">
          {player.agentArt ? (
            <img className="duel-agent-art" src={player.agentArt} alt="" loading="lazy" />
          ) : null}
          <span className="duel-agent-copy">
            {player.agentPortrait ? (
              <img className="duel-agent-face" src={player.agentPortrait} alt="" loading="lazy" />
            ) : null}
            <strong>{player.agent ?? "No pick"}</strong>
            <em>{player.role ?? "—"}</em>
          </span>
        </span>
      )}

      {player.statsPending && (
        <span className="duel-row-pending" title="Refreshing recent form from Riot">
          <Loader2 size={11} className="spin" />
        </span>
      )}
    </button>
  );
}

function TeamPanel({
  board,
  team,
  isSelf,
  index,
  onPlayer,
}: {
  board: NonNullable<Awaited<ReturnType<typeof api.liveBoard>>>;
  team: string;
  isSelf: boolean;
  index: number;
  onPlayer?: (player: Player) => void;
}) {
  const players = board.teams?.[team] ?? [];
  const stats: TeamStats | undefined = board.teamStats?.[team];
  const isLobby = board.state === "MENUS";
  const showAgents = !isLobby;
  const side = isLobby ? null : isSelf ? board.side : oppositeSide(board.side);
  const parties = (board.parties ?? []).filter(
    (party) =>
      party.size > 1 &&
      players.some((player) => player.party?.id === party.id),
  );

  return (
    <article
      className={cn("duel-team", isSelf ? "ally" : "enemy")}
      style={{ "--team-index": index } as CSSProperties}
    >
      <header className="duel-head">
        <div className="duel-head-top">
          <span className={cn("duel-tag", isSelf ? "ally" : "enemy")}>
            {isSelf ? "YOUR TEAM" : "ENEMY TEAM"}
          </span>
          <strong>{isLobby ? "Lobby" : team}</strong>
          {side && <span className="duel-side">{side}</span>}
          <span className="duel-size">
            <Users size={11} /> {players.length}
          </span>
          <span className="duel-parties">
            {parties.map((party) => (
              <span className="duel-party" key={party.id}>
                <i style={{ background: party.color }} />
                {party.size}-stack
              </span>
            ))}
          </span>
        </div>
        <div className="duel-kpis">
          <Kpi
            label="avg rank"
            value={stats?.avgRank ?? "-"}
            color={stats?.rankColor}
            icon={stats?.rankIcon}
          />
          <Kpi label="avg K/D" value={num(stats?.avgKd)} />
          <Kpi label="win rate" value={pct(stats?.avgWinRate)} />
          <Kpi label="smurfs" value={String(stats?.smurfCount ?? 0)} smurfs />
        </div>
      </header>

      {showAgents && (
        <div className="duel-agents">
          <span className="duel-agents-label">
            <Swords size={11} /> picks
          </span>
          {players.map((player) => (
            <button
              key={player.puuid}
              className="duel-agent-chip"
              onClick={() => onPlayer?.(player)}
              title={`${player.name}${player.agent ? ` · ${player.agent}` : ""}`}
            >
              {player.agentPortrait ? (
                <img src={player.agentPortrait} alt={player.agent ?? "Agent"} loading="lazy" />
              ) : (
                <span className="duel-agent-empty">?</span>
              )}
              <span>
                <strong>{player.agent ?? "No pick"}</strong>
                <em>{player.name.split("#")[0]}</em>
              </span>
            </button>
          ))}
        </div>
      )}

      <div className="duel-list">
        {players.map((player, playerIndex) => (
          <PlayerCard
            key={player.puuid}
            player={player}
            index={playerIndex}
            showAgent={showAgents}
            onSelect={onPlayer}
          />
        ))}
        {players.some((player) => player.statsPending) && (
          <div className="duel-pending">
            <Loader2 size={12} className="spin" />
            syncing recent form from Riot
          </div>
        )}
      </div>
    </article>
  );
}

export function Scoreboard({
  board,
  onRefresh,
  refreshing,
  onPlayer,
}: {
  board: NonNullable<Awaited<ReturnType<typeof api.liveBoard>>>;
  onRefresh: () => void;
  refreshing: boolean;
  onPlayer?: (player: Player) => void;
}) {
  const teams = Object.keys(board.teams ?? {});
  const selfTeam = board.selfTeam || teams[0];
  const ordered = [selfTeam, ...teams.filter((team) => team !== selfTeam)].filter(
    (team) => (board.teams?.[team]?.length ?? 0) > 0,
  );
  const isLobby = board.state === "MENUS";
  const rounds = board.score?.rounds ?? [];
  const [copied, setCopied] = useState(false);

  async function shareBoard() {
    const lines = [
      `${board.map ?? "Lobby"} · ${board.mode}${board.score ? ` · ${board.score.ally}:${board.score.enemy}` : ""}`,
      ...ordered.map((team) => {
        const players = board.teams?.[team] ?? [];
        const label = team === selfTeam ? "Your team" : "Enemy team";
        return `${label}: ${players
          .map(
            (player) =>
              `${player.agent ?? "?"} ${player.name} (${player.rank}${player.rr > 0 ? ` ${player.rr} RR` : ""})`,
          )
          .join(", ")}`;
      }),
    ];
    if (await copyText(lines.join("\n"))) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    }
  }

  return (
    <section className="duel-board">
      <header
        className={cn("duel-match", isLobby && "lobby")}
        style={board.mapSplash ? { backgroundImage: `url(${board.mapSplash})` } : undefined}
      >
        <span className="duel-match-shade" />
        <div className="duel-match-top">
          <div>
            <div className="duel-kicker">
              <span className="live-dot" />
              {board.stateLabel}
              <span className="duel-slash">/</span>
              {board.mode}
              {board.source === "demo" && <span className="duel-demo">demo data</span>}
              {board.matchId && !isLobby && (
                <span className="duel-match-id">{board.matchId.slice(0, 8)}</span>
              )}
            </div>
            <h1>{isLobby ? "Your lobby" : board.map || "Match"}</h1>
            <div className="duel-match-sub">
              {board.side && <span className="duel-side pill">{board.side}</span>}
              {board.lockProgress && (
                <span className="duel-lock">
                  {board.lockProgress.locked}/{board.lockProgress.total} locked
                  <i>
                    <b
                      style={{
                        width: `${(board.lockProgress.locked / board.lockProgress.total) * 100}%`,
                      }}
                    />
                  </i>
                </span>
              )}
              {board.score?.round !== null && board.score?.round !== undefined && (
                <span className="duel-round-label">round {board.score.round}</span>
              )}
              {isLobby && (
                <span className="duel-lobby-note">
                  {board.players?.length ?? 0} in lobby · queue up to load both teams
                </span>
              )}
            </div>
          </div>

          {board.score && (
            <div className="duel-scoreboard">
              <div className="duel-score-side">
                <span>YOU</span>
                <strong>{board.score.ally}</strong>
              </div>
              <em>:</em>
              <div className="duel-score-side them">
                <span>THEM</span>
                <strong>{board.score.enemy}</strong>
              </div>
            </div>
          )}

          <div className="duel-actions">
            <button className="icon-button" onClick={onRefresh} title="Refresh scoreboard">
              <RefreshCw size={16} className={refreshing ? "spin" : ""} />
            </button>
            <button className="icon-button" onClick={() => void shareBoard()} title="Copy lobby summary">
              {copied ? <Check size={16} /> : <Share2 size={16} />}
            </button>
          </div>
        </div>

        {rounds.length > 0 && (
          <div className="duel-rounds" aria-label="Round history">
            <span>ROUNDS</span>
            {rounds.map((round, index) => (
              <i key={index} className={round === "W" ? "w" : "l"} title={round === "W" ? "Win" : "Loss"} />
            ))}
          </div>
        )}

        {board.winProb !== null && (
          <div className="duel-winbar">
            <span>
              <strong>{board.winProb}%</strong> win probability
            </span>
            <div>
              <i style={{ width: `${board.winProb}%` }} />
              <b style={{ left: `${board.winProb}%` }} />
            </div>
            <em>rank + recent form</em>
          </div>
        )}
      </header>

      {board.notice && !board.players?.length && (
        <div className="notice">
          <AlertTriangle size={16} />
          <span>{board.notice.message}</span>
        </div>
      )}

      {ordered.length > 0 ? (
        <div className={cn("duel-teams", ordered.length === 1 && "single")}>
          {ordered.map((team, index) => (
            <TeamPanel
              key={team}
              board={board}
              team={team}
              isSelf={team === selfTeam}
              index={index}
              onPlayer={onPlayer}
            />
          ))}
        </div>
      ) : (
        <div className="duel-empty">
          <span className="duel-empty-icon">
            <ShieldCheck size={22} />
          </span>
          <h2>Client connected</h2>
          <p>The board fills in once you join a lobby, agent select or a match.</p>
        </div>
      )}
    </section>
  );
}

export function ScoreboardSkeleton() {
  return (
    <section className="duel-board duel-skeleton">
      <div className="duel-skeleton-head">
        <span />
        <span />
        <span />
      </div>
      <div className="duel-teams">
        {[0, 1].map((column) => (
          <div className="duel-skeleton-team" key={column}>
            <div className="duel-skeleton-row tall" />
            {[0, 1, 2, 3, 4].map((row) => (
              <div className="duel-skeleton-row" key={row} />
            ))}
          </div>
        ))}
      </div>
    </section>
  );
}
