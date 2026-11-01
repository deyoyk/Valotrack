import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Check,
  Copy,
  Loader2,
  Lock,
  QrCode,
  ShieldCheck,
  Smartphone,
  Wifi,
} from "lucide-react";
import { api, RemoteInfo } from "../lib/api";
import { copyText } from "../lib/clipboard";

export default function Remote() {
  const [info, setInfo] = useState<RemoteInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .remoteInfo()
      .then((value) => setInfo(value))
      .catch(() => void 0);
  }, []);

  async function start() {
    setBusy(true);
    setError(null);
    try {
      setInfo(await api.remoteStart());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page remote-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <Smartphone size={12} /> phone remote
          </span>
          <h1>Your board, in your pocket.</h1>
          <p>
            Scan the QR code to mirror the live board on your phone over your local
            network. Ranks, parties, smurf flags and queue controls - no cloud.
          </p>
        </div>
        <div className="head-status">
          <div className={info?.running ? "status-pill online" : "status-pill"}>
            <span className="dot" />
            {info?.running ? "remote live" : "remote off"}
          </div>
        </div>
      </header>

      {error && <div className="error-banner"><span>{error}</span></div>}

      <div className="remote-grid">
        <section className="panel remote-qr-panel">
          {info?.running && info.qrSvg ? (
            <>
              <div className="remote-qr" dangerouslySetInnerHTML={{ __html: info.qrSvg }} />
              <span className="panel-label">Scan with your phone camera</span>
              <div className="remote-url">
                <code>{info.url}</code>
                <button
                  className="icon-button"
                  onClick={async () => {
                    if (info.url && (await copyText(info.url))) {
                      setCopied(true);
                      window.setTimeout(() => setCopied(false), 1400);
                    }
                  }}
                  aria-label="Copy remote URL"
                >
                  {copied ? <Check size={14} /> : <Copy size={14} />}
                </button>
              </div>
              <button className="button small" onClick={() => info.url && void openUrl(info.url)}>
                Open on this PC
              </button>
            </>
          ) : (
            <div className="remote-idle">
              <div className="remote-phone">
                <QrCode size={38} />
              </div>
              <h2>Start the local remote</h2>
              <p>
                The app opens a token-protected page on this PC. Your phone must be on
                the same Wi-Fi network.
              </p>
              <button className="button primary" onClick={start} disabled={busy}>
                {busy ? <Loader2 size={14} className="spin" /> : <Smartphone size={14} />}
                Start phone remote
              </button>
            </div>
          )}
        </section>

        <section className="panel remote-info-panel">
          <div className="panel-heading">
            <div>
              <span className="panel-label">What the phone gets</span>
              <h2>Full live view</h2>
            </div>
            <Wifi size={19} className="panel-icon" />
          </div>
          <ul className="remote-features">
            <li>
              <ShieldCheck size={15} />
              <span>Both teams, ranks, parties, K/D and smurf flags.</span>
            </li>
            <li>
              <Wifi size={15} />
              <span>Live queue status and start / cancel from the couch.</span>
            </li>
            <li>
              <Lock size={15} />
              <span>Random access token. Only devices with the QR link can read it.</span>
            </li>
            <li>
              <Smartphone size={15} />
              <span>Agent lock and side check controls, enforced by your live-actions setting.</span>
            </li>
          </ul>
          <div className="remote-note">
            <ShieldCheck size={13} />
            <span>
              Data travels only over your LAN. If your phone can't connect, allow Valotrack
              through Windows Firewall for private networks.
            </span>
          </div>
          <div className="remote-meta">
            <span>Port</span>
            <strong>{info?.port ?? "-"}</strong>
            <span>Token</span>
            <code>{info?.token ? `${info.token.slice(0, 4)}••••${info.token.slice(-4)}` : "-"}</code>
          </div>
        </section>
      </div>
    </div>
  );
}
