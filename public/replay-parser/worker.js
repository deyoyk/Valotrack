// Module Web Worker hosting the .NET WebAssembly replay parser.
// Boots the runtime once, then services {id, cmd:'parse', vrf} messages from replay-client.js.
import { dotnet } from "./_framework/dotnet.js";

let parse = null;

async function boot() {
  const { getAssemblyExports, getConfig } = await dotnet.create();
  const exports = await getAssemblyExports(getConfig().mainAssemblyName);
  parse = exports.ValorantReplay.Wasm.ReplayInterop.Parse;
}

const ready = boot().then(
  () => postMessage({ ev: "ready" }),
  (err) => postMessage({ ev: "bootError", message: String((err && err.message) || err) })
);

self.onmessage = async (event) => {
  const { id, cmd, vrf } = event.data || {};
  if (cmd !== "parse") return;

  await ready;
  if (!parse) return; // boot already reported bootError

  let lastReported = -1;
  const onProgress = (fraction) => {
    const pct = Math.floor(fraction * 100);
    if (pct === lastReported) return;
    lastReported = pct;
    postMessage({ id, ev: "progress", fraction });
  };

  try {
    const json = parse(vrf, onProgress);
    postMessage({ id, ev: "done", json });
  } catch (err) {
    postMessage({ id, ev: "error", message: String((err && err.message) || err) });
  }
};
