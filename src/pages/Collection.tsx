import { useEffect, useState } from "react";
import {
  AlertTriangle,
  Diamond,
  Loader2,
  RefreshCw,
  Sparkles,
  Wallet,
} from "lucide-react";
import { api, Inventory } from "../lib/api";

const CURRENCIES = [
  { key: "vp", label: "VALORANT Points", color: "#ff4655" },
  { key: "rad", label: "Radianite", color: "#38d6b0" },
  { key: "kc", label: "Kingdom Credits", color: "#e8b45a" },
] as const;

export default function Collection() {
  const [data, setData] = useState<Inventory | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  async function load(quiet = false, refresh = false) {
    if (!quiet) setRefreshing(true);
    try {
      setData(await api.inventory(refresh));
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }

  useEffect(() => {
    void load(true);
  }, []);

  if (loading) {
    return (
      <div className="page">
        <div className="center-state">
          <Loader2 className="spin" size={20} />
          <span>Valuing your collection</span>
        </div>
      </div>
    );
  }

  return (
    <div className="page collection-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Wallet size={12} /> collection
          </span>
          <h1>What's it worth?</h1>
          <p>
            Every owned weapon skin valued from Riot's content tiers, plus wallets and
            item counts read straight from your account entitlements.
          </p>
        </div>
        <div className="head-status">
          {data?.account && (
            <span className="account-badge">
              <span className="dot" />
              {data.account}
            </span>
          )}
          <button className="button small" onClick={() => load(false, true)} disabled={refreshing}>
            <RefreshCw size={13} className={refreshing ? "spin" : ""} /> Refresh
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <AlertTriangle size={16} />
          <div>
            <strong>Collection unavailable</strong>
            <span>{error}</span>
          </div>
          <button className="button small" onClick={() => load(false, true)}>
            Retry
          </button>
        </div>
      )}

      {data ? (
        <>
          <section className="collection-hero">
            <div className="collection-total">
              <span>Collection value</span>
              <strong>{(data.totalVp ?? 0).toLocaleString()} VP</strong>
              <em>≈ ${(data.usdApprox ?? 0).toLocaleString()} USD</em>
            </div>
            <div className="wallet-grid">
              {CURRENCIES.map((currency) => (
                <div className="wallet-card" key={currency.key}>
                  <i style={{ background: currency.color }} />
                  <span>{currency.label}</span>
                  <strong>{(data.wallet[currency.key] ?? 0).toLocaleString()}</strong>
                </div>
              ))}
            </div>
          </section>

          <div className="collection-grid">
            <section className="panel">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Inventory</span>
                  <h2>Owned items</h2>
                </div>
                <Diamond size={19} className="panel-icon" />
              </div>
              <div className="count-grid">
                {Object.entries(data.counts).map(([key, value]) => (
                  <div key={key}>
                    <strong>{(value ?? 0).toLocaleString()}</strong>
                    <span>{key === "earned" ? "free skins" : key}</span>
                  </div>
                ))}
              </div>
              {(data.owned?.length ?? 0) > 0 ? (
                <div className="owned-grid">
                  {data.owned.map((item, index) => (
                    <div className="owned-card" key={`${item.type}-${item.uuid}-${index}`}>
                      <div className="owned-art">
                        {item.icon ? (
                          <img src={item.icon} alt={item.name} loading="lazy" />
                        ) : (
                          <span>{item.name.slice(0, 1).toUpperCase()}</span>
                        )}
                      </div>
                      <strong>{item.name}</strong>
                      <span>
                        {item.type === "skin"
                          ? `${item.tier ?? "Skin"} · ${(item.vp ?? 0).toLocaleString()} VP`
                          : item.type}
                        {item.earned ? " · free" : ""}
                      </span>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="owned-empty">
                  <Diamond size={17} />
                  <span>No items on this Riot account yet.</span>
                </div>
              )}
            </section>

            <section className="panel">
              <div className="panel-heading">
                <div>
                  <span className="panel-label">Breakdown</span>
                  <h2>By tier</h2>
                </div>
                <Sparkles size={19} className="panel-icon" />
              </div>
              <div className="tier-list">
                {Object.entries(data.tiers)
                  .sort((a, b) => b[1].vp - a[1].vp)
                  .map(([tier, row]) => {
                    const share = data.totalVp ? (row.vp / data.totalVp) * 100 : 0;
                    return (
                      <div className="tier-row" key={tier}>
                        <span>{tier}</span>
                        <div className="mini-bar">
                          <i style={{ width: `${share}%` }} />
                        </div>
                        <strong>{row.vp.toLocaleString()} VP</strong>
                        <em>{row.skins} skins</em>
                      </div>
                    );
                  })}
              </div>
            </section>
          </div>

          <section className="panel top-skins">
            <div className="panel-heading">
              <div>
                <span className="panel-label">Highest value</span>
                <h2>Top skins</h2>
              </div>
            </div>
            <div className="skin-grid">
              {data.top.length > 0 ? (
                data.top.map((skin) => (
                  <div className="skin-card" key={`${skin.name}-${skin.vp}`}>
                    <div className="skin-art">
                      {skin.icon ? <img src={skin.icon} alt={skin.name} loading="lazy" /> : null}
                    </div>
                    <strong>{skin.name}</strong>
                    <span>
                      {skin.tier} · {skin.vp.toLocaleString()} VP
                    </span>
                  </div>
                ))
              ) : (
                <div className="owned-empty">
                  <Diamond size={17} />
                  <span>No paid weapon skins on this Riot account yet.</span>
                </div>
              )}
            </div>
          </section>
        </>
      ) : (
        <div className="board-empty">
          <div className="empty-icon">
            <AlertTriangle size={20} />
          </div>
          <h2>Open VALORANT</h2>
          <p>Your collection loads from the signed-in Riot client.</p>
          <button className="button primary" onClick={() => load(false, true)}>
            <RefreshCw size={14} /> Try again
          </button>
        </div>
      )}
    </div>
  );
}
