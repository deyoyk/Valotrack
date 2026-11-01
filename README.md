# Valotrack

A fast, local VALORANT companion for Windows. Live lobby intel, match history, rank trends, collection value, lineups, crosshairs, sensitivity tools and an ASCII studio in one desktop app. No browser, no terminal.

Built with Tauri 2, Rust and React. All game data is read from your own Riot client and stored on your machine.

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

