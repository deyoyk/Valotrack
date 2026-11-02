import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ArrowUpRight,
  Crosshair,
  Loader2,
  ShieldAlert,
  Swords,
  UserRound,
} from "lucide-react";
import { api, Agent, CareerMatch, CareerProfile, Player } from "../lib/api";
import { demoProfile } from "../lib/demo";
import { cn, formatDate, num, pct } from "../lib/format";
import { Modal } from "./Modal";

function ResultDot({ result }: { result: string }) {
  return (
    <span
      className={cn(
        "result-dot",
        result === "Victory" ? "win" : result === "Defeat" ? "loss" : "draw",
      )}
    >
      {result === "Victory" ? "W" : result === "Defeat" ? "L" : "D"}
    </span>
  );
}

export function ProfileModal({
  player,
  onClose,
  onMatch,
  demo,
  agents = [],
}: {
  player: Player;
  onClose: () => void;
  onMatch: (match: CareerMatch) => void;
  demo?: boolean;
  agents?: Agent[];
}) {
  const [profile, setProfile] = useState<CareerProfile | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<"overview" | "matches" | "collection">("overview");

  useEffect(() => {
    let cancelled = false;
    setProfile(null);
    setError(null);
    if (demo) {
      setProfile(demoProfile(player, agents));
      return () => {
        cancelled = true;
      };
    }
    api
      .profile(player.puuid)
      .then((data) => {
        if (!cancelled) setProfile(data);
      })
      .catch((cause) => {
        if (!cancelled) {
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [agents, demo, player]);

  const encounters = profile?.encounter;
  const trackerUrl = `https://tracker.gg/valorant/profile/riot/${encodeURIComponent(
    player.name.replace("#", "%23"),
  )}/overview`;

  return (
    <Modal onClose={onClose} label={`${player.name} profile`} className="profile-modal">
      <div className="profile-hero">
        {player.agentArt && (
          <div
            className="profile-art"
            style={{ backgroundImage: `url(${player.agentArt})` }}
          />
        )}
        <div className="profile-identity">
          <div className="profile-card-art">
            {player.playerCard ? (
              <img src={player.playerCard} alt="" />
            ) : (
              <UserRound size={34} />
            )}
          </div>
          <div>
            <span className="profile-kicker">
              {player.role ?? "Player"} {player.title ? `· ${player.title}` : ""}
            </span>
            <h2>{player.name}</h2>
            <div className="profile-tags">
              <span style={{ color: player.rankColor }}>{player.rank}</span>
              {player.rr > 0 && <span>{player.rr} RR</span>}
              {player.level > 0 && <span>level {player.level}</span>}
              {player.isSelf && <em>you</em>}
              {player.smurf && (
                <span className="smurf-badge" title={player.smurfReasons.join(" · ")}>
                  smurf signal
                </span>
              )}
            </div>
          </div>
          <div className="profile-headline-stats">
            <div>
              <strong>{num(player.kd)}</strong>
              <span>K/D</span>
            </div>
            <div>
              <strong>{pct(player.winRate)}</strong>
              <span>win rate</span>
            </div>
            <div>
              <strong>{pct(player.hsPct)}</strong>
              <span>headshots</span>
            </div>
            <div>
              <strong>{player.peakRankTier > 0 ? player.peakRank : "-"}</strong>
              <span>peak</span>
            </div>
          </div>
        </div>
        <div className="profile-hero-actions">
          <button
            className="button small"
            onClick={() => void openUrl(trackerUrl)}
          >
            tracker.gg <ArrowUpRight size={12} />
          </button>
        </div>
      </div>

      <div className="profile-tabs">
        <button className={tab === "overview" ? "active" : ""} onClick={() => setTab("overview")}>
          Overview
        </button>
        <button className={tab === "matches" ? "active" : ""} onClick={() => setTab("matches")}>
          Matches
        </button>
        <button
          className={tab === "collection" ? "active" : ""}
          onClick={() => setTab("collection")}
        >
          Collection
        </button>
      </div>

      {!profile && !error && (
        <div className="profile-loading">
          <Loader2 size={18} className="spin" />
          <span>Reading match history from Riot</span>
        </div>
      )}

      {error && (
        <div className="profile-loading error">
          <ShieldAlert size={18} />
          <span>{error}</span>
        </div>
      )}

      {profile && tab === "overview" && (
        <div className="profile-body">
          <div className="profile-stat-grid">
            {[
              ["Games", String(profile.averages.games)],
              ["Win rate", `${profile.averages.winRate}%`],
              ["K/D", num(profile.averages.kd)],
              ["HS", pct(profile.averages.hsPct)],
              ["Kills", num(profile.averages.kills, 1)],
              ["Deaths", num(profile.averages.deaths, 1)],
              ["Assists", num(profile.averages.assists, 1)],
              ["Wins", String(profile.averages.wins)],
            ].map(([label, value]) => (
              <div key={label}>
                <strong>{value}</strong>
                <span>{label}</span>
              </div>
            ))}
          </div>

          <div className="profile-columns">
            <div className="profile-section">
              <span className="panel-label">Agent pool</span>
              {profile.agentPool.length ? (
                profile.agentPool.map((agent) => (
                  <div className="mini-bar-row" key={agent.agent}>
                    <span>{agent.agent}</span>
                    <div className="mini-bar">
                      <i style={{ width: `${agent.winRate}%` }} />
                    </div>
                    <strong>{agent.winRate}%</strong>
                    <em>{agent.games}g</em>
                  </div>
                ))
              ) : (
                <p className="quiet">No agent games in the window.</p>
              )}
            </div>
            <div className="profile-section">
              <span className="panel-label">Maps</span>
              {profile.mapStats.length ? (
                profile.mapStats.map((map) => (
                  <div className="mini-bar-row" key={map.map}>
                    <span>{map.map}</span>
                    <div className="mini-bar">
                      <i style={{ width: `${map.winRate}%` }} />
                    </div>
                    <strong>{map.winRate}%</strong>
                    <em>{map.games}g</em>
                  </div>
                ))
              ) : (
                <p className="quiet">No map games in the window.</p>
              )}
            </div>
          </div>

          <div className="profile-columns">
            <div className="profile-section">
              <span className="panel-label">Frequent teammates</span>
              {profile.coPlayers.length ? (
                profile.coPlayers.map((mate) => (
                  <div className="mate-row" key={mate.puuid}>
                    <span className="mate-avatar">{mate.name.slice(0, 2).toUpperCase()}</span>
                    <div>
                      <strong>{mate.name}</strong>
                      <span>
                        {mate.sharedMatches} shared · {mate.agents.join(", ") || "mixed"}
                      </span>
                    </div>
                    {mate.isParty && <em>party</em>}
                  </div>
                ))
              ) : (
                <p className="quiet">No repeat teammates.</p>
              )}
            </div>
            <div className="profile-section">
              <span className="panel-label">Cross-session history</span>
              {encounters && encounters.withCount + encounters.againstCount > 0 ? (
                <div className="encounter-summary">
                  <div>
                    <strong>{encounters.withCount}</strong>
                    <span>with you</span>
                  </div>
                  <div>
                    <strong>{encounters.againstCount}</strong>
                    <span>against you</span>
                  </div>
                  <div>
                    <strong>{encounters.winsWith}</strong>
                    <span>wins together</span>
                  </div>
                  <div>
                    <strong>{encounters.winsAgainst}</strong>
                    <span>beaten by you</span>
                  </div>
                  <p>
                    Last seen {formatDate(encounters.lastSeen)}
                    {encounters.agent ? ` on ${encounters.agent}` : ""}.
                  </p>
                </div>
              ) : (
                <p className="quiet">First recorded encounter.</p>
              )}
            </div>
          </div>
        </div>
      )}

      {profile && tab === "matches" && (
        <div className="profile-body match-list">
          {profile.matches.length ? (
            profile.matches.map((match) => (
              <button
                className="career-row"
                key={match.matchId}
                onClick={() => onMatch(match)}
              >
                <ResultDot result={match.result} />
                <span className="career-map">
                  {match.mapSplash ? <img src={match.mapSplash} alt="" /> : <Swords size={17} />}
                  <span>
                    <strong>{match.map}</strong>
                    <em>{match.mode}</em>
                  </span>
                </span>
                <span className="career-agent">
                  <img src={match.agentPortrait} alt={match.agent} />
                  {match.agent}
                </span>
                <span className="career-kda tnum">
                  <strong>
                    {match.kills}/{match.deaths}/{match.assists}
                  </strong>
                  <em>K/D/A</em>
                </span>
                <span className="career-kda tnum">
                  <strong>{match.acs}</strong>
                  <em>ACS</em>
                </span>
                <span className="career-kda tnum">
                  <strong>{pct(match.hsPct)}</strong>
                  <em>HS</em>
                </span>
                <span className="career-score tnum">
                  {match.score}:{match.opponentScore}
                </span>
                <span className="career-date">{formatDate(match.startMillis / 1000)}</span>
                {match.rrDelta !== undefined && match.rrDelta !== null && (
                  <span className={cn("rr-delta", match.rrDelta >= 0 ? "up" : "down")}>
                    {match.rrDelta > 0 ? "+" : ""}
                    {match.rrDelta} RR
                  </span>
                )}
              </button>
            ))
          ) : (
            <p className="quiet">No recent matches returned by Riot.</p>
          )}
        </div>
      )}

      {profile && tab === "collection" && (
        <div className="profile-body">
          <div className="collection-head">
            <span className="panel-label">Weapons seen in the current match</span>
            <span>{player.weapons.length} items</span>
          </div>
          {player.weapons.length ? (
            <div className="weapon-grid">
              {player.weapons.map((item) => (
                <div className="weapon-card" key={item.weapon}>
                  {item.skin.icon ? (
                    <img src={item.skin.icon} alt={item.skin.name} loading="lazy" />
                  ) : (
                    <Crosshair size={22} />
                  )}
                  <strong>{item.skin.name}</strong>
                  <span>{item.weapon}</span>
                </div>
              ))}
            </div>
          ) : (
            <p className="quiet">
              Loadouts appear during agent select and in-game. Open VALORANT to read this
              player's inventory.
            </p>
          )}
        </div>
      )}
    </Modal>
  );
}
