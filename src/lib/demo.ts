import {
  Agent,
  CareerMatch,
  CareerPlayer,
  CareerProfile,
  LiveBoard,
  MatchDetail,
  MatchPlayer,
  Player,
  TeamStats,
} from "./api";

const RANKS = [
  { tier: 27, name: "Radiant", group: "Radiant", color: "#fff1b8" },
  { tier: 24, name: "Immortal 3", group: "Immortal", color: "#ff4655" },
  { tier: 22, name: "Immortal 1", group: "Immortal", color: "#ff4655" },
  { tier: 21, name: "Ascendant 3", group: "Ascendant", color: "#2fbf71" },
  { tier: 19, name: "Ascendant 1", group: "Ascendant", color: "#2fbf71" },
  { tier: 18, name: "Diamond 3", group: "Diamond", color: "#d977ff" },
  { tier: 15, name: "Platinum 2", group: "Platinum", color: "#4fb6c9" },
  { tier: 12, name: "Gold 1", group: "Gold", color: "#e8c15a" },
  { tier: 9, name: "Silver 3", group: "Silver", color: "#c2c6cc" },
  { tier: 6, name: "Bronze 2", group: "Bronze", color: "#b07a5c" },
];

const NAMES = [
  "NOVAWISP#NA1",
  "PIXELDASH#EUW",
  "KARMADIFF#APAC",
  "TOXICCLUTCH#OCE",
  "NEONAIM#GG",
  "ZENEDGE#KR",
  "VIBEFRAG#GG",
  "RIOTMAIN#OCE",
  "PIXELTTV#OCE",
  "NEONFRAG#GG",
];

const TITLES = [
  "Red Flag",
  "Clutch",
  "Legend",
  "Ace",
  "First Blood",
  "Mastermind",
  "Rookie",
  "Clutch",
  "Legend",
  "Mastermind",
];

const LEVELS = [54, 371, 88, 183, 417, 219, 115, 62, 227, 146];

const PARTY_COLORS = ["#ff4655", "#4d8dff", "#e0b25e", "#62d3a6", "#d977ff"];

const FORM = [
  ["W", "W", "L", "W", "W"],
  ["L", "W", "W", "L", "W"],
  ["W", "L", "W", "W", "L"],
  ["W", "W", "W", "L", "W"],
  ["L", "L", "W", "W", "L"],
  ["W", "W", "L", "W", "W"],
  ["L", "W", "W", "W", "L"],
  ["W", "L", "L", "W", "W"],
  ["W", "W", "W", "W", "L"],
  ["L", "W", "L", "W", "W"],
];

const MAP_SPLASH =
  "https://media.valorant-api.com/maps/7eaecc1b-4337-bbf6-6ab9-04b8f06b3319/splash.png";

function fallbackAgents(): Agent[] {
  return [
    "Jett",
    "Phoenix",
    "Sage",
    "Sova",
    "Viper",
    "Omen",
    "Killjoy",
    "Fade",
    "Astra",
    "Raze",
  ].map((name, index) => ({
    name,
    uuid: `demo-agent-${index}`,
    role: ["Duelist", "Initiator", "Controller", "Sentinel"][index % 4] as Agent["role"],
    color: PARTY_COLORS[index % PARTY_COLORS.length],
    portrait: "",
    fullPortrait: "",
  }));
}

function teamAverage(players: Player[]): TeamStats {
  const ranked = players.filter((player) => player.rankTier > 0);
  const kd = players.map((player) => player.kd ?? 0);
  const wr = players.map((player) => player.winRate);
  const top = [...ranked].sort((a, b) => b.rankTier - a.rankTier)[0];
  return {
    avgRankTier: top?.rankTier ?? 0,
    avgRank: top?.rank ?? "Unranked",
    rankColor: top?.rankColor ?? "#8b93a1",
    rankIcon: null,
    avgKd: kd.length ? kd.reduce((sum, value) => sum + value, 0) / kd.length : null,
    avgWinRate: wr.length ? wr.reduce((sum, value) => sum + value, 0) / wr.length : null,
    smurfCount: players.filter((player) => player.smurf).length,
    size: players.length,
  };
}

export function buildDemoBoard(catalog: Agent[]): LiveBoard {
  const agents = catalog.length >= 10 ? catalog : catalog.length ? catalog : fallbackAgents();
  const players = NAMES.map((name, index) => {
    const agent = agents[index % agents.length];
    const rank = RANKS[index % RANKS.length];
    const rankIndex = RANKS.findIndex((entry) => entry.tier === rank.tier);
    const peak = RANKS[Math.max(0, rankIndex - (index % 2))];
    const team = index < 5 ? "Blue" : "Red";
    const partyIndex = index < 5 ? (index < 2 ? 0 : 1) : index < 8 ? 2 : 3;
    const smurf = index === 2 || index === 7;
    const player: Player = {
      puuid: `demo-${index}`,
      name,
      nameHidden: false,
      team,
      isSelf: index === 0,
      title: TITLES[index],
      playerCard: null,
      agent: agent.name,
      agentId: agent.uuid,
      agentPortrait: agent.portrait || null,
      agentArt: agent.fullPortrait || null,
      agentColor: agent.color || "#ff4655",
      role: agent.role,
      selection: index < 8 ? "locked" : null,
      rankTier: rank.tier,
      rank: rank.name,
      rankColor: rank.color,
      rankGroup: rank.group,
      rankIcon: null,
      rr: [32, 79, 8, 16, 97, 30, 51, 4, 89, 10][index],
      rrEarned: [22, 9, -18, 17, -6, 13, 24, 5, -11, 18][index],
      leaderboard: rank.tier >= 24 ? 4810 + index * 73 : 0,
      peakRankTier: peak.tier,
      peakRank: peak.name,
      peakColor: peak.color,
      peakIcon: null,
      peakAct: `V26 Act ${(index % 3) + 1}`,
      previousRank: RANKS[(index + 4) % RANKS.length].name,
      winRate: [58, 61, 61, 54, 55, 64, 53, 63, 52, 57][index],
      games: [42, 118, 33, 74, 96, 151, 67, 21, 88, 63][index],
      kd: [1.71, 1.24, 1.62, 1.42, 0.84, 1.76, 0.67, 1.58, 1.19, 1.44][index],
      hsPct: [28.4, 24.1, 33.8, 21.7, 26.9, 13.4, 18.2, 27.4, 22.8, 25.1][index],
      skin: null,
      weapons: [],
      level: LEVELS[index],
      levelHidden: index === 1 || index === 6,
      party: {
        id: `party-${partyIndex}`,
        color: PARTY_COLORS[partyIndex],
        number: partyIndex + 1,
        inferred: false,
      },
      smurf,
      smurfReasons: smurf
        ? index === 2
          ? ["low level for Immortal", "K/D well above rank average"]
          : ["low level for Gold", "win rate above rank average"]
        : [],
      topAgents: [
        { agent: agent.name, games: 18 },
        { agent: agents[(index + 3) % agents.length].name, games: 11 },
        { agent: agents[(index + 6) % agents.length].name, games: 7 },
      ],
      form: FORM[index],
      streak: index % 3 === 0 ? { type: "W", count: 3 } : null,
      mapWinRate: { winRate: 60 - index * 2, games: 18 + index },
      statsPending: false,
    };
    return player;
  });

  const teams = {
    Blue: players.filter((player) => player.team === "Blue"),
    Red: players.filter((player) => player.team === "Red"),
  };

  const parties = [0, 1, 2, 3].map((partyIndex) => {
    const members = players.filter((player) => player.party?.number === partyIndex + 1);
    return {
      id: `party-${partyIndex}`,
      color: PARTY_COLORS[partyIndex],
      number: partyIndex + 1,
      size: members.length,
      members: members.map((player) => player.name),
      inferred: false,
    };
  });

  return {
    available: true,
    state: "INGAME",
    stateLabel: "In game",
    source: "demo",
    map: "Ascent",
    mapSplash: MAP_SPLASH,
    mode: "Competitive",
    matchId: "demo-ascent-competitive",
    selfTeam: "Blue",
    side: "Attack",
    players,
    teams,
    teamStats: {
      Blue: teamAverage(teams.Blue),
      Red: teamAverage(teams.Red),
    },
    winProb: 61,
    parties,
    score: {
      ally: 8,
      enemy: 5,
      round: 14,
      rounds: ["W", "W", "L", "W", "L", "W", "W", "L", "W", "W", "L", "W", "W"],
    },
    lockProgress: null,
  };
}

function careerPlayer(player: Player, team: string): CareerPlayer {
  return {
    puuid: player.puuid,
    name: player.name,
    rankTier: player.rankTier,
    rank: player.rank,
    rankColor: player.rankColor,
    agent: player.agent ?? "Jett",
    agentPortrait: player.agentPortrait ?? "",
    agentColor: player.agentColor,
    level: player.level,
    kills: [18, 14, 21, 9, 16, 12, 19, 8, 11, 15][Number(player.puuid.slice(-1)) || 0],
    deaths: [12, 16, 14, 17, 13, 15, 11, 18, 16, 14][Number(player.puuid.slice(-1)) || 0],
    assists: [5, 3, 7, 2, 4, 6, 3, 1, 5, 2][Number(player.puuid.slice(-1)) || 0],
    acs: [286, 214, 301, 142, 238, 197, 274, 128, 186, 222][Number(player.puuid.slice(-1)) || 0],
    shotsHit: 120,
    headshots: 32,
    team,
  } as CareerPlayer & { team: string };
}

function demoMatches(players: Player[]): CareerMatch[] {
  const maps = ["Ascent", "Bind", "Haven", "Split", "Icebox", "Lotus"];
  const results: CareerMatch["result"][] = ["Victory", "Defeat", "Victory", "Victory", "Defeat"];
  const allies = players.slice(0, 5);
  const enemies = players.slice(5);
  return maps.slice(0, 5).map((map, index) => {
    const agent = allies[index % allies.length];
    return {
      matchId: `demo-match-${index + 1}`,
      map,
      mapSplash: MAP_SPLASH,
      mode: index === 3 ? "Swiftplay" : "Competitive",
      startMillis: Date.now() - (index + 1) * 86400000,
      result: results[index],
      team: "Blue",
      score: index === 1 ? 9 : 13,
      opponentScore: index === 1 ? 13 : index === 4 ? 11 : 7,
      agent: agent.agent ?? "Jett",
      agentPortrait: agent.agentPortrait ?? "",
      agentColor: agent.agentColor,
      kills: [24, 17, 21, 19, 14][index],
      deaths: [12, 19, 15, 13, 18][index],
      assists: [6, 4, 8, 3, 5][index],
      kd: [2.0, 0.89, 1.4, 1.46, 0.78][index],
      acs: [312, 186, 274, 251, 168][index],
      hsPct: [31.2, 19.8, 27.4, 24.1, 18.6][index],
      partySize: index % 2 ? 2 : 3,
      scores: { Blue: index === 1 ? 9 : 13, Red: index === 1 ? 13 : index === 4 ? 11 : 7 },
      teammates: allies.slice(1).map((player) => careerPlayer(player, "Blue")),
      opponents: enemies.map((player) => careerPlayer(player, "Red")),
      rrDelta: [24, -19, 18, 15, -21][index],
      rrAfter: [56, 32, 51, 36, 15][index],
      rankAfter: allies[0].rank,
      rankColor: allies[0].rankColor,
    };
  });
}

export function demoProfile(player: Player, catalog: Agent[]): CareerProfile {
  const board = buildDemoBoard(catalog);
  const matches = demoMatches(board.players);
  return {
    source: "demo",
    puuid: player.puuid,
    averages: {
      games: 10,
      wins: 6,
      winRate: 60,
      kills: 18.4,
      deaths: 15.1,
      assists: 5.2,
      kd: 1.22,
      hsPct: 24.8,
    },
    coPlayers: board.players
      .filter((mate) => mate.puuid !== player.puuid)
      .slice(0, 4)
      .map((mate, index) => ({
        puuid: mate.puuid,
        name: mate.name,
        sharedMatches: 6 - index,
        agents: [mate.agent ?? "Jett"],
        isParty: index < 2,
      })),
    agentPool: [
      { agent: player.agent ?? "Jett", games: 10, wins: 6, winRate: 60 },
      { agent: "Omen", games: 7, wins: 4, winRate: 57 },
      { agent: "Sova", games: 5, wins: 3, winRate: 60 },
      { agent: "Killjoy", games: 4, wins: 2, winRate: 50 },
    ],
    mapStats: [
      { map: "Ascent", games: 8, wins: 5, winRate: 63 },
      { map: "Bind", games: 6, wins: 4, winRate: 67 },
      { map: "Haven", games: 5, wins: 2, winRate: 40 },
      { map: "Lotus", games: 4, wins: 2, winRate: 50 },
    ],
    matches,
    encounter: {
      puuid: player.puuid,
      name: player.name,
      withCount: 3,
      againstCount: 4,
      winsWith: 2,
      lossesWith: 1,
      winsAgainst: 3,
      lossesAgainst: 1,
      rank: player.rank,
      rankColor: player.rankColor,
      peakRank: player.peakRank,
      level: player.level,
      kd: player.kd,
      winRate: player.winRate,
      agent: player.agent,
      agentPortrait: player.agentPortrait,
      agentColor: player.agentColor,
      lastSeen: Math.floor(Date.now() / 1000) - 3600,
      lastMatch: "demo-match-1",
    },
  };
}

export function demoMatchDetail(matchId: string, catalog: Agent[]): MatchDetail {
  const board = buildDemoBoard(catalog);
  const players: MatchPlayer[] = board.players.map((player, index) => ({
    puuid: player.puuid,
    name: player.name,
    team: player.team,
    agent: player.agent ?? "Jett",
    agentPortrait: player.agentPortrait,
    agentColor: player.agentColor,
    kills: [24, 17, 21, 19, 14, 12, 19, 8, 11, 15][index],
    deaths: [12, 19, 15, 13, 18, 15, 11, 18, 16, 14][index],
    assists: [6, 4, 8, 3, 5, 6, 3, 1, 5, 2][index],
    kd: [2.0, 0.89, 1.4, 1.46, 0.78, 0.8, 1.73, 0.44, 0.69, 1.07][index],
    acs: [312, 186, 274, 251, 168, 154, 268, 112, 149, 201][index],
    hsPct: [31.2, 19.8, 27.4, 24.1, 18.6, 22.3, 29.1, 16.8, 21.5, 23.8][index],
    adr: [178, 132, 161, 149, 118, 108, 157, 74, 103, 126][index],
    firstBloods: [4, 2, 3, 2, 1, 1, 3, 0, 1, 2][index],
    firstDeaths: [1, 3, 2, 2, 3, 3, 1, 4, 2, 1][index],
    multiKills: [3, 1, 2, 1, 0, 0, 2, 0, 1, 1][index],
    aces: index === 0 ? 1 : 0,
    plants: index % 4 === 2 ? 3 : 0,
    defuses: index % 5 === 1 ? 1 : 0,
    rankTier: player.rankTier,
    rank: player.rank,
    rankColor: player.rankColor,
    rankIcon: player.rankIcon,
    rr: player.rr,
    leaderboard: player.leaderboard,
    peakRankTier: player.peakRankTier,
    peakRank: player.peakRank,
    peakColor: player.peakColor,
    peakIcon: player.peakIcon,
    level: player.level,
    isSubject: index === 0,
    isMatchMvp: index === 0,
    isTeamMvp: index === 0 || index === 5,
  }));
  return {
    matchId,
    map: "Ascent",
    mapSplash: MAP_SPLASH,
    mode: "Competitive",
    scores: { Blue: 13, Red: 11 },
    result: "Victory",
    players,
    teamStats: {
      Blue: {
        avgRankTier: 20,
        avgRank: "Ascendant 1",
        avgRankColor: "#2fbf71",
        rankIcon: null,
      },
      Red: {
        avgRankTier: 18,
        avgRank: "Diamond 3",
        avgRankColor: "#d977ff",
        rankIcon: null,
      },
    },
    rounds: Array.from({ length: 24 }, (_, index) => ({
      n: index + 1,
      won: [true, true, false, true, false, true, true, false, true, true, false, true, true, false, false, true, true, true, false, true, false, true, false, true][index],
      how: index % 4 === 0 ? "spike" : index % 3 === 0 ? "defuse" : "elimination",
      kills: 5 - (index % 3),
      died: index % 2 === 0,
    })),
  };
}
