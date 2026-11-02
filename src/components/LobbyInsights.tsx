import { AlertTriangle, ShieldCheck, Users } from "lucide-react";
import { LiveBoard, Player } from "../lib/api";

function teamLabel(board: LiveBoard, puuid: string) {
  const player = board.players.find((entry) => entry.puuid === puuid);
  if (!player) return "team";
  return player.team === board.selfTeam ? "your team" : "enemy team";
}

export function LobbyInsights({
  board,
  onPlayer,
}: {
  board: LiveBoard;
  onPlayer: (player: Player) => void;
}) {
  const flagged = board.players.filter((player) => player.smurf);
  const parties = (board.parties ?? []).filter((party) => party.size > 1);
  if (!flagged.length && !parties.length) return null;

  return (
    <div className="lobby-insights">
      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="panel-label">Smurf radar</span>
            <h2>Signals to watch</h2>
          </div>
          <AlertTriangle size={19} className="panel-icon" />
        </div>
        {flagged.length ? (
          <div className="smurf-list">
            {flagged.map((player) => (
              <button
                className="smurf-row"
                key={player.puuid}
                onClick={() => onPlayer(player)}
              >
                <span className="smurf-avatar">
                  {player.agentPortrait ? (
                    <img src={player.agentPortrait} alt="" loading="lazy" />
                  ) : (
                    <AlertTriangle size={15} />
                  )}
                </span>
                <span className="smurf-copy">
                  <strong>{player.name}</strong>
                  <span>
                    level {player.level || "-"} · {player.rank}
                    {player.kd !== null ? ` · ${player.kd.toFixed(2)} K/D` : ""}
                  </span>
                  <em>{player.smurfReasons.join(" · ") || "performance signals detected"}</em>
                </span>
                <span className="smurf-team">{teamLabel(board, player.puuid)}</span>
              </button>
            ))}
          </div>
        ) : (
          <div className="insight-empty">
            <ShieldCheck size={20} />
            <span>No smurf signals in this lobby.</span>
          </div>
        )}
      </section>

      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="panel-label">Party breakdown</span>
            <h2>Who queued together</h2>
          </div>
          <Users size={19} className="panel-icon" />
        </div>
        {parties.length ? (
          <div className="party-list">
            {parties.map((party) => {
              const members = board.players.filter(
                (player) => player.party?.id === party.id,
              );
              return (
                <div className="party-block" key={party.id}>
                  <span className="party-block-head">
                    <i style={{ background: party.color }} />
                    {party.size}-stack
                    <em>{teamLabel(board, members[0]?.puuid ?? "")}</em>
                  </span>
                  <span className="party-members">
                    {members
                      .map((member) => member.name.split("#")[0])
                      .join(" · ")}
                  </span>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="insight-empty">
            <Users size={20} />
            <span>Everyone in this lobby is solo.</span>
          </div>
        )}
      </section>
    </div>
  );
}
