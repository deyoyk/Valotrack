import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  Check,
  Crosshair,
  Loader2,
  RefreshCw,
  ShieldAlert,
  Sparkles,
  X,
} from "lucide-react";
import { api, LoadoutSnapshot, OwnedSkin, Settings } from "../lib/api";
import { cn } from "../lib/format";

export default function Loadout() {
  const [data, setData] = useState<LoadoutSnapshot | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [selected, setSelected] = useState<string>("");
  const [dryRun, setDryRun] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [snapshot, config] = await Promise.all([
        api.loadoutSnapshot(),
        api.settings(),
      ]);
      setData(snapshot);
      setSettings(config);
      setDryRun(!config.allowLiveActions);
      setSelected((current) => current || snapshot.guns[0]?.id || "");
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const gun = useMemo(
    () => data?.guns.find((item) => item.id === selected) ?? data?.guns[0] ?? null,
    [data, selected],
  );
  const ownedForGun = useMemo(
    () => (data?.ownedSkins ?? []).filter((skin) => skin.weaponId === gun?.id),
    [data, gun],
  );
  const liveActions = settings?.allowLiveActions ?? false;

  async function enableLiveActions() {
    if (!settings) return;
    if (
      !window.confirm(
        "Enable live client actions? This lets Valotrack save loadout changes to Riot. It may violate Riot's Terms of Service.",
      )
    ) {
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

  async function equip(skin: OwnedSkin) {
    if (!gun) return;
    setBusy(skin.skinId);
    try {
      const result = await api.setLoadoutSkin(gun.id, skin.skinId, dryRun);
      setMessage({ ok: result.ok, text: result.message });
      if (result.ok && result.status === "equipped") await load();
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
          <span>Reading your loadout</span>
        </div>
      </div>
    );
  }

  return (
    <div className="page loadout-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Crosshair size={12} /> armory
          </span>
          <h1>Show your kit.</h1>
          <p>
            Every equipped weapon skin, charm, spray, card and title on the signed-in Riot
            account.
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
            <strong>Loadout unavailable</strong>
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
            <span>Loadout changes only simulate until live controls are on.</span>
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
        <div className="loadout-layout">
          <section className="panel loadout-weapons">
            <div className="panel-heading">
              <div>
                <span className="panel-label">Weapons</span>
                <h2>{data.guns.length} equipped</h2>
              </div>
              <Crosshair size={19} className="panel-icon" />
            </div>
            <div className="weapon-list">
              {data.guns.map((item) => (
                <button
                  className={cn("weapon-row", item.id === gun?.id && "active")}
                  key={item.id}
                  onClick={() => setSelected(item.id)}
                >
                  <span className="weapon-icon">
                    {item.weapon.icon ? (
                      <img src={item.weapon.icon} alt="" loading="lazy" />
                    ) : (
                      <Crosshair size={16} />
                    )}
                  </span>
                  <span className="weapon-copy">
                    <strong>{item.weapon.name}</strong>
                    <em>{item.skin?.name ?? "Standard"}</em>
                  </span>
                  {item.skin?.icon && <img className="weapon-skin" src={item.skin.icon} alt="" />}
                </button>
              ))}
            </div>
          </section>

          <section className="panel loadout-detail">
            <div className="panel-heading">
              <div>
                <span className="panel-label">Equipped</span>
                <h2>{gun?.weapon.name ?? "Weapon"}</h2>
              </div>
              <span className="store-tier">{gun?.skin?.tier ?? "Standard"}</span>
            </div>
            <div className="loadout-hero-art">
              {gun?.skin?.icon ? (
                <img src={gun.skin.icon} alt={gun.skin.name} />
              ) : (
                <Crosshair size={48} />
              )}
            </div>
            <div className="loadout-equip-copy">
              <div>
                <span className="profile-kicker">Equipped skin</span>
                <h2>{gun?.skin?.name ?? "Standard"}</h2>
              </div>
              {gun?.charm && (
                <span className="loadout-charm">
                  {gun.charm.icon && <img src={gun.charm.icon} alt="" />}
                  {gun.charm.name}
                </span>
              )}
            </div>

            <div className="section-heading compact">
              <div>
                <span className="panel-label">Your collection</span>
                <h3>{ownedForGun.length} owned for this weapon</h3>
              </div>
              <span className="queue-note">{dryRun ? "preview" : "live"}</span>
            </div>
            {ownedForGun.length > 0 ? (
              <div className="owned-skin-grid">
                {ownedForGun.map((skin) => (
                  <button
                    className={cn(
                      "owned-skin-card",
                      gun?.skinId === skin.skinId && "equipped",
                    )}
                    key={skin.skinId}
                    onClick={() => equip(skin)}
                    disabled={busy !== null}
                  >
                    <span className="owned-skin-art">
                      {skin.icon ? <img src={skin.icon} alt="" loading="lazy" /> : <Crosshair size={18} />}
                    </span>
                    <strong>{skin.name}</strong>
                    <em>
                      {skin.tier ?? "Skin"}
                      {skin.vp ? ` · ${skin.vp.toLocaleString()} VP` : ""}
                    </em>
                    {busy === skin.skinId && <Loader2 size={14} className="spin" />}
                    {gun?.skinId === skin.skinId && <Check size={14} />}
                  </button>
                ))}
              </div>
            ) : (
              <div className="owned-empty">
                <Sparkles size={17} />
                <span>You don't own any weapon skins for this gun yet.</span>
              </div>
            )}

            <div className="loadout-sprays">
              <span className="panel-label">Equipped sprays</span>
              <div>
                {data.sprays.map((spray) => (
                  <span className="spray-chip" key={`${spray.slot}-${spray.slotNumber}`}>
                    {spray.spray.icon ? <img src={spray.spray.icon} alt="" /> : <Sparkles size={12} />}
                    {spray.spray.name}
                  </span>
                ))}
              </div>
            </div>
          </section>

          <section className="panel loadout-identity">
            <div className="panel-heading">
              <div>
                <span className="panel-label">Identity</span>
                <h2>Player card</h2>
              </div>
              <Sparkles size={19} className="panel-icon" />
            </div>
            <div className="identity-card">
              {data.identity.card.icon ? (
                <img src={data.identity.card.icon} alt="" />
              ) : (
                <Sparkles size={34} />
              )}
              <strong>{data.identity.card.name}</strong>
            </div>
            <dl className="identity-list">
              <div>
                <dt>Title</dt>
                <dd>{data.identity.title.name}</dd>
              </div>
              <div>
                <dt>Account level</dt>
                <dd>{data.identity.accountLevel || "Hidden"}</dd>
              </div>
              <div>
                <dt>Incognito</dt>
                <dd>{data.incognito ? "On" : "Off"}</dd>
              </div>
            </dl>
          </section>
        </div>
      )}
    </div>
  );
}
