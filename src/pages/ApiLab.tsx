import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  Check,
  Clipboard,
  Clock3,
  Code2,
  ExternalLink,
  Globe2,
  KeyRound,
  Loader2,
  Play,
  RefreshCw,
  Search,
  ShieldCheck,
  Terminal,
  Wifi,
  X,
} from "lucide-react";
import {
  api,
  ApiCatalog,
  ApiEndpoint,
  ApiEndpointStatus,
  ApiProbeResult,
} from "../lib/api";
import { cn } from "../lib/format";
import { usePoll } from "../lib/usePoll";

const statusCopy: Record<ApiEndpointStatus, { label: string; className: string }> = {
  read: { label: "live probe", className: "read" },
  write: { label: "guarded action", className: "write" },
  reauth: { label: "needs reauth", className: "reauth" },
  realtime: { label: "live socket", className: "realtime" },
  legacy: { label: "legacy route", className: "legacy" },
};

const targetCopy: Record<string, string> = {
  local: "Local client",
  pd: "PD shard",
  glz: "GLZ match service",
  shared: "Shared content",
  auth: "Riot auth",
  clientconfig: "Riot config",
  realtime: "Realtime",
  reauth: "Credentials required",
};

function methodClass(method: string) {
  if (method === "GET") return "get";
  if (method === "POST") return "post";
  if (method === "PUT") return "put";
  if (method === "DELETE") return "delete";
  return "other";
}

function endpointSearchText(endpoint: ApiEndpoint) {
  return `${endpoint.label} ${endpoint.category} ${endpoint.path} ${endpoint.note}`.toLowerCase();
}

function Stat({
  label,
  value,
  detail,
  icon: Icon,
}: {
  label: string;
  value: number | string;
  detail: string;
  icon: typeof Terminal;
}) {
  return (
    <div className="api-stat">
      <div className="api-stat-icon">
        <Icon size={17} />
      </div>
      <div>
        <span>{label}</span>
        <strong>{value}</strong>
        <em>{detail}</em>
      </div>
    </div>
  );
}

export default function ApiLab() {
  const [catalog, setCatalog] = useState<ApiCatalog | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("All");
  const [only, setOnly] = useState<"all" | "read" | "feature" | "reauth">("all");
  const [active, setActive] = useState<ApiEndpoint | null>(null);
  const [result, setResult] = useState<ApiProbeResult | null>(null);
  const [running, setRunning] = useState<string | null>(null);
  const [args, setArgs] = useState<Record<string, string>>({});
  const [copied, setCopied] = useState(false);

  const load = useCallback(async () => {
    try {
      const data = await api.apiCatalog();
      setCatalog(data);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  usePoll(load, 20000);

  useEffect(() => {
    void load();
  }, [load]);

  const endpoints = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return (catalog?.endpoints ?? []).filter((endpoint) => {
      if (category !== "All" && endpoint.category !== category) return false;
      if (only === "read" && endpoint.status !== "read") return false;
      if (only === "feature" && !endpoint.feature) return false;
      if (only === "reauth" && endpoint.status !== "reauth") return false;
      if (needle && !endpointSearchText(endpoint).includes(needle)) return false;
      return true;
    });
  }, [catalog, category, only, search]);

  const grouped = useMemo(() => {
    const order = catalog?.categories ?? [];
    return order
      .map((name) => ({
        name,
        rows: endpoints.filter((endpoint) => endpoint.category === name),
      }))
      .filter((group) => group.rows.length > 0);
  }, [catalog, endpoints]);

  const all = catalog?.endpoints ?? [];
  const readCount = all.filter((endpoint) => endpoint.status === "read").length;
  const featureCount = all.filter((endpoint) => endpoint.feature).length;
  const reauthCount = all.filter((endpoint) => endpoint.status === "reauth").length;
  const context = catalog?.context;

  async function run(endpoint: ApiEndpoint) {
    setActive(endpoint);
    setRunning(endpoint.id);
    setResult(null);
    setCopied(false);
    try {
      const response = await api.apiProbe(endpoint.id, args[endpoint.id]);
      setResult(response);
    } catch (cause) {
      setResult({
        ok: false,
        status: "error",
        method: endpoint.method,
        path: endpoint.path,
        target: endpoint.target,
        note: endpoint.note,
        error: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setRunning(null);
    }
  }

  async function copyResult() {
    if (!result) return;
    await navigator.clipboard.writeText(JSON.stringify(result.data ?? result, null, 2));
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  }

  function openFeature(page: string) {
    location.hash = page;
  }

  return (
    <div className="page api-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Terminal size={12} /> developer lab
          </span>
          <h1>API command center.</h1>
          <p>
            Live VALAPI coverage across PVP, party, store, pre-game, current game,
            contracts, local chat and Riot auth. Read-only probes run against your
            signed-in local client; tokens are redacted automatically.
          </p>
        </div>
        <div className="head-status">
          {context?.account && (
            <span className="account-badge">
              <span className="dot" />
              {context.account}
            </span>
          )}
          <span className="status-pill online">
            <Wifi size={10} />
            {context?.available ? `${context.shard ?? "live"} · ${context.region ?? "?"}` : "client offline"}
          </span>
          <button className="button small" onClick={() => void load()} disabled={running !== null}>
            <RefreshCw size={13} /> Refresh
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <AlertTriangle size={16} />
          <div>
            <strong>Local client unavailable</strong>
            <span>{error}</span>
          </div>
          <button className="button small" onClick={() => void load()}>
            Retry
          </button>
        </div>
      )}

      <div className="api-stats">
        <Stat
          label="Endpoints mapped"
          value={all.length}
          detail="across 10 VALAPI groups"
          icon={Code2}
        />
        <Stat
          label="Safe live probes"
          value={readCount}
          detail="real local and shard reads"
          icon={Play}
        />
        <Stat
          label="Product surfaces"
          value={featureCount}
          detail="wired into Valotrack pages"
          icon={ShieldCheck}
        />
        <Stat
          label="Credential-gated"
          value={reauthCount}
          detail="never stored by Valotrack"
          icon={KeyRound}
        />
      </div>

      <div className="api-layout">
        <section className="panel api-browser">
          <div className="api-toolbar">
            <div className="search-box api-search">
              <Search size={14} />
              <input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Search endpoints, paths or features"
              />
              {search && (
                <button className="icon-button tiny" onClick={() => setSearch("")}>
                  <X size={12} />
                </button>
              )}
            </div>
            <div className="api-filters">
              {(
                [
                  ["all", "All"],
                  ["read", "Live probes"],
                  ["feature", "In product"],
                  ["reauth", "Auth only"],
                ] as const
              ).map(([value, label]) => (
                <button
                  key={value}
                  className={cn("api-filter", only === value && "active")}
                  onClick={() => setOnly(value)}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>

          <div className="api-categories">
            <button
              className={cn("api-category", category === "All" && "active")}
              onClick={() => setCategory("All")}
            >
              All <b>{all.length}</b>
            </button>
            {(catalog?.categories ?? []).map((name) => (
              <button
                key={name}
                className={cn("api-category", category === name && "active")}
                onClick={() => setCategory(name)}
              >
                {name}{" "}
                <b>{all.filter((endpoint) => endpoint.category === name).length}</b>
              </button>
            ))}
          </div>

          <div className="api-groups">
            {grouped.map((group) => (
              <section className="api-group" key={group.name}>
                <div className="api-group-head">
                  <span>{group.name}</span>
                  <b>{group.rows.length}</b>
                </div>
                {group.rows.map((endpoint) => {
                  const status = statusCopy[endpoint.status];
                  const busy = running === endpoint.id;
                  return (
                    <article
                      className={cn("api-row", active?.id === endpoint.id && "active")}
                      key={endpoint.id}
                    >
                      <div className="api-row-top">
                        <span className={cn("api-method", methodClass(endpoint.method))}>
                          {endpoint.method}
                        </span>
                        <div className="api-row-title">
                          <strong>{endpoint.label}</strong>
                          <code>{endpoint.path}</code>
                        </div>
                        <span className={cn("api-status", status.className)}>{status.label}</span>
                      </div>
                      <p>{endpoint.note}</p>
                      <div className="api-row-foot">
                        <span className="api-target">
                          <Globe2 size={10} />
                          {targetCopy[endpoint.target] ?? endpoint.target}
                        </span>
                        {endpoint.arg && (
                          <input
                            className="api-arg"
                            value={args[endpoint.id] ?? ""}
                            onChange={(event) =>
                              setArgs((current) => ({
                                ...current,
                                [endpoint.id]: event.target.value,
                              }))
                            }
                            placeholder={
                              endpoint.arg === "cid"
                                ? "Conversation ID (optional)"
                                : "Match ID (optional)"
                            }
                          />
                        )}
                        <div className="api-actions">
                          {endpoint.feature && (
                            <button
                              className="button small"
                              onClick={() => openFeature(endpoint.feature!)}
                            >
                              <ExternalLink size={12} /> Open
                            </button>
                          )}
                          {endpoint.status === "read" && (
                            <button
                              className="button small primary"
                              onClick={() => void run(endpoint)}
                              disabled={running !== null}
                            >
                              {busy ? (
                                <Loader2 size={12} className="spin" />
                              ) : (
                                <Play size={12} />
                              )}
                              {busy ? "Running" : "Run"}
                            </button>
                          )}
                        </div>
                      </div>
                    </article>
                  );
                })}
              </section>
            ))}
            {grouped.length === 0 && (
              <div className="api-empty">
                <Search size={18} />
                <strong>No matching endpoints</strong>
                <span>Try a different search or clear the filters.</span>
              </div>
            )}
          </div>
        </section>

        <aside className="panel api-response">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Response</span>
              <h2>{active?.label ?? "Pick an endpoint"}</h2>
            </div>
            <Terminal size={19} className="panel-icon" />
          </div>

          {!active && (
            <div className="api-empty response-empty">
              <Terminal size={28} />
              <strong>Nothing probed yet</strong>
              <span>
                Run a live probe to see the real Riot response, latency and shape.
                Tokens and secrets are redacted before they reach the UI.
              </span>
            </div>
          )}

          {active && (
            <>
              <div className="api-response-meta">
                <span className={cn("api-method", methodClass(active.method))}>
                  {active.method}
                </span>
                <code>{result?.path ?? active.path}</code>
                <button className="icon-button" onClick={() => void copyResult()} disabled={!result}>
                  {copied ? <Check size={13} /> : <Clipboard size={13} />}
                </button>
              </div>

              {running === active.id && (
                <div className="api-running">
                  <Loader2 size={16} className="spin" />
                  <span>Waiting on Riot services…</span>
                </div>
              )}

              {result && running !== active.id && (
                <>
                  <div className={cn("api-result-head", result.ok ? "ok" : "error")}>
                    <span className="api-result-dot" />
                    <strong>{result.ok ? "200 OK" : result.status}</strong>
                    {result.ms !== undefined && (
                      <em>
                        <Clock3 size={10} /> {result.ms} ms
                      </em>
                    )}
                    <span className="api-result-target">
                      {targetCopy[result.target] ?? result.target}
                    </span>
                  </div>
                  {result.error && (
                    <div className="api-result-error">
                      <AlertTriangle size={13} />
                      <span>{result.error}</span>
                    </div>
                  )}
                  {result.data !== undefined && (
                    <pre className="api-json">{JSON.stringify(result.data, null, 2)}</pre>
                  )}
                  {result.data === undefined && !result.error && (
                    <div className="api-empty response-empty">
                      <Check size={20} />
                      <strong>Empty response</strong>
                      <span>Riot returned a successful request with no body.</span>
                    </div>
                  )}
                  <p className="api-response-note">{result.note}</p>
                </>
              )}
            </>
          )}

          <div className="api-response-foot">
            <ShieldCheck size={12} />
            <span>Read-only. Mutations stay on their guarded product pages.</span>
          </div>
        </aside>
      </div>
    </div>
  );
}
