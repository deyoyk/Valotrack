import { useEffect, useState } from "react";
import {
  Check,
  Database,
  Download,
  Info,
  Loader2,
  RefreshCw,
  Save,
  ShieldCheck,
  SlidersHorizontal,
} from "lucide-react";
import { api, Health, Settings as AppSettings } from "../lib/api";
import { useUpdater } from "../lib/updater";

const REGIONS = [
  { id: "", label: "Auto-detect" },
  { id: "na", label: "North America" },
  { id: "eu", label: "Europe" },
  { id: "ap", label: "Asia Pacific" },
  { id: "kr", label: "Korea" },
  { id: "latam", label: "Latin America" },
  { id: "br", label: "Brazil" },
];

export default function Settings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [saved, setSaved] = useState<AppSettings | null>(null);
  const [health, setHealth] = useState<Health | null>(null);
  const [status, setStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const update = useUpdater();

  const updateLine =
    update.status === "checking"
      ? "Checking GitHub releases…"
      : update.status === "available"
        ? `Version ${update.version} is available.`
        : update.status === "downloading"
          ? `Downloading update${update.progress ? ` · ${update.progress}%` : ""}…`
          : update.status === "ready"
            ? "Installed. Restart to finish."
            : update.status === "current"
              ? "You're on the newest version."
              : update.status === "error"
                ? update.error ?? "Couldn't check for updates."
                : "Automatic checks run every six hours.";

  useEffect(() => {
    Promise.all([api.settings(), api.health()]).then(([next, info]) => {
      setSettings(next);
      setSaved(next);
      setHealth(info);
    });
  }, []);

  async function save() {
    if (!settings) return;
    setStatus("saving");
    try {
      const next = await api.saveSettings(settings);
      setSettings(next);
      setSaved(next);
      setStatus("saved");
      window.setTimeout(() => setStatus("idle"), 1600);
    } catch {
      setStatus("error");
    }
  }

  if (!settings) {
    return (
      <div className="page">
        <div className="center-state">
          <Loader2 className="spin" size={20} />
          <span>Loading settings</span>
        </div>
      </div>
    );
  }

  const dirty = JSON.stringify(settings) !== JSON.stringify(saved);

  return (
    <div className="page settings-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <SlidersHorizontal size={12} /> configuration
          </span>
          <h1>Settings.</h1>
          <p>Client, refresh and safety preferences. Everything is stored locally.</p>
        </div>
      </header>

      <div className="settings-grid">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Connection</span>
              <h2>Riot client</h2>
            </div>
            <span className={health?.clientAvailable ? "status-pill online" : "status-pill"}>
              <span className="dot" />
              {health?.clientAvailable ? "online" : "offline"}
            </span>
          </div>

          <label className="field">
            <span>Region</span>
            <select
              value={settings.region}
              onChange={(event) =>
                setSettings({ ...settings, region: event.target.value })
              }
            >
              {REGIONS.map((region) => (
                <option value={region.id} key={region.id}>
                  {region.label}
                </option>
              ))}
            </select>
            <small>Auto-detect reads the region from VALORANT's own logs.</small>
          </label>

          <label className="field">
            <span>Live refresh</span>
            <div className="range-row">
              <input
                type="range"
                min={2}
                max={15}
                step={1}
                value={settings.liveRefreshSecs}
                onChange={(event) =>
                  setSettings({
                    ...settings,
                    liveRefreshSecs: Number(event.target.value),
                  })
                }
              />
              <strong>{settings.liveRefreshSecs}s</strong>
            </div>
            <small>Lower is fresher. Riot's local endpoints are rate limited.</small>
          </label>

          {health && (
            <div className="info-row">
              <Database size={15} />
              <div>
                <span>Lockfile</span>
                <code>{health.lockfilePath}</code>
              </div>
            </div>
          )}
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Safety</span>
              <h2>Client actions</h2>
            </div>
            <ShieldCheck size={20} className="panel-icon" />
          </div>

          <label className="toggle-row">
            <div>
              <strong>Allow live actions</strong>
              <span>
                Instalock, dodge and queue actions touch the Riot client. Off by default.
              </span>
            </div>
            <input
              type="checkbox"
              checked={settings.allowLiveActions}
              onChange={(event) =>
                setSettings({ ...settings, allowLiveActions: event.target.checked })
              }
            />
            <i />
          </label>

          <label className="toggle-row">
            <div>
              <strong>Discord Rich Presence</strong>
              <span>
                Show your map, agent and score using Valotrack's built-in Discord app.
              </span>
            </div>
            <input
              type="checkbox"
              checked={settings.discordRpc}
              onChange={(event) =>
                setSettings({ ...settings, discordRpc: event.target.checked })
              }
            />
            <i />
          </label>

          <div className="warn-note">
            <Info size={15} />
            <span>
              Client automation may violate Riot's Terms of Service. Every action is
              opt-in and previewable before it runs.
            </span>
          </div>
        </section>

        <section className="panel update-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Updates</span>
              <h2>Stay current</h2>
            </div>
            <RefreshCw size={20} className="panel-icon" />
          </div>
          <p className="panel-copy">
            Updates are signed and delivered from GitHub releases. Valotrack checks in the
            background and never installs anything without your click.
          </p>
          <div className="update-state">
            <div>
              <strong>Version {health?.appVersion ?? "0.1.3"}</strong>
              <span className={update.status === "error" ? "update-note error" : "update-note"}>
                {updateLine}
              </span>
            </div>
            {update.status === "available" ? (
              <button className="button primary" onClick={() => void update.install()}>
                <Download size={15} /> Install update
              </button>
            ) : update.status === "ready" ? (
              <button className="button primary" onClick={() => void update.restart()}>
                <RefreshCw size={15} /> Restart now
              </button>
            ) : (
              <button
                className="button"
                onClick={() => void update.checkNow()}
                disabled={update.status === "checking" || update.status === "downloading"}
              >
                {update.status === "checking" ? (
                  <Loader2 size={15} className="spin" />
                ) : (
                  <RefreshCw size={15} />
                )}
                Check now
              </button>
            )}
          </div>
          {update.status === "available" && update.notes && (
            <p className="update-notes">{update.notes}</p>
          )}
        </section>

        <section className="panel about-panel">
          <span className="panel-label">About</span>
          <h2>Valotrack</h2>
          <p>
            A native, local-first VALORANT companion. Reads the client directly and
            keeps every byte on your machine.
          </p>
          <dl>
            <div>
              <dt>Version</dt>
              <dd>{health?.appVersion ?? "0.1.3"}</dd>
            </div>
            <div>
              <dt>Runtime</dt>
              <dd>Tauri 2 · Rust</dd>
            </div>
            <div>
              <dt>Storage</dt>
              <dd>%APPDATA%\\com.valotrack.desktop</dd>
            </div>
          </dl>
        </section>
      </div>

      <div className="save-bar">
        <span className={status === "error" ? "save-status error" : "save-status"}>
          {status === "saving" && <Loader2 size={13} className="spin" />}
          {status === "saved" && <Check size={13} />}
          {status === "error"
            ? "Could not save settings"
            : status === "saved"
              ? "Saved"
              : dirty
                ? "Unsaved changes"
                : "Up to date"}
        </span>
        <button
          className="button"
          disabled={!dirty}
          onClick={() => {
            setSettings(saved!);
            setStatus("idle");
          }}
        >
          Reset
        </button>
        <button className="button primary" onClick={save} disabled={!dirty || status === "saving"}>
          <Save size={15} /> Save changes
        </button>
      </div>
    </div>
  );
}
