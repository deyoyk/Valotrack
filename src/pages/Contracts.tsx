import { useCallback, useEffect, useState } from "react";
import { confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  Check,
  FileText,
  Loader2,
  RefreshCw,
  ShieldAlert,
  Sparkles,
  X,
} from "lucide-react";
import { api, ContractsSnapshot, Settings } from "../lib/api";
import { cn } from "../lib/format";

function formatDate(value: string | null) {
  if (!value) return "Not available";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

export default function Contracts() {
  const [data, setData] = useState<ContractsSnapshot | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [dryRun, setDryRun] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [snapshot, config] = await Promise.all([
        api.contractsSnapshot(),
        api.settings(),
      ]);
      setData(snapshot);
      setSettings(config);
      setDryRun(!config.allowLiveActions);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const liveActions = settings?.allowLiveActions ?? false;

  async function enableLiveActions() {
    if (!settings) return;
    const confirmed = await confirmDialog(
      "Enable live client actions? This lets Valotrack activate contracts on your account. It may violate Riot's Terms of Service.",
      { title: "Enable live client actions", kind: "warning" },
    );
    if (!confirmed) {
      return;
    }
    setBusy("enable");
    try {
      const next = await api.saveSettings({ ...settings, allowLiveActions: true });
      setSettings(next);
      setDryRun(false);
      setMessage({ ok: true, text: "Live client actions enabled." });
    } catch (cause) {
      setMessage({
        ok: false,
        text: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setBusy(null);
    }
  }

  async function activate(id: string, name: string) {
    setBusy(id);
    try {
      const result = await api.activateContract(id, dryRun);
      setMessage({ ok: result.ok, text: result.message });
      if (result.ok && result.status === "active") await load();
      else if (result.ok) setMessage({ ok: true, text: `${result.message} (${name})` });
    } catch (cause) {
      setMessage({
        ok: false,
        text: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setBusy(null);
    }
  }

  if (!data && !error) {
    return (
      <div className="page">
        <div className="center-state">
          <Loader2 className="spin" size={20} />
          <span>Reading contract progress</span>
        </div>
      </div>
    );
  }

  const active = data?.contracts.find((contract) => contract.active);

  return (
    <div className="page contracts-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <FileText size={12} /> progression
          </span>
          <h1>Contracts, decoded.</h1>
          <p>
            Active recruitment, agent contracts, weekly missions and the exact XP left on
            each level.
          </p>
        </div>
        <div className="head-status">
          {data?.account && (
            <span className="account-badge">
              <span className="dot" />
              {data.account}
            </span>
          )}
          <button className="button small" onClick={() => load()} disabled={busy !== null}>
            <RefreshCw size={13} /> Refresh
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <AlertTriangle size={16} />
          <div>
            <strong>Contracts unavailable</strong>
            <span>{error}</span>
          </div>
          <button className="button small" onClick={() => load()}>
            Retry
          </button>
        </div>
      )}

      {message && (
        <div className={cn("action-result", message.ok ? "ok" : "error")}>
          {message.ok ? <Check size={16} /> : <AlertTriangle size={16} />}
          <span>{message.text}</span>
          <button className="icon-button" onClick={() => setMessage(null)}>
            <X size={14} />
          </button>
        </div>
      )}

      {!liveActions && (
        <div className="live-actions-note">
          <ShieldAlert size={17} />
          <div>
            <strong>Preview mode</strong>
            <span>Contract activation only simulates until live controls are on.</span>
          </div>
          <button
            className="button primary small"
            onClick={enableLiveActions}
            disabled={busy !== null}
          >
            {busy === "enable" ? <Loader2 size={13} className="spin" /> : null} Enable live
          </button>
        </div>
      )}

      {data && (
        <>
          <section className="contract-hero">
            <div className="contract-active">
              <span className="panel-label">Active recruitment</span>
              <h2>{active?.name ?? "No active special contract"}</h2>
              <p>
                {active
                  ? `Level ${active.level} · ${active.towardsNext.toLocaleString()} XP toward the next level`
                  : "Choose an agent or event contract below."}
              </p>
            </div>
            <div className="contract-weekly">
              <span className="panel-label">Weekly missions refill</span>
              <strong>{formatDate(data.weeklyRefill)}</strong>
              <em>{data.upgradeCount.toLocaleString()} upgrade definitions available</em>
            </div>
          </section>

          <section className="store-section">
            <div className="section-heading">
              <div>
                <span className="panel-label">Contracts</span>
                <h2>{data.contracts.length} tracked</h2>
              </div>
            </div>
            <div className="contract-grid">
              {data.contracts.map((contract) => (
                <article
                  className={cn("contract-card", contract.active && "active")}
                  key={contract.id}
                >
                  <div className="contract-icon">
                    {contract.icon ? (
                      <img src={contract.icon} alt="" loading="lazy" />
                    ) : (
                      <FileText size={22} />
                    )}
                  </div>
                  <div className="contract-copy">
                    <span className="store-tier">{contract.relationType || "Contract"}</span>
                    <strong>{contract.name}</strong>
                    <span>
                      Level {contract.level} · {contract.towardsNext.toLocaleString()} XP
                    </span>
                  </div>
                  {contract.active ? (
                    <span className="active-badge">
                      <Check size={12} /> Active
                    </span>
                  ) : (
                    <button
                      className="button small"
                      onClick={() => activate(contract.id, contract.name)}
                      disabled={busy !== null}
                    >
                      {busy === contract.id ? (
                        <Loader2 size={12} className="spin" />
                      ) : (
                        <Sparkles size={12} />
                      )}
                      Activate
                    </button>
                  )}
                </article>
              ))}
            </div>
          </section>

          <div className="contract-lower">
            <section className="panel">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Weekly missions</span>
                  <h2>This week</h2>
                </div>
                <Sparkles size={18} className="panel-icon" />
              </div>
              <div className="mission-list">
                {data.missions.length > 0 ? (
                  data.missions.map((mission) => {
                    const percent = Math.min(
                      100,
                      Math.max(0, (mission.progress / Math.max(1, mission.target)) * 100),
                    );
                    return (
                      <div className={cn("mission-row", mission.complete && "complete")} key={mission.id}>
                        <div className="mission-copy">
                          <strong>{mission.title}</strong>
                          <span>
                            {mission.progress.toLocaleString()} / {mission.target.toLocaleString()}
                            {mission.complete ? " · complete" : ""}
                          </span>
                        </div>
                        <div className="mission-bar">
                          <i style={{ width: `${percent}%` }} />
                        </div>
                        <em>{mission.xp.toLocaleString()} XP</em>
                      </div>
                    );
                  })
                ) : (
                  <div className="owned-empty">
                    <FileText size={17} />
                    <span>No weekly missions are active.</span>
                  </div>
                )}
              </div>
            </section>

            <section className="panel">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Recent XP</span>
                  <h2>Last sessions</h2>
                </div>
                <Sparkles size={18} className="panel-icon" />
              </div>
              {data.recent.length > 0 ? (
                <div className="xp-list">
                  {data.recent.map((row) => {
                    const total = row.gamePlayed + row.gameWon + row.roundPlayed + row.roundWon;
                    return (
                      <div className="xp-row" key={row.matchId}>
                        <span>{row.matchId.slice(0, 8)}</span>
                        <strong>{total.toLocaleString()} XP</strong>
                        <em>
                          {row.gameWon > 0 ? "win" : "game"} + {row.roundWon.toLocaleString()} rounds
                        </em>
                      </div>
                    );
                  })}
                </div>
              ) : (
                <div className="owned-empty">
                  <FileText size={17} />
                  <span>No processed match XP on this contract response.</span>
                </div>
              )}
              {data.upgradeable.length > 0 && (
                <div className="upgrade-list">
                  {data.upgradeable.slice(0, 6).map((upgrade) => (
                    <div className="upgrade-row" key={upgrade.id}>
                      {upgrade.item.icon ? <img src={upgrade.item.icon} alt="" /> : <Sparkles size={14} />}
                      <strong>{upgrade.item.name}</strong>
                      <span>{upgrade.levels} levels</span>
                    </div>
                  ))}
                </div>
              )}
            </section>
          </div>
        </>
      )}
    </div>
  );
}
