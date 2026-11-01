# Valotrack

A fast, local VALORANT companion for Windows. Live lobby intel, match history, rank trends, collection value, social chat, an API coverage lab, lineups, crosshairs, sensitivity tools and an ASCII studio in one desktop app. No browser, no terminal.

Built with Tauri 2, Rust and React. All game data is read from your own Riot client and stored on your machine.

## What's inside

- **Play**: live lobby, friends and Riot chat, performance, encounters, collection, matchmaking and appear-offline tools.
- **Armory**: daily store, loadouts, contracts and owned-item validation.
- **Intel**: leaderboard and account overview.
- **API Lab**: coverage for PVP, party, store, pre-game, current-game, contracts, local, local-chat and authentication endpoints. Read-only probes show real responses with tokens redacted; mutations stay on their guarded pages with dry-run.
- Instalock only offers agents owned by the signed-in account, including per-map overrides.

Riot credentials and cookies are never stored. The app uses the already signed-in local Riot client.

## Run

```bash
npm install
npm run tauri dev
```

Build an installer with `npm run tauri build`. Release builds are signed for the built-in updater; set `TAURI_SIGNING_PRIVATE_KEY` to the updater private-key path before bundling.

## Releases

Run the **Release** workflow manually from GitHub Actions. It auto-increments the build number (`v123`, `v124`, ...), builds a signed NSIS installer and publishes `latest.json` for the updater.

Requires Windows 10/11, WebView2, Rust and Node.js. VALORANT must be running for live features.

Community crosshair, lineup, callout, sensitivity and ASCII data plus the .vrf parser come from the open-source Valorant Scout project. Licensed GPL-3.0.

