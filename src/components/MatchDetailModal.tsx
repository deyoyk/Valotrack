import { useEffect, useState } from "react";
import { AlertTriangle, Check, Loader2, Save, Swords } from "lucide-react";
import { api, Agent, MatchDetail, MatchPlayer } from "../lib/api";
import { demoMatchDetail } from "../lib/demo";
import { cn, num, pct } from "../lib/format";
import { Modal } from "./Modal";

function TeamBlock({
  title,
  players,
  stats,
  score,
  highlight,
}: {
  title: string;
  players: MatchPlayer[];
  stats?: MatchDetail["teamStats"][string];
  score?: number;
  highlight: boolean;
}) {
  return (
    <div className={cn("detail-team", highlight && "self")}>
      <div className="detail-team-head">
        <div>
          <span className="team-tag">{highlight ? "Your team" : "Enemy team"}</span>
          <strong>{title}</strong>
          {stats && <em style={{ color: stats.avgRankColor }}>{stats.avgRank}</em>}
        </div>
        {score !== undefined && <strong className="detail-score">{score}</strong>}
      </div>
      <div className="detail-columns">
        <span>Player</span>
        <span>K</span>
        <span>D</span>
        <span>A</span>
        <span>ACS</span>
        <span>HS</span>
        <span>ADR</span>
        <span>FK</span>
        <span>MK</span>
        <span>ACE</span>
      </div>
      {players.map((player) => (
        <div className={cn("detail-row", player.isSubject && "subject")} key={player.puuid}>
          <span className="detail-player">
            <span className="detail-agent" style={{ borderColor: player.agentColor }}>
              {player.agentPortrait ? <img src={player.agentPortrait} alt="" /> : null}
            </span>
            <span>
              <strong>
                {player.name}
                {player.isSubject && <em>you</em>}
                {player.isMatchMvp && <b className="mvp">MVP</b>}
              </strong>
              <small style={{ color: player.rankColor }}>{player.rank}</small>
            </span>
          </span>
          <span className="tnum">{player.kills}</span>
          <span className="tnum">{player.deaths}</span>
          <span className="tnum">{player.assists}</span>
          <span className="tnum strong">{player.acs}</span>
          <span className="tnum">{pct(player.hsPct)}</span>
          <span className="tnum">{num(player.adr, 0)}</span>
          <span className="tnum">{player.firstBloods ?? "-"}</span>
          <span className="tnum">{player.multiKills ?? "-"}</span>
          <span className="tnum">{player.aces ?? "-"}</span>
        </div>
      ))}
    </div>
  );
}

export function MatchDetailModal({
  matchId,
  subject,
  title,
  onClose,
  demo,
  agents = [],
}: {
  matchId: string;
  subject?: string;
  title: string;
  onClose: () => void;
  demo?: boolean;
  agents?: Agent[];
}) {
  const [detail, setDetail] = useState<MatchDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const [tags, setTags] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (demo) return;
    api
      .matchNote(matchId)
      .then((value) => {
        setNote(value.note ?? "");
        setTags((value.tags ?? []).join(", "));
      })
      .catch(() => void 0);
  }, [demo, matchId]);

  useEffect(() => {
    let cancelled = false;
    if (demo) {
      setDetail(demoMatchDetail(matchId, agents));
      return () => {
        cancelled = true;
      };
    }
    api
      .matchDetail(matchId, subject)
      .then((value) => {
        if (!cancelled) setDetail(value);
      })
      .catch((cause) => {
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [agents, demo, matchId, subject]);

  const selfTeam = detail?.players.find((player) => player.isSubject)?.team;
  const teams = Object.keys(detail?.scores ?? {});

  return (
    <Modal onClose={onClose} label="Match details" className="match-detail-modal">
      <div className="detail-hero">
        {detail?.mapSplash && (
          <div
            className="detail-splash"
            style={{ backgroundImage: `url(${detail.mapSplash})` }}
          />
        )}
        <div className="detail-hero-inner">
          <div>
            <span className="profile-kicker">{detail?.mode ?? "Match"}</span>
            <h2>{detail?.map ?? title}</h2>
            <div className="detail-result-line">
              {detail && (
                <span
                  className={cn(
                    "detail-result",
                    detail.result === "Victory"
                      ? "win"
                      : detail.result === "Defeat"
                        ? "loss"
                        : "draw",
                  )}
                >
                  {detail.result}
                </span>
              )}
              {detail && teams.length > 0 && (
                <span className="detail-scoreline">
                  {teams.map((team) => detail.scores[team]).join(" : ")}
                </span>
              )}
            </div>
          </div>
          <Swords size={30} className="detail-icon" />
        </div>
        {detail?.rounds.length ? (
          <div className="round-strip">
            {detail.rounds.map((round) => (
              <span
                key={round.n}
                className={cn("round-chip", round.won ? "won" : "lost")}
                title={`Round ${round.n}: ${round.how}`}
              >
                {round.n}
              </span>
            ))}
          </div>
        ) : null}
      </div>

      {!detail && !error && (
        <div className="profile-loading">
          <Loader2 size={18} className="spin" />
          <span>Reading the full scoreboard</span>
        </div>
      )}
      {error && (
        <div className="profile-loading error">
          <AlertTriangle size={18} />
          <span>{error}</span>
        </div>
      )}

      {detail && (
        <div className="detail-teams">
          {teams.map((team) => (
            <TeamBlock
              key={team}
              title={team}
              players={detail.players.filter((player) => player.team === team)}
              stats={detail.teamStats[team]}
              score={detail.scores[team]}
              highlight={team === selfTeam}
            />
          ))}
          {!demo && (
            <div className="match-note">
              <div className="match-note-head">
                <span className="panel-label">Match note</span>
                {saved && (
                  <span className="saved-note">
                    <Check size={11} /> saved
                  </span>
                )}
              </div>
              <textarea
                value={note}
                onChange={(event) => {
                  setNote(event.target.value);
                  setSaved(false);
                }}
                placeholder="What worked? What went wrong? What do you want to remember next time?"
                rows={3}
              />
              <div className="match-note-foot">
                <input
                  value={tags}
                  onChange={(event) => {
                    setTags(event.target.value);
                    setSaved(false);
                  }}
                  placeholder="tags: clutch, retake, aim"
                />
                <button
                  className="button small"
                  onClick={async () => {
                    await api.matchNoteUpdate(
                      matchId,
                      note,
                      tags.split(",").map((tag) => tag.trim()).filter(Boolean),
                    );
                    setSaved(true);
                  }}
                >
                  <Save size={12} /> Save note
                </button>
              </div>
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}
