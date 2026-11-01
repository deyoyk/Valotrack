import { invoke } from "@tauri-apps/api/core";

export type Health = {
  ok: boolean;
  appVersion: string;
  clientAvailable: boolean;
  region: string | null;
  lockfilePath: string;
};

export type ClientStatus = {
  available: boolean;
  lockfilePath: string;
  region: string | null;
  error: string | null;
};

export type Settings = {
  region: string;
  liveRefreshSecs: number;
  allowLiveActions: boolean;
  discordRpc: boolean;
  favoriteCrosshairs: number[];
};

export type Agent = {
  name: string;
  uuid: string;
  role: "Duelist" | "Initiator" | "Controller" | "Sentinel";
  color: string;
  portrait: string;
  fullPortrait: string;
};

export type Rank = {
  tier: number;
  name: string;
  group: string;
  color: string;
};

export type Party = {
  id: string;
  color: string;
  number: number;
  size: number;
  members: string[];
  inferred: boolean;
};

export type Player = {
  puuid: string;
  name: string;
  nameHidden: boolean;
  team: string;
  isSelf: boolean;
  title: string | null;
  playerCard: string | null;
  agent: string | null;
  agentId: string | null;
  agentPortrait: string | null;
  agentArt: string | null;
  agentColor: string;
  role: string | null;
  selection: string | null;
  rankTier: number;
  rank: string;
  rankColor: string;
  rankGroup: string;
  rankIcon: string | null;
  rr: number;
  rrEarned: number | null;
  leaderboard: number;
  peakRankTier: number;
  peakRank: string;
  peakColor: string;
  peakIcon: string | null;
  peakAct: string | null;
  previousRank: string;
  winRate: number;
  games: number;
  kd: number | null;
  hsPct: number | null;
  skin: { name: string; icon: string | null } | null;
  weapons: { weapon: string; skin: { name: string; icon: string | null } }[];
  level: number;
  levelHidden: boolean;
  party: { id: string; color: string; number: number; inferred: boolean } | null;
  smurf: boolean;
  smurfReasons: string[];
  topAgents: { agent: string; games: number }[];
  form: string[];
  streak: { type: string; count: number } | null;
  mapWinRate: { winRate: number; games: number } | null;
  statsPending?: boolean;
};

export type TeamStats = {
  avgRankTier: number;
  avgRank: string;
  rankColor: string;
  rankIcon: string | null;
  avgKd: number | null;
  avgWinRate: number | null;
  smurfCount: number;
  size: number;
};

export type LiveBoard = {
  available?: boolean;
  state: string;
  stateLabel: string;
  source: string;
  map: string | null;
  mapSplash: string | null;
  mode: string;
  matchId: string;
  selfTeam: string;
  side: string | null;
  players: Player[];
  teams: Record<string, Player[]>;
  teamStats: Record<string, TeamStats>;
  winProb: number | null;
  parties: Party[];
  score: { ally: number; enemy: number; round: number | null; rounds?: string[] } | null;
  lockProgress: { locked: number; total: number } | null;
  queue?: QueueState;
  notice?: { level: string; action: string; message: string };
};

export type ActionResponse = {
  ok: boolean;
  status?: string;
  message: string;
  queue?: QueueState;
  inQueue?: boolean;
  side?: string;
  map?: string;
  agent?: string;
};

export type InstalockStatus = {
  running: boolean;
  status: string;
  message: string;
  agent: string | null;
  mode: string;
  side: string | null;
  map: string | null;
};

export type QueueState = {
  available: boolean;
  partyId?: string;
  queueId?: string | null;
  queueName?: string | null;
  eligible?: { id: string; name: string }[];
  state?: string;
  inQueue?: boolean;
  queuedAt?: number | null;
  queueElapsed?: number;
  partySize?: number;
  isOwner?: boolean;
  allReady?: boolean;
  message?: string;
};

export type CareerMatch = {
  matchId: string;
  map: string;
  mapSplash: string | null;
  mode: string;
  startMillis: number;
  result: "Victory" | "Defeat" | "Draw";
  team: string;
  score: number;
  opponentScore: number | null;
  agent: string;
  agentPortrait: string;
  agentColor: string;
  kills: number;
  deaths: number;
  assists: number;
  kd: number;
  acs: number;
  hsPct: number | null;
  partySize: number;
  scores: Record<string, number>;
  teammates: CareerPlayer[];
  opponents: CareerPlayer[];
  rrDelta?: number | null;
  rrAfter?: number | null;
  rankAfter?: string;
  rankColor?: string;
};

export type CareerPlayer = {
  puuid: string;
  name: string;
  rankTier: number;
  rank: string;
  rankColor: string;
  agent: string;
  agentPortrait: string;
  agentColor: string;
  level: number;
  kills: number;
  deaths: number;
  assists: number;
  acs: number;
  shotsHit: number;
  headshots: number;
};

export type CareerProfile = {
  source: string;
  puuid: string;
  averages: {
    games: number;
    wins: number;
    winRate: number;
    kills: number;
    deaths: number;
    assists: number;
    kd: number;
    hsPct: number | null;
  };
  coPlayers: {
    puuid: string;
    name: string;
    sharedMatches: number;
    agents: string[];
    isParty: boolean;
  }[];
  agentPool: {
    agent: string;
    games: number;
    wins: number;
    winRate: number;
  }[];
  mapStats: {
    map: string;
    games: number;
    wins: number;
    winRate: number;
  }[];
  matches: CareerMatch[];
  encounter?: EncounterPlayer | null;
};

export type EncounterPlayer = {
  puuid: string;
  name: string;
  withCount: number;
  againstCount: number;
  winsWith: number;
  lossesWith: number;
  winsAgainst: number;
  lossesAgainst: number;
  rank: string | null;
  rankColor: string | null;
  peakRank: string | null;
  level: number;
  kd: number | null;
  winRate: number | null;
  agent: string | null;
  agentPortrait: string | null;
  agentColor: string | null;
  lastSeen: number;
  lastMatch: string;
};

export type MatchPlayer = {
  puuid: string;
  name: string;
  team: string;
  agent: string;
  agentPortrait: string | null;
  agentColor: string;
  kills: number;
  deaths: number;
  assists: number;
  kd: number;
  acs: number;
  hsPct: number | null;
  adr: number | null;
  firstBloods: number | null;
  firstDeaths: number | null;
  multiKills: number | null;
  aces: number | null;
  plants: number | null;
  defuses: number | null;
  rankTier: number;
  rank: string;
  rankColor: string;
  rankIcon: string | null;
  rr: number;
  leaderboard: number;
  peakRankTier: number;
  peakRank: string;
  peakColor: string;
  peakIcon: string | null;
  level: number;
  isSubject?: boolean;
  isMatchMvp?: boolean;
  isTeamMvp?: boolean;
};

export type MatchDetail = {
  matchId: string;
  map: string;
  mapSplash: string | null;
  mode: string;
  scores: Record<string, number>;
  result: string;
  players: MatchPlayer[];
  teamStats: Record<
    string,
    { avgRankTier: number; avgRank: string; avgRankColor: string; rankIcon: string | null }
  >;
  rounds: { n: number; won: boolean; how: string; kills: number; died: boolean }[];
};

export type Performance = {
  owner: string | null;
  accounts: { puuid: string; riotId: string | null; matches: number; lastSeenAt: number }[];
  points: {
    matchId: string;
    ts: number;
    map: string;
    result: string;
    delta: number;
    tier: number;
    rr: number;
  }[];
  recent: {
    matchId: string;
    ts: number;
    map: string;
    result: string;
    delta: number;
    tier: number;
    rr: number;
  }[];
  summary: {
    games: number;
    wins: number;
    losses: number;
    winRate: number;
    avgDelta: number;
    netRr: number;
    maps: { map: string; games: number; wins: number; winRate: number; delta: number }[];
    streak: { type: string; count: number } | null;
    tilt: { active: boolean; level: string; message: string } | null;
  };
  insights: { title: string; text: string; tone: string }[];
  sessions: {
    id: string;
    start: number;
    end: number;
    games: number;
    wins: number;
    losses: number;
    winRate: number;
    netRr: number;
  }[];
  rank: string;
  rankColor: string;
  rankTier: number;
  rr: number;
  currentAccountPuuid?: string | null;
  manualSessions?: SessionList;
  matchMeta?: Record<string, MatchNote>;
};

export type Inventory = {
  available: boolean;
  totalVp: number;
  usdApprox: number;
  wallet: { vp: number; rad: number; kc: number };
  counts: Record<string, number>;
  tiers: Record<string, { skins: number; vp: number }>;
  top: { name: string; icon: string | null; vp: number; tier: string }[];
  cachedAt: number;
};

export type EncounterList = {
  players: EncounterPlayer[];
  accountCount: number;
  scope: string;
};

export type SessionView = {
  id: string;
  goalType: "wins" | "games" | "rr";
  target: number;
  startedAt: number;
  endedAt: number | null;
  active: boolean;
  games: number;
  wins: number;
  losses: number;
  winRate: number;
  netRr: number;
  current: number;
  progress: number;
  complete: boolean;
};

export type SessionList = {
  owner: string;
  active: SessionView | null;
  sessions: SessionView[];
};

export type MatchNote = {
  matchId: string;
  note: string;
  tags: string[];
  updatedAt?: number;
};

export type OfflineStatus = {
  running: boolean;
  active: boolean;
  connected: boolean;
  status: string;
  enabled: boolean;
  configPort: number | null;
  chatPort: number | null;
  message: string;
};

export const api = {
  health: () => invoke<Health>("health"),
  clientStatus: () => invoke<ClientStatus>("client_status"),
  liveBoard: () => invoke<LiveBoard>("live_board"),
  queueState: () => invoke<QueueState>("queue_state"),
  agents: () => invoke<{ agents: Agent[]; count: number }>("agents"),
  settings: () => invoke<Settings>("settings_get"),
  saveSettings: (settings: Settings) => invoke<Settings>("settings_set", { settings }),
  queueAction: (action: string, queueId: string | null, dryRun: boolean) =>
    invoke<ActionResponse>("queue_action", { action, queueId, dryRun }),
  dodge: (dryRun: boolean) => invoke<ActionResponse>("dodge", { dryRun }),
  checkSide: () => invoke<ActionResponse>("check_side"),
  instalockNow: (agent: string, mode: string, dryRun: boolean) =>
    invoke<ActionResponse>("instalock_now", { agent, mode, dryRun }),
  instalockStart: (
    agent: string,
    mode: string,
    delay: number,
    perMap: Record<string, string>,
  ) => invoke<ActionResponse & { running?: boolean }>("instalock_start", { agent, mode, delay, perMap }),
  instalockStop: () => invoke<{ ok: boolean; running: boolean }>("instalock_stop"),
  instalockStatus: () => invoke<InstalockStatus>("instalock_status"),
  profile: (puuid: string) => invoke<CareerProfile>("profile", { puuid }),
  matchDetail: (matchId: string, subject?: string) =>
    invoke<MatchDetail>("match_detail", { matchId, subject }),
  performance: () => invoke<Performance>("performance"),
  encounters: (scope = "current") => invoke<EncounterList>("encounters", { scope }),
  encounter: (puuid: string) => invoke<EncounterPlayer | null>("encounter", { puuid }),
  inventory: () => invoke<Inventory>("inventory"),
  offlineStatus: () => invoke<OfflineStatus>("offline_status"),
  offlineLaunch: (status: string) =>
    invoke<{ ok: boolean; status: string; message: string }>("offline_launch", { status }),
  offlineSetStatus: (status: string) =>
    invoke<{ ok: boolean; status: string; enabled: boolean; message: string }>(
      "offline_set_status",
      { status },
    ),
  offlineSetEnabled: (enabled: boolean) =>
    invoke<{ ok: boolean; status: string; enabled: boolean; message: string }>(
      "offline_set_enabled",
      { enabled },
    ),
  sessions: () => invoke<SessionList>("sessions_list"),
  sessionStart: (goalType: string, target: number) =>
    invoke<{ ok: boolean; session: SessionView }>("session_start", { goalType, target }),
  sessionEnd: () => invoke<{ ok: boolean; session: SessionView }>("session_end"),
  sessionDelete: (id: string) => invoke<{ ok: boolean }>("session_delete", { id }),
  matchNote: (matchId: string) => invoke<MatchNote>("match_note", { matchId }),
  matchNoteUpdate: (matchId: string, note: string, tags: string[]) =>
    invoke<MatchNote>("match_note_update", { matchId, note, tags }),
};
