import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
  Clock3,
  Loader2,
  Package,
  RefreshCw,
  ShoppingBag,
  Sparkles,
  Tag,
} from "lucide-react";
import { api, StoreBundle, StoreItem, StoreSnapshot } from "../lib/api";
import { cn } from "../lib/format";

const WALLET = [
  { key: "vp", label: "VALORANT Points", color: "#ff4655" },
  { key: "rad", label: "Radianite", color: "#38d6b0" },
  { key: "kc", label: "Kingdom Credits", color: "#e8b45a" },
] as const;

function formatDuration(total: number) {
  const seconds = Math.max(0, Math.floor(total));
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${secs}s`;
  return `${secs}s`;
}

function currencyLabel(currency?: string) {
  if (currency === "kc") return "KC";
  if (currency === "rad") return "RAD";
  return "VP";
}

function Countdown({ remaining, fetchedAt }: { remaining: number; fetchedAt: number }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);
  const elapsed = Math.max(0, Math.floor((now - fetchedAt * 1000) / 1000));
  return <>{formatDuration(remaining - elapsed)}</>;
}

function StoreCard({ item }: { item: StoreItem }) {
  const discount = item.discountPercent ?? 0;
  return (
    <article className={cn("store-card", discount > 0 && "discounted")}>
      <div className="store-art">
        {item.icon ? <img src={item.icon} alt={item.name} loading="lazy" /> : <ShoppingBag size={28} />}
        {discount > 0 && <span className="discount-badge">-{discount}%</span>}
      </div>
      <div className="store-meta">
        <span className="store-tier">{item.tier ?? item.type}</span>
        <strong>{item.name}</strong>
        <span>{item.type === "skin" ? item.weapon ?? "Weapon skin" : item.type}</span>
      </div>
      <div className="store-price">
        {item.owned ? (
          <em className="owned-tag">Owned</em>
        ) : (
          <>
            <strong>
              {(item.cost ?? 0).toLocaleString()} {currencyLabel(item.currency)}
            </strong>
            {discount > 0 && item.vp ? <s>{item.vp.toLocaleString()} VP</s> : null}
          </>
        )}
      </div>
    </article>
  );
}

function BundleCard({ bundle }: { bundle: StoreBundle }) {
  return (
    <article className="bundle-card">
      <div className="bundle-art">
        {bundle.icon ? <img src={bundle.icon} alt={bundle.name} loading="lazy" /> : <Package size={32} />}
      </div>
      <div className="bundle-copy">
        <span className="store-tier">Featured bundle</span>
        <h2>{bundle.name}</h2>
        <p>
          {bundle.items.length} items
          {bundle.discountPercent > 0 ? ` · ${bundle.discountPercent}% off` : ""}
        </p>
        <div className="bundle-price">
          <strong>{bundle.discountedCost.toLocaleString()} VP</strong>
          {bundle.baseCost > bundle.discountedCost && (
            <s>{bundle.baseCost.toLocaleString()} VP</s>
          )}
        </div>
        <div className="bundle-items">
          {bundle.items.slice(0, 12).map((item, index) => (
            <span key={`${item.name}-${index}`} title={item.name}>
              {item.icon ? <img src={item.icon} alt="" loading="lazy" /> : <Package size={12} />}
            </span>
          ))}
        </div>
      </div>
    </article>
  );
}

export default function Store() {
  const [data, setData] = useState<StoreSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const load = useCallback(async (refresh = false) => {
    setRefreshing(true);
    try {
      setData(await api.storeSnapshot(refresh));
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void load(false);
  }, [load]);

  if (loading) {
    return (
      <div className="page">
        <div className="center-state">
          <Loader2 className="spin" size={20} />
          <span>Opening the armory</span>
        </div>
      </div>
    );
  }

  return (
    <div className="page store-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <ShoppingBag size={12} /> armory
          </span>
          <h1>The daily drop.</h1>
          <p>
            Live store rotation, featured bundles, night market and accessory offers read
            directly from your Riot account.
          </p>
        </div>
        <div className="head-status">
          {data?.account && (
            <span className="account-badge">
              <span className="dot" />
              {data.account}
            </span>
          )}
          <button className="button small" onClick={() => load(true)} disabled={refreshing}>
            <RefreshCw size={13} className={refreshing ? "spin" : ""} /> Refresh
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <AlertTriangle size={16} />
          <div>
            <strong>Store unavailable</strong>
            <span>{error}</span>
          </div>
          <button className="button small" onClick={() => load(true)}>
            Retry
          </button>
        </div>
      )}

      {data ? (
        <>
          <section className="collection-hero">
            <div className="collection-total store-total">
              <span>Daily store resets in</span>
              <strong>
                <Countdown remaining={data.daily.remaining} fetchedAt={data.fetchedAt} />
              </strong>
              <em>{data.account}</em>
            </div>
            <div className="wallet-grid">
              {WALLET.map((currency) => (
                <div className="wallet-card" key={currency.key}>
                  <i style={{ background: currency.color }} />
                  <span>{currency.label}</span>
                  <strong>{(data.wallet[currency.key] ?? 0).toLocaleString()}</strong>
                </div>
              ))}
            </div>
          </section>

          <section className="store-section">
            <div className="section-heading">
              <div>
                <span className="panel-label">Daily rotation</span>
                <h2>Four for you</h2>
              </div>
              <span className="section-clock">
                <Clock3 size={13} />{" "}
                <Countdown remaining={data.daily.remaining} fetchedAt={data.fetchedAt} />
              </span>
            </div>
            {data.daily.offers.length > 0 ? (
              <div className="store-grid">
                {data.daily.offers.map((item) => (
                  <StoreCard item={item} key={item.offerId ?? `${item.name}-${item.cost}`} />
                ))}
              </div>
            ) : (
              <div className="owned-empty">
                <ShoppingBag size={17} />
                <span>No daily offers are available right now.</span>
              </div>
            )}
          </section>

          {data.featured.bundles.length > 0 && (
            <section className="store-section">
              <div className="section-heading">
                <div>
                  <span className="panel-label">Featured</span>
                  <h2>Bundle drops</h2>
                </div>
                <span className="section-clock">
                  <Clock3 size={13} />{" "}
                  <Countdown remaining={data.featured.remaining} fetchedAt={data.fetchedAt} />
                </span>
              </div>
              <div className="bundle-list">
                {data.featured.bundles.map((bundle) => (
                  <BundleCard bundle={bundle} key={bundle.id} />
                ))}
              </div>
            </section>
          )}

          {data.nightMarket && data.nightMarket.offers.length > 0 && (
            <section className="store-section night-market">
              <div className="section-heading">
                <div>
                  <span className="panel-label">Limited time</span>
                  <h2>Night market</h2>
                </div>
                <span className="section-clock">
                  <Clock3 size={13} />{" "}
                  <Countdown
                    remaining={data.nightMarket.remaining}
                    fetchedAt={data.fetchedAt}
                  />
                </span>
              </div>
              <div className="store-grid">
                {data.nightMarket.offers.map((item) => (
                  <StoreCard item={item} key={item.offerId ?? `${item.name}-${item.cost}`} />
                ))}
              </div>
            </section>
          )}

          <div className="store-lower">
            {data.accessories.length > 0 && (
              <section className="panel">
                <div className="panel-heading">
                  <div>
                    <span className="panel-label">Accessories</span>
                    <h2>Extras</h2>
                  </div>
                  <Sparkles size={18} className="panel-icon" />
                </div>
                <div className="compact-offers">
                  {data.accessories.map((item) => (
                    <div className="compact-offer" key={`${item.name}-${item.offerId}`}>
                      <div>
                        {item.icon ? <img src={item.icon} alt="" loading="lazy" /> : <Tag size={14} />}
                      </div>
                      <strong>{item.name}</strong>
                      <span>
                        {(item.cost ?? 0).toLocaleString()} {currencyLabel(item.currency)}
                      </span>
                    </div>
                  ))}
                </div>
              </section>
            )}

            {data.upgradeCurrency.length > 0 && (
              <section className="panel">
                <div className="panel-heading">
                  <div>
                    <span className="panel-label">Upgrades</span>
                    <h2>Radianite offers</h2>
                  </div>
                  <Package size={18} className="panel-icon" />
                </div>
                <div className="compact-offers">
                  {data.upgradeCurrency.map((item) => (
                    <div className="compact-offer" key={`${item.storefrontItemId}-${item.offerId}`}>
                      <div>
                        {item.icon ? <img src={item.icon} alt="" loading="lazy" /> : <Tag size={14} />}
                      </div>
                      <strong>{item.name}</strong>
                      <span>
                        {(item.cost ?? 0).toLocaleString()} {currencyLabel(item.currency)}
                      </span>
                    </div>
                  ))}
                </div>
              </section>
            )}
          </div>
        </>
      ) : (
        !error && (
          <div className="board-empty">
            <div className="empty-icon">
              <ShoppingBag size={20} />
            </div>
            <h2>Sign in to the Riot client</h2>
            <p>The store appears once your Riot account is fully signed in.</p>
            <button className="button primary" onClick={() => load(true)}>
              <RefreshCw size={14} /> Try again
            </button>
          </div>
        )
      )}
    </div>
  );
}
