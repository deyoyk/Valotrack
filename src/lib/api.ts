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
};
