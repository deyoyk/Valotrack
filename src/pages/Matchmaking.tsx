import { type CSSProperties, useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  Check,
  Clock3,
  Copy,
  Crosshair,
  Link2,
  Loader2,
  Lock,
  Play,
  RefreshCw,
  Search,
  ShieldAlert,
  Swords,
  Timer,
  Unlock,
  UserPlus,
  Users,
  X,
} from "lucide-react";
import {
  ActionResponse,
  Agent,
  api,
  InstalockStatus,
  PartyDetails,
  QueueState,
  Settings,
} from "../lib/api";
import { cn } from "../lib/format";
import { usePoll } from "../lib/usePoll";

const MODES = [
  { id: "competitive", label: "Competitive" },
  { id: "unrated", label: "Unrated" },
  { id: "swiftplay", label: "Swiftplay" },
  { id: "deathmatch", label: "Deathmatch" },
  { id: "spikerush", label: "Spike Rush" },
  { id: "ggteam", label: "Escalation" },
  { id: "hurm", label: "Team Deathmatch" },
];

const MAPS = [
  "Ascent",
  "Bind",
  "Haven",
  "Split",
  "Lotus",
  "Sunset",
  "Abyss",
  "Breeze",
  "Icebox",
  "Fracture",
  "Pearl",
  "Corrode",
  "Summit",
];

const ROLES = ["Duelist", "Initiator", "Controller", "Sentinel"] as const;

export default function Matchmaking() {
  const [queue, setQueue] = useState<QueueState | null>(null);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [selectedAgent, setSelectedAgent] = useState("Jett");
  const [search, setSearch] = useState("");
  const [role, setRole] = useState<string>("all");
  const [mode, setMode] = useState("lock");
  const [delay, setDelay] = useState(6);
  const [perMapEnabled, setPerMapEnabled] = useState(false);
  const [perMap, setPerMap] = useState<Record<string, string>>({});
  const [dryRun, setDryRun] = useState(true);
  const [instalock, setInstalock] = useState<InstalockStatus | null>(null);
  const [result, setResult] = useState<ActionResponse | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [party, setParty] = useState<PartyDetails | null>(null);
  const [inviteName, setInviteName] = useState("");
  const [inviteTag, setInviteTag] = useState("");
  const [joinCode, setJoinCode] = useState("");
  const [partyCode, setPartyCode] = useState("");

  const loadQueue = useCallback(async () => {
    try {
      setQueue(await api.queueState());
    } catch {
      setQueue({ available: false, message: "Riot client offline" });
    }
  }, []);

  const loadStatus = useCallback(async () => {
    try {
      setInstalock(await api.instalockStatus());
    } catch {
      void 0;
    }
  }, []);

  const loadParty = useCallback(async () => {
    try {
      setParty(await api.partyDetails());
    } catch {
      setParty(null);
    }
  }, []);

  useEffect(() => {
    Promise.all([api.agents(), api.settings(), api.instalockStatus()])
      .then(([catalog, config, status]) => {
        setAgents(catalog.agents);
        setSettings(config);
        setInstalock(status);
        setDryRun(!config.allowLiveActions);
      })
      .catch(() => void 0);
  }, []);

  usePoll(loadQueue, 5000);
  usePoll(loadStatus, 2000);
  usePoll(loadParty, 5000);

  const eligible = useMemo(() => {
    const valid = new Set((queue?.eligible ?? []).map((item) => item.id));
    return queue?.available && valid.size > 0
      ? MODES.filter((item) => valid.has(item.id))
      : MODES;
  }, [queue]);

  const filteredAgents = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return agents.filter((agent) => {
      if (role !== "all" && agent.role !== role) return false;
      if (!needle) return true;
      return agent.name.toLowerCase().includes(needle) || agent.role.toLowerCase().includes(needle);
    });
  }, [agents, role, search]);

  async function run(label: string, task: () => Promise<ActionResponse>) {
    setBusy(label);
    setResult(null);
    try {
      const response = await task();
      setResult(response);
      if (response.queue) setQueue(response.queue);
      else void loadQueue();
      void loadStatus();
    } catch (cause) {
      setResult({
        ok: false,
        message: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setBusy(null);
    }
  }

  async function arm() {
    setBusy("arm");
    try {
      const response = await api.instalockStart(
        selectedAgent,
        mode,
        delay,
        perMapEnabled ? perMap : {},
      );
      setResult({ ...response, message: response.message });
      await loadStatus();
    } catch (cause) {
      setResult({
        ok: false,
        message: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setBusy(null);
    }
  }

  async function stopInstalock() {
    setBusy("stop");
    try {
      await api.instalockStop();
      await loadStatus();
      setResult({ ok: true, message: "Instalock disarmed." });
    } finally {
      setBusy(null);
    }
  }

  async function enableLiveActions() {
    if (!settings) return;
    if (
      !window.confirm(
        "Enable live client actions? Valotrack will be able to start and cancel queues, dodge agent select and instalock. This may violate Riot's Terms of Service.",
      )
    ) {
      return;
    }
    setBusy("enable");
    try {
      const next = await api.saveSettings({ ...settings, allowLiveActions: true });
      setSettings(next);
      setDryRun(false);
      setResult({ ok: true, message: "Live client actions enabled." });
    } catch (cause) {
      setResult({
        ok: false,
        message: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setBusy(null);
    }
  }

  async function partyAction(
    label: string,
    task: () => Promise<{ ok: boolean; message: string; code?: string }>,
  ) {
    setBusy(label);
    try {
      const response = await task();
      setResult({ ok: response.ok, message: response.message });
      if (response.code) setPartyCode(response.code);
      await loadParty();
    } catch (cause) {
      setResult({
        ok: false,
        message: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setBusy(null);
    }
  }

  const inQueue = queue?.inQueue === true;
  const queueLabel = queue?.queueName ?? "Competitive";
  const liveActions = settings?.allowLiveActions ?? false;

  return (
    <div className="page matchmaking-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Swords size={12} /> match control
          </span>
          <h1>Queue. Lock. Go.</h1>
          <p>
            Party state, game mode and agent select controls wired straight into the
            local client. Every action previews first unless you turn live actions on.
          </p>
        </div>
        <div className="head-status">
          <div className={queue?.available ? "status-pill online" : "status-pill"}>
            <span className="dot" />
            {queue?.available ? "party ready" : "no party"}
          </div>
          <span className="checked">
            <ShieldAlert size={12} />
            {liveActions ? "live actions enabled" : "dry-run enforced"}
          </span>
        </div>
      </header>

      {result && (
        <div className={cn("action-result", result.ok ? "ok" : "error")}>
          {result.ok ? <Check size={16} /> : <AlertTriangle size={16} />}
          <span>{result.message}</span>
          <button className="icon-button" onClick={() => setResult(null)}>
            <X size={14} />
          </button>
        </div>
      )}

      <div className="match-grid">
        <section className="panel queue-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Party</span>
              <h2>{inQueue ? "Searching" : queue?.available ? "In lobby" : "No party"}</h2>
            </div>
            <button className="icon-button" onClick={() => loadQueue()} title="Refresh party">
              <RefreshCw size={15} />
            </button>
          </div>

          {queue?.available ? (
            <>
              {!liveActions && (
                <div className="live-actions-note">
                  <ShieldAlert size={17} />
                  <div>
                    <strong>Preview mode</strong>
                    <span>Queue, dodge and instalock only simulate until live controls are on.</span>
                  </div>
                  <button
                    className="button primary small"
                    onClick={enableLiveActions}
                    disabled={busy !== null}
                  >
                    {busy === "enable" ? (
                      <Loader2 size={13} className="spin" />
                    ) : null}
                    Enable live
                  </button>
                </div>
              )}

              <div className="party-meta">
                <div>
                  <Users size={14} />
                  <span>{queue.partySize ?? 1} in party</span>
                </div>
                <div>
                  <Clock3 size={14} />
                  <span>{inQueue ? "in queue" : queue.isOwner ? "you own the party" : "member"}</span>
                </div>
                <div>
                  <span className={queue.allReady ? "ready-dot on" : "ready-dot"} />
                  <span>{queue.allReady ? "all ready" : "waiting on ready"}</span>
                </div>
              </div>

              {inQueue && (
                <div className="queue-live">
                  <div className="queue-radar">
                    <i />
                    <i />
                    <i />
                  </div>
                  <div>
                    <strong>{queueLabel}</strong>
                    <span>
                      {queue.queueElapsed !== undefined
                        ? `${Math.floor(queue.queueElapsed / 60)}m ${Math.round(queue.queueElapsed % 60)}s elapsed`
                        : "Matchmaking in progress"}
                    </span>
                  </div>
                  <Timer size={19} />
                </div>
              )}

              <span className="field-label">Game mode</span>
              <div className="mode-grid">
                {eligible.map((item) => (
                  <button
                    key={item.id}
                    className={cn("mode-pill", queue.queueId === item.id && "active")}
                    onClick={() =>
                      run("mode", () => api.queueAction("select", item.id, dryRun))
                    }
                    disabled={busy !== null || inQueue}
                  >
                    {item.label}
                  </button>
                ))}
              </div>

              <div className="queue-actions">
                {inQueue ? (
                  <button
                    className="button danger"
                    onClick={() => run("stop", () => api.queueAction("stop", null, dryRun))}
                    disabled={busy !== null}
                  >
                    <X size={15} /> Cancel queue
                  </button>
                ) : (
                  <button
                    className="button primary"
                    onClick={() => run("start", () => api.queueAction("start", null, dryRun))}
                    disabled={busy !== null || !queue.isOwner || !queue.allReady}
                  >
                    <Play size={15} /> Start queue
                  </button>
                )}
                <span className="queue-note">
                  {!queue.isOwner
                    ? "Only the party owner can start."
                    : !queue.allReady
                      ? "Everyone must be ready."
                      : `${queueLabel} · ${dryRun ? "preview" : "live"}`}
                </span>
              </div>
            </>
          ) : (
            <div className="inline-empty">
              <Swords size={22} />
              <strong>Get into the VALORANT lobby</strong>
              <span>Party state and game modes appear when Riot is signed in.</span>
            </div>
          )}

          <div className="dry-toggle">
            <div>
              <strong>Dry run</strong>
              <span>
                {liveActions
                  ? "Preview actions without touching the client."
                  : "Enable live controls above to use queue actions."}
              </span>
            </div>
            <label className="mini-switch">
              <input
                type="checkbox"
                checked={dryRun}
                disabled={!liveActions}
                onChange={(event) => setDryRun(event.target.checked)}
              />
              <i />
            </label>
          </div>
        </section>

        <section className="panel select-tools">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Agent select</span>
              <h2>Side check</h2>
            </div>
            <Crosshair size={19} className="panel-icon" />
          </div>
          <p className="tool-copy">
            Read which side you start on, or dodge the lobby before the round begins.
          </p>
          <div className="tool-buttons">
            <button
              className="button"
              onClick={() => run("side", () => api.checkSide())}
              disabled={busy !== null}
            >
              <Swords size={15} /> Check side
            </button>
            <button
              className="button danger"
              onClick={() => {
                if (window.confirm("Dodge this agent select?")) {
                  void run("dodge", () => api.dodge(dryRun));
                }
              }}
              disabled={busy !== null}
            >
              <X size={15} /> Dodge
            </button>
          </div>
          <div className="safety-list">
            <div>
              <Lock size={13} />
              <span>Actions require live mode and a signed-in client.</span>
            </div>
            <div>
              <AlertTriangle size={13} />
              <span>Dodging can lead to queue penalties from Riot.</span>
            </div>
          </div>
        </section>

        <section className="panel instalock-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Auto select</span>
              <h2>Instalock</h2>
            </div>
            <div className={cn("instalock-state", instalock?.running && "running")}>
              {instalock?.running ? <Loader2 size={12} className="spin" /> : <ShieldAlert size={12} />}
              {instalock?.status ?? "idle"}
            </div>
          </div>

          {instalock?.message && (
            <div className="instalock-message">
              <span>{instalock.message}</span>
              {instalock.agent && <strong>{instalock.agent}</strong>}
              {instalock.map && <em>{instalock.map}</em>}
              {instalock.side && <em>{instalock.side}</em>}
            </div>
          )}

          <div className="agent-toolbar">
            <div className="search-box">
              <Search size={14} />
              <input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Search agents"
              />
            </div>
            <div className="role-tabs">
              <button className={role === "all" ? "active" : ""} onClick={() => setRole("all")}>
                All
              </button>
              {ROLES.map((item) => (
                <button
                  key={item}
                  className={role === item ? "active" : ""}
                  onClick={() => setRole(item)}
                >
                  {item}
                </button>
              ))}
            </div>
          </div>

          <div className="agent-grid">
            {filteredAgents.map((agent) => (
              <button
                key={agent.uuid}
                className={cn("agent-card", selectedAgent === agent.name && "selected")}
                onClick={() => setSelectedAgent(agent.name)}
                style={{ "--agent-color": agent.color } as CSSProperties}
              >
                <img src={agent.portrait} alt={agent.name} loading="lazy" />
                <span>{agent.name}</span>
                <small>{agent.role}</small>
              </button>
            ))}
          </div>

          <div className="instalock-options">
            <div className="segmented">
              <button className={mode === "lock" ? "active" : ""} onClick={() => setMode("lock")}>
                Lock
              </button>
              <button className={mode === "hover" ? "active" : ""} onClick={() => setMode("hover")}>
                Hover
              </button>
            </div>
            <label className="delay-field">
              <span>Delay</span>
              <input
                type="number"
                min={0}
                max={30}
                step={1}
                value={delay}
                onChange={(event) => setDelay(Number(event.target.value))}
              />
              <em>sec</em>
            </label>
            <label className="check-line">
              <input
                type="checkbox"
                checked={perMapEnabled}
                onChange={(event) => setPerMapEnabled(event.target.checked)}
              />
              <span>Per-map agent overrides</span>
            </label>
          </div>

          {perMapEnabled && (
            <div className="per-map-grid">
              {MAPS.map((map) => (
                <label key={map}>
                  <span>{map}</span>
                  <select
                    value={perMap[map.toLowerCase()] ?? ""}
                    onChange={(event) =>
                      setPerMap((current) => ({
                        ...current,
                        [map.toLowerCase()]: event.target.value,
                      }))
                    }
                  >
                    <option value="">{selectedAgent}</option>
                    {agents.map((agent) => (
                      <option key={agent.uuid} value={agent.name}>
                        {agent.name}
                      </option>
                    ))}
                  </select>
                </label>
              ))}
            </div>
          )}

          <div className="instalock-actions">
            {instalock?.running ? (
              <button className="button danger" onClick={stopInstalock} disabled={busy !== null}>
                <X size={15} /> Disarm
              </button>
            ) : (
              <button className="button primary" onClick={arm} disabled={busy !== null}>
                <Crosshair size={15} /> Arm {selectedAgent}
              </button>
            )}
            <button
              className="button"
              onClick={() =>
                run("now", () => api.instalockNow(selectedAgent, mode, dryRun))
              }
              disabled={busy !== null}
            >
              <ZapIcon /> {mode === "lock" ? "Lock now" : "Hover now"}
            </button>
            <span className="queue-note">
              {dryRun ? "preview only" : "live"} · {mode} · {selectedAgent}
              {delay > 0 ? ` · ${delay}s delay` : ""}
            </span>
          </div>
        </section>
      </div>

      <section className="panel party-advanced">
        <div className="panel-heading">
          <div>
            <span className="panel-label">Party tools</span>
            <h2>Advanced controls</h2>
          </div>
          <Link2 size={19} className="panel-icon" />
        </div>

        {party?.available ? (
          <>
            <div className="party-members">
              {party.members.map((member) => (
                <div className="party-member" key={member.puuid}>
                  <span className="party-member-avatar">
                    {member.card ? <img src={member.card} alt="" /> : <Users size={16} />}
                  </span>
                  <div>
                    <strong>
                      {member.name}
                      {member.tag ? `#${member.tag}` : ""}
                      {member.isSelf ? " · you" : ""}
                    </strong>
                    <span>
                      {member.isOwner ? "owner · " : ""}
                      level {member.accountLevel || "?"}
                      {member.title ? ` · ${member.title}` : ""}
                    </span>
                  </div>
                  <em className={member.isReady ? "ready" : ""}>
                    {member.isReady ? "ready" : "not ready"}
                  </em>
                </div>
              ))}
            </div>

            <div className="party-tools-row">
              <button
                className="button"
                onClick={() =>
                  partyAction("ready", () => api.partyReady(!party.selfReady, dryRun))
                }
                disabled={busy !== null}
              >
                {busy === "ready" ? <Loader2 size={14} className="spin" /> : <Check size={14} />}
                {party.selfReady ? "Unready" : "Ready up"}
              </button>
              <button
                className="button"
                onClick={() =>
                  partyAction("accessibility", () =>
                    api.partyAccessibility(party.accessibility !== "OPEN", dryRun),
                  )
                }
                disabled={busy !== null || !party.isOwner}
              >
                {party.accessibility === "OPEN" ? <Unlock size={14} /> : <Lock size={14} />}
                {party.accessibility === "OPEN" ? "Close party" : "Open party"}
              </button>
              <span className="queue-note">
                {party.accessibility === "OPEN" ? "anyone can request" : "invite only"} ·{" "}
                {dryRun ? "preview" : "live"}
              </span>
            </div>

            <div className="party-code-row">
              <div>
                <span className="panel-label">Invite code</span>
                <code>{partyCode || party.inviteCode || "Not generated"}</code>
              </div>
              <button
                className="button small"
                onClick={() =>
                  partyAction("code", () => api.partyGenerateCode(dryRun))
                }
                disabled={busy !== null}
              >
                {busy === "code" ? <Loader2 size={13} className="spin" /> : <RefreshCw size={13} />}
                Generate
              </button>
              <button
                className="button small"
                onClick={() => {
                  const code = partyCode || party.inviteCode;
                  if (code) {
                    void navigator.clipboard.writeText(code);
                    setResult({ ok: true, message: "Invite code copied." });
                  }
                }}
                disabled={!(partyCode || party.inviteCode)}
              >
                <Copy size={13} /> Copy
              </button>
            </div>

            <div className="party-forms">
              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  if (!inviteName.trim() || !inviteTag.trim()) {
                    setResult({ ok: false, message: "Enter a Riot name and tagline." });
                    return;
                  }
                  void partyAction("invite", () =>
                    api.partyInvite(inviteName.trim(), inviteTag.trim(), dryRun),
                  );
                }}
              >
                <span className="field-label">Invite by Riot ID</span>
                <div>
                  <input
                    value={inviteName}
                    onChange={(event) => setInviteName(event.target.value)}
                    placeholder="Name"
                  />
                  <span>#</span>
                  <input
                    value={inviteTag}
                    onChange={(event) => setInviteTag(event.target.value)}
                    placeholder="Tag"
                  />
                  <button className="button primary" type="submit" disabled={busy !== null}>
                    <UserPlus size={14} /> Invite
                  </button>
                </div>
              </form>
              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  if (!joinCode.trim()) return;
                  void partyAction("join", () => api.partyJoinCode(joinCode.trim(), dryRun));
                }}
              >
                <span className="field-label">Join another party</span>
                <div>
                  <input
                    value={joinCode}
                    onChange={(event) => setJoinCode(event.target.value)}
                    placeholder="Paste invite code"
                  />
                  <button className="button" type="submit" disabled={busy !== null}>
                    {busy === "join" ? <Loader2 size={14} className="spin" /> : <Link2 size={14} />}
                    Join
                  </button>
                </div>
              </form>
            </div>
          </>
        ) : (
          <div className="party-forms single">
            <form
              onSubmit={(event) => {
                event.preventDefault();
                if (!joinCode.trim()) return;
                void partyAction("join", () => api.partyJoinCode(joinCode.trim(), dryRun));
              }}
            >
              <span className="field-label">Join a party by invite code</span>
              <div>
                <input
                  value={joinCode}
                  onChange={(event) => setJoinCode(event.target.value)}
                  placeholder="Paste invite code"
                />
                <button className="button" type="submit" disabled={busy !== null}>
                  {busy === "join" ? <Loader2 size={14} className="spin" /> : <Link2 size={14} />}
                  Join
                </button>
              </div>
            </form>
          </div>
        )}
      </section>
    </div>
  );
}

function ZapIcon() {
  return (
    <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M13 2 4.5 13.5H11L9.5 22 19 10h-6.5L13 2Z" />
    </svg>
  );
}
