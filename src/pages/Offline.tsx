import { type CSSProperties, useCallback, useEffect, useState } from "react";
import { confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  Check,
  Loader2,
  Moon,
  Play,
  Radio,
  ShieldCheck,
  Smartphone,
  Square,
  WifiOff,
  X,
} from "lucide-react";
import { api, OfflineStatus } from "../lib/api";
import { cn } from "../lib/format";
import { usePoll } from "../lib/usePoll";

const MODES = [
  {
    id: "offline",
    label: "Offline",
    description: "Friends see the grey offline dot. You can still queue and play.",
    icon: WifiOff,
    color: "#8b93a1",
  },
  {
    id: "away",
    label: "Away",
    description: "Show the yellow away status while you play.",
    icon: Moon,
    color: "#e8b45a",
  },
  {
    id: "mobile",
    label: "Mobile",
    description: "Appear on the mobile phone icon.",
    icon: Smartphone,
    color: "#4d8dff",
  },
  {
    id: "online",
    label: "Online",
    description: "Turn the relay off and appear normally.",
    icon: Radio,
    color: "#49d6a3",
  },
];

export default function Offline() {
  const [status, setStatus] = useState<OfflineStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);

  const load = useCallback(async () => {
    try {
      setStatus(await api.offlineStatus());
    } catch {
      void 0;
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);
  usePoll(load, 5000);

  async function setStatusMode(mode: string) {
    setBusy(mode);
    try {
      const result = await api.offlineSetStatus(mode);
      setMessage({ ok: result.ok, text: result.message });
      await load();
    } catch (cause) {
      setMessage({ ok: false, text: cause instanceof Error ? cause.message : String(cause) });
    } finally {
      setBusy(null);
    }
  }

  async function stop() {
    const confirmed = await confirmDialog(
      "Stop the offline relay? Riot chat will disconnect until you launch through it again.",
      { title: "Stop offline relay", kind: "warning" },
    );
    if (!confirmed) {
      return;
    }
    setBusy("stop");
    try {
      const next = await api.offlineStop();
      setStatus(next);
      setMessage({ ok: true, text: "Relay stopped." });
    } catch (cause) {
      setMessage({ ok: false, text: cause instanceof Error ? cause.message : String(cause) });
    } finally {
      setBusy(null);
    }
  }

  async function launch() {
    const confirmed = await confirmDialog(
      "Launch VALORANT through the offline relay? This closes the running Riot Client and game first.",
      { title: "Launch through offline relay", kind: "warning" },
    );
    if (!confirmed) {
      return;
    }
    setBusy("launch");
    try {
      const result = await api.offlineLaunch(status?.status ?? "offline");
      setMessage({ ok: result.ok, text: result.message });
      await load();
    } catch (cause) {
      setMessage({ ok: false, text: cause instanceof Error ? cause.message : String(cause) });
    } finally {
      setBusy(null);
    }
  }

  const current = status?.status ?? "offline";

  return (
    <div className="page offline-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <WifiOff size={12} /> appear offline
          </span>
          <h1>Vanish from the list.</h1>
          <p>
            Play with the friends list seeing offline, away or mobile. The relay rewrites
            your outgoing presence on this machine - no third-party binary and no account.
            It keeps running after you close Valotrack.
          </p>
        </div>
        <div className="head-status">
          <div className={status?.connected ? "status-pill online" : "status-pill"}>
            <span className="dot" />
            {status?.connected ? "relay connected" : status?.running ? "relay ready" : "relay stopped"}
          </div>
          <span className="checked">
            {status?.configPort ? `config :${status.configPort}` : "not listening"}
          </span>
        </div>
      </header>

      {message && (
        <div className={cn("action-result", message.ok ? "ok" : "error")}>
          {message.ok ? <Check size={16} /> : <AlertTriangle size={16} />}
          <span>{message.text}</span>
          <button className="icon-button" onClick={() => setMessage(null)}>
            <X size={14} />
          </button>
        </div>
      )}

      <section className="offline-hero">
        <div className="offline-state">
          <div className={cn("offline-orbit", status?.active && "active")}>
            <span />
            <span />
            <span />
            {status?.status === "mobile" ? <Smartphone size={26} /> : <WifiOff size={26} />}
          </div>
          <div>
            <span className="profile-kicker">Current visibility</span>
            <h2>{MODES.find((mode) => mode.id === current)?.label ?? "Offline"}</h2>
            <p>{status?.message ?? "Start the relay to control how friends see you."}</p>
          </div>
        </div>
        <div className="offline-actions">
          <button className="button primary" onClick={launch} disabled={busy !== null}>
            {busy === "launch" ? <Loader2 size={15} className="spin" /> : <Play size={15} />}
            Launch with relay
          </button>
          {!status?.running && (
            <button
              className="button"
              onClick={async () => {
                setBusy("start");
                try {
                  const next = await api.offlineStart();
                  setStatus(next);
                  setMessage({ ok: true, text: "Relay started. Launch VALORANT to connect it." });
                } catch (cause) {
                  setMessage({
                    ok: false,
                    text: cause instanceof Error ? cause.message : String(cause),
                  });
                } finally {
                  setBusy(null);
                }
              }}
              disabled={busy !== null}
            >
              {busy === "start" ? <Loader2 size={15} className="spin" /> : <Radio size={15} />}
              Start relay only
            </button>
          )}
          {status?.running && (
            <button className="button" onClick={stop} disabled={busy !== null}>
              {busy === "stop" ? <Loader2 size={15} className="spin" /> : <Square size={14} />}
              Stop relay
            </button>
          )}
          <button
            className="button"
            onClick={() => setStatusMode("online")}
            disabled={busy !== null || !status?.running}
          >
            <Radio size={15} /> Go online
          </button>
        </div>
      </section>

      <div className="offline-grid">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Visibility</span>
              <h2>How friends see you</h2>
            </div>
          </div>
          <div className="visibility-grid">
            {MODES.map((mode) => {
              const Icon = mode.icon;
              return (
                <button
                  key={mode.id}
                  className={cn("visibility-card", current === mode.id && "active")}
                  onClick={() => setStatusMode(mode.id)}
                  disabled={busy !== null || !status?.running}
                  style={{ "--mode-color": mode.color } as CSSProperties}
                >
                  <Icon size={19} />
                  <strong>{mode.label}</strong>
                  <span>{mode.description}</span>
                  {current === mode.id && <em>active</em>}
                </button>
              );
            })}
          </div>
          <div className="offline-footnote">
            <ShieldCheck size={14} />
            <span>
              Status changes apply instantly to the live chat connection. If you are not
              connected yet, launch VALORANT through the relay.
            </span>
          </div>
        </section>

        <section className="panel offline-how">
          <div className="panel-heading">
            <div>
              <span className="panel-label">How it works</span>
              <h2>Local presence relay</h2>
            </div>
            <ShieldCheck size={20} className="panel-icon" />
          </div>
          <ol>
            <li>
              <span>1</span>
              <div>
                <strong>Riot points chat at this app</strong>
                <p>
                  A local config relay rewrites the chat host to an encrypted loopback
                  proxy. No DNS or hosts file changes.
                </p>
              </div>
            </li>
            <li>
              <span>2</span>
              <div>
                <strong>Presence is rewritten</strong>
                <p>
                  Your outgoing XMPP presence is stripped of game activity and tagged
                  offline, away or mobile before it leaves your PC.
                </p>
              </div>
            </li>
            <li>
              <span>3</span>
              <div>
                <strong>Everything else keeps working</strong>
                <p>
                  Friends, messages, queues and matches continue to work normally. A pinned
                  friend named Valotrack Active accepts the commands online, offline, away,
                  mobile, status and help in chat.
                </p>
              </div>
            </li>
            <li>
              <span>4</span>
              <div>
                <strong>Close the app, not the relay</strong>
                <p>
                  The relay lives in a small background helper, so closing Valotrack does not
                  change your status. Stop it here when you want to remove it.
                </p>
              </div>
            </li>
          </ol>
          <div className="warn-note">
            <AlertTriangle size={15} />
            <span>
              Launching through the relay closes VALORANT and the Riot Client first. Save
              any settings before continuing. This is a client modification and may
              violate Riot's Terms of Service.
            </span>
          </div>
        </section>
      </div>
    </div>
  );
}
