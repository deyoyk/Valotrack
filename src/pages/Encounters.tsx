import { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  ArrowDownUp,
  Loader2,
  RefreshCw,
  Search,
  Users,
} from "lucide-react";
import { api, EncounterList, Player } from "../lib/api";
import { formatDate, num, pct } from "../lib/format";
import { ProfileModal } from "../components/ProfileModal";
import { CareerMatch } from "../lib/api";
import { MatchDetailModal } from "../components/MatchDetailModal";

function asPlayer(encounter: EncounterList["players"][number]): Player {
  return {
    puuid: encounter.puuid,
    name: encounter.name,
    nameHidden: false,
    team: "Blue",
    isSelf: false,
    title: null,
    playerCard: null,
    agent: encounter.agent,
    agentId: null,
    agentPortrait: encounter.agentPortrait,
    agentArt: null,
    agentColor: encounter.agentColor ?? "#8B978F",
    role: null,
    selection: null,
    rankTier: 0,
    rank: encounter.rank ?? "Unranked",
    rankColor: encounter.rankColor ?? "#4A4A4A",
    rankGroup: "",
    rankIcon: null,
    rr: 0,
    rrEarned: null,
    leaderboard: 0,
    peakRankTier: 0,
    peakRank: encounter.peakRank ?? "Unranked",
    peakColor: "#4A4A4A",
    peakIcon: null,
    peakAct: null,
    previousRank: "Unranked",
    winRate: encounter.winRate ?? 0,
    games: 0,
    kd: encounter.kd,
    hsPct: null,
    skin: null,
    weapons: [],
    level: encounter.level ?? 0,
    levelHidden: false,
    party: null,
    smurf: false,
    smurfReasons: [],
    topAgents: [],
    form: [],
    streak: null,
    mapWinRate: null,
  };
}

type Sort = "encounters" | "recent" | "winrate";

export default function Encounters() {
  const [data, setData] = useState<EncounterList | null>(null);
  const [scope, setScope] = useState<"current" | "all">("current");
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<Sort>("encounters");
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<Player | null>(null);
  const [selectedMatch, setSelectedMatch] = useState<CareerMatch | null>(null);

  async function load(nextScope = scope, quiet = false) {
    if (!quiet) setLoading(true);
    try {
      setData(await api.encounters(nextScope));
    } catch {
      setData({ players: [], accountCount: 0, scope: nextScope });
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load(scope, true);
  }, [scope]);

  const players = useMemo(() => {
    const needle = search.trim().toLowerCase();
    const rows = (data?.players ?? []).filter(
      (row) =>
        !needle ||
        row.name.toLowerCase().includes(needle) ||
        (row.agent ?? "").toLowerCase().includes(needle),
    );
    return rows.sort((a, b) => {
      if (sort === "recent") return (b.lastSeen ?? 0) - (a.lastSeen ?? 0);
      if (sort === "winrate") return (b.winRate ?? 0) - (a.winRate ?? 0);
      return b.withCount + b.againstCount - (a.withCount + a.againstCount);
    });
  }, [data, search, sort]);

  const total = data?.players.reduce((sum, row) => sum + row.withCount + row.againstCount, 0) ?? 0;

  return (
    <div className="page encounters-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Users size={12} /> encounter log
          </span>
          <h1>I know that name.</h1>
          <p>
            Everyone you queue with or against is recorded on this machine. Track
            repeat teammates, rivals and the lobbies you keep landing in.
          </p>
        </div>
        <div className="head-status">
          <span className="checked">
            {data?.accountCount ?? 0} accounts · {total} encounters
          </span>
          <button className="button small" onClick={() => load(scope)} disabled={loading}>
            <RefreshCw size={13} className={loading ? "spin" : ""} /> Refresh
          </button>
        </div>
      </header>

      <div className="encounter-toolbar">
        <div className="search-box encounter-search">
          <Search size={14} />
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search player or agent"
          />
        </div>
        <div className="segmented">
          <button className={scope === "current" ? "active" : ""} onClick={() => setScope("current")}>
            Current account
          </button>
          <button className={scope === "all" ? "active" : ""} onClick={() => setScope("all")}>
            All accounts
          </button>
        </div>
        <div className="segmented">
          {(
            [
              ["encounters", "Most met"],
              ["recent", "Recent"],
              ["winrate", "Win rate"],
            ] as [Sort, string][]
          ).map(([value, label]) => (
            <button
              key={value}
              className={sort === value ? "active" : ""}
              onClick={() => setSort(value)}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {loading ? (
        <div className="center-state">
          <Loader2 className="spin" size={20} />
          <span>Reading the encounter ledger</span>
        </div>
      ) : players.length ? (
        <div className="encounter-table">
          <div className="encounter-columns">
            <span>Player</span>
            <span>Rank</span>
            <span>Together</span>
            <span>Against</span>
            <span>Record</span>
            <span>K/D</span>
            <span>WR</span>
            <span>Last met</span>
          </div>
          {players.map((row) => (
            <button
              className="encounter-row"
              key={row.puuid}
              onClick={() => setSelected(asPlayer(row))}
            >
              <span className="encounter-player">
                <span className="encounter-agent" style={{ borderColor: row.agentColor ?? "#2a2f3a" }}>
                  {row.agentPortrait ? <img src={row.agentPortrait} alt="" /> : <Users size={12} />}
                </span>
                <span>
                  <strong>{row.name}</strong>
                  <em>
                    level {row.level || "-"} {row.agent ? `· ${row.agent}` : ""}
                  </em>
                </span>
              </span>
              <span style={{ color: row.rankColor ?? "var(--muted)" }} className="encounter-rank">
                {row.rank ?? "Unranked"}
              </span>
              <span className="tnum">{row.withCount}</span>
              <span className="tnum">{row.againstCount}</span>
              <span className="record-cell tnum">
                <b className="pos">{row.winsWith}</b>
                <i>/</i>
                <b className="neg">{row.lossesWith}</b>
                <span>with</span>
                <b className="pos">{row.winsAgainst}</b>
                <i>/</i>
                <b className="neg">{row.lossesAgainst}</b>
                <span>vs</span>
              </span>
              <span className="tnum">{num(row.kd)}</span>
              <span className="tnum">{pct(row.winRate)}</span>
              <span className="encounter-date">{formatDate(row.lastSeen)}</span>
            </button>
          ))}
        </div>
      ) : (
        <div className="board-empty">
          <div className="empty-icon">
            <AlertTriangle size={20} />
          </div>
          <h2>No encounters yet</h2>
          <p>
            Jump into a match with VALORANT open. Everyone in the lobby is logged
            automatically and grouped by account.
          </p>
        </div>
      )}

      <p className="ledger-note">
        <ArrowDownUp size={12} /> The ledger is local JSON. Clear it from Settings at any
        time.
      </p>

      {selected && (
        <ProfileModal
          player={selected}
          onClose={() => setSelected(null)}
          onMatch={setSelectedMatch}
        />
      )}
      {selectedMatch && (
        <MatchDetailModal
          matchId={selectedMatch.matchId}
          subject={selected?.puuid}
          title={`${selectedMatch.map} · ${selectedMatch.mode}`}
          onClose={() => setSelectedMatch(null)}
        />
      )}
    </div>
  );
}
