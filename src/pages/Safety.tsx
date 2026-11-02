import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ArrowUpRight,
  Check,
  Database,
  EyeOff,
  HardDrive,
  LockKeyhole,
  ShieldAlert,
  ShieldCheck,
  Wifi,
} from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { displayVersion } from "../lib/version";

export default function Safety() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    void getVersion().then(setVersion).catch(() => void 0);
  }, []);

  return (
    <div className="page safety-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <ShieldCheck size={12} /> trust center
          </span>
          <h1>Nothing leaves this PC.</h1>
          <p>
            Valotrack reads your own Riot client, stores everything on this machine and
            keeps automation opt-in. Here is exactly what that means.
          </p>
        </div>
        <div className="head-status">
          <span className="status-pill online">
            <span className="dot" />
            local-first
          </span>
          <span className="checked">version {displayVersion(version) || "loading"}</span>
        </div>
      </header>

      <section className="safety-hero">
        <div className="panel">
          <span className="panel-label">Privacy model</span>
          <h2>Read local. Keep local.</h2>
          <p>
            Match data comes from the same local endpoints VALORANT already exposes on
            your computer. Scores, ranks, parties and profiles are rendered straight to
            this window and are never uploaded to a Valotrack server.
          </p>
          <div className="head-buttons" style={{ marginTop: 22 }}>
            <button
              className="button primary"
              onClick={() => void openUrl("https://github.com/deyoyk/Valotrack")}
            >
              Read the source <ArrowUpRight size={14} />
            </button>
          </div>
        </div>
        <div className="panel safety-score">
          <strong>100%</strong>
          <span>stored on this machine</span>
          <p className="quiet">No account. No cloud sync. No advertising profile.</p>
        </div>
      </section>

      <div className="safety-grid">
        <section className="panel safety-card">
          <span className="panel-label">Data path</span>
          <h3>What Valotrack reads</h3>
          <ul>
            <li>
              <Check size={15} /> Local Riot lockfile and client endpoints for the live
              lobby.
            </li>
            <li>
              <Check size={15} /> Your own match history, rank, wallet, store, loadout and
              contracts.
            </li>
            <li>
              <Check size={15} /> Party, friends and presence data the client already
              receives.
            </li>
            <li>
              <Check size={15} /> Replay files you choose to drop into the replay viewer.
            </li>
          </ul>
        </section>

        <section className="panel safety-card">
          <span className="panel-label">Storage</span>
          <h3>Where it lives</h3>
          <ul>
            <li>
              <HardDrive size={15} /> Settings, encounter history and RR history live in
              your Windows app-data folder.
            </li>
            <li>
              <Database size={15} /> Match notes, sessions and strategy boards stay in
              local storage on this device.
            </li>
            <li>
              <LockKeyhole size={15} /> The phone remote opens a token-protected page only
              on your own network.
            </li>
            <li>
              <EyeOff size={15} /> There is no telemetry pipeline and no analytics SDK in
              the desktop app.
            </li>
          </ul>
        </section>

        <section className="panel safety-card risk">
          <span className="panel-label">Automation</span>
          <h3>The honest risk</h3>
          <p>
            Instalock, dodge and queue actions automate part of agent select. Client
            automation can violate Riot's Terms of Service.
          </p>
          <ul>
            <li>
              <ShieldAlert size={15} /> Live actions are off by default and always preview
              first.
            </li>
            <li>
              <ShieldAlert size={15} /> Read-only pages never need live actions enabled.
            </li>
            <li>
              <ShieldAlert size={15} /> Use automation only if you accept the account risk.
            </li>
          </ul>
        </section>

        <section className="panel safety-card">
          <span className="panel-label">Network</span>
          <h3>What goes online</h3>
          <ul>
            <li>
              <Wifi size={15} /> Riot client calls stay on localhost.
            </li>
            <li>
              <Wifi size={15} /> Public art and metadata come from Riot's community asset
              APIs.
            </li>
            <li>
              <Wifi size={15} /> Update checks read signed GitHub releases.
            </li>
            <li>
              <Wifi size={15} /> Nothing about your lobby is sent to those services.
            </li>
          </ul>
        </section>
      </div>
    </div>
  );
}
