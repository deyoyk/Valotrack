import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { confirm as confirmDialog, open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  AlertTriangle,
  AudioLines,
  Check,
  ExternalLink,
  Headphones,
  Loader2,
  Mic,
  Plus,
  RefreshCw,
  Square,
  Trash2,
  Volume2,
  VolumeX,
} from "lucide-react";
import { api, SoundboardSnapshot } from "../lib/api";
import { cn } from "../lib/format";

const AUDIO_EXTENSIONS = ["mp3", "wav", "ogg", "flac", "m4a"];

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function Soundboard() {
  const [data, setData] = useState<SoundboardSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [lastPlayed, setLastPlayed] = useState<string | null>(null);
  const volumeTimer = useRef<number | null>(null);

  const load = useCallback(async () => {
    try {
      const snapshot = await api.soundboard();
      setData(snapshot);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
    return () => {
      if (volumeTimer.current !== null) window.clearTimeout(volumeTimer.current);
    };
  }, [load]);

  const selectedOutput = useMemo(() => {
    if (!data) return null;
    return (
      data.outputs.find((device) => device.name === data.selectedDevice) ??
      data.outputs.find((device) => device.isDefault) ??
      null
    );
  }, [data]);
  const defaultInput = useMemo(
    () => data?.inputs.find((device) => device.isDefault) ?? null,
    [data],
  );
  const hasVirtualOutput = useMemo(
    () => data?.outputs.some((device) => device.isVirtual) ?? false,
    [data],
  );

  async function importSounds() {
    setError(null);
    setNotice(null);
    let picked: string | string[] | null = null;
    try {
      picked = await openDialog({
        multiple: true,
        title: "Add sounds",
        filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
      });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      return;
    }
    const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
    if (!paths.length) return;
    setBusy("import");
    try {
      const snapshot = await api.soundboardImport(paths);
      setData(snapshot);
      setNotice(`${paths.length} sound${paths.length === 1 ? "" : "s"} added.`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function play(id: string) {
    setError(null);
    try {
      await api.soundboardPlay(id);
      setLastPlayed(id);
      window.setTimeout(() => setLastPlayed((current) => (current === id ? null : current)), 1400);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function stop(id: string) {
    try {
      await api.soundboardStop(id);
      setLastPlayed(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function stopAll() {
    try {
      await api.soundboardStopAll();
      setLastPlayed(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function remove(id: string, name: string) {
    const confirmed = await confirmDialog(`Remove "${name}" from the soundboard?`, {
      title: "Remove sound",
      kind: "warning",
    });
    if (!confirmed) return;
    setBusy("remove");
    try {
      setData(await api.soundboardRemove(id));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function testRoute() {
    setNotice(null);
    setBusy("test");
    try {
      await api.soundboardTest();
      setNotice("Test tone sent to the selected output.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function changeDevice(device: string) {
    setBusy("device");
    setError(null);
    try {
      setData(await api.soundboardSetDevice(device));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  function changeVolume(volume: number) {
    setData((current) => (current ? { ...current, volume } : current));
    if (volumeTimer.current !== null) window.clearTimeout(volumeTimer.current);
    volumeTimer.current = window.setTimeout(() => {
      api
        .soundboardSetVolume(volume)
        .then(setData)
        .catch((cause) =>
          setError(cause instanceof Error ? cause.message : String(cause)),
        );
    }, 180);
  }

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!data || event.repeat || event.ctrlKey || event.metaKey || event.altKey) return;
      const target = event.target as HTMLElement | null;
      if (target && /input|select|textarea/i.test(target.tagName)) return;
      const index = Number(event.key) - 1;
      if (index >= 0 && index < 9 && data.sounds[index]) {
        event.preventDefault();
        void play(data.sounds[index].id);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [data]);

  const sounds = data?.sounds ?? [];
  const muted = (data?.volume ?? 0) <= 0.001;

  return (
    <div className="page soundboard-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <AudioLines size={12} /> soundboard
          </span>
          <h1>Press play. They hear it.</h1>
          <p>
            Play your own sounds into a virtual microphone so your party hears them in
            game. Everything stays on this PC.
          </p>
        </div>
        <div className="head-status">
          <span
            className={cn(
              "status-pill",
              selectedOutput?.isVirtual && "online",
              !selectedOutput?.isVirtual && "warn",
            )}
          >
            <span className="dot" />
            {selectedOutput?.isVirtual
              ? "virtual mic route"
              : selectedOutput
                ? "speakers only"
                : "no output"}
          </span>
          <span className="checked">
            {sounds.length} sound{sounds.length === 1 ? "" : "s"} local
          </span>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <AlertTriangle size={17} />
          <div>
            <strong>Soundboard error</strong>
            <span>{error}</span>
          </div>
          <button className="button small" onClick={() => setError(null)}>
            Dismiss
          </button>
        </div>
      )}

      {notice && (
        <div className="action-result ok soundboard-notice">
          <Check size={15} />
          <span>{notice}</span>
        </div>
      )}

      <div className="soundboard-layout">
        <section className="panel soundboard-library">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Library</span>
              <h2>Your sounds</h2>
            </div>
            <div className="soundboard-head-actions">
              <button className="button small" onClick={() => void stopAll()}>
                <Square size={11} /> Stop all
              </button>
              <button
                className="button primary small"
                onClick={() => void importSounds()}
                disabled={busy !== null}
              >
                {busy === "import" ? (
                  <Loader2 size={13} className="spin" />
                ) : (
                  <Plus size={13} />
                )}
                Add sounds
              </button>
            </div>
          </div>

          {loading ? (
            <div className="center-state inline-state">
              <Loader2 size={18} className="spin" />
              <span>Reading the sound library</span>
            </div>
          ) : sounds.length ? (
            <>
              <div className="sound-grid">
                {sounds.map((sound, index) => (
                  <div
                    className={cn("sound-card", lastPlayed === sound.id && "playing")}
                    key={sound.id}
                  >
                    <button
                      className="sound-play"
                      onClick={() => void play(sound.id)}
                      title={`Play ${sound.name}`}
                    >
                      <Volume2 size={20} />
                    </button>
                    <div className="sound-copy">
                      <strong>{sound.name}</strong>
                      <span>{formatSize(sound.size)}</span>
                    </div>
                    <div className="sound-card-actions">
                      <button
                        className="sound-mini"
                        onClick={() => void stop(sound.id)}
                        title="Stop this sound"
                      >
                        <Square size={11} />
                      </button>
                      <button
                        className="sound-mini danger"
                        onClick={() => void remove(sound.id, sound.name)}
                        title="Remove sound"
                      >
                        <Trash2 size={11} />
                      </button>
                    </div>
                    {index < 9 && <kbd className="sound-key">{index + 1}</kbd>}
                  </div>
                ))}
              </div>
              <p className="soundboard-hint">
                Press <kbd>1</kbd>–<kbd>9</kbd> to fire sounds while this page is open.
              </p>
            </>
          ) : (
            <div className="sound-empty">
              <span className="sound-empty-icon">
                <AudioLines size={26} />
              </span>
              <h3>Drop in your first sound.</h3>
              <p>MP3, WAV, OGG, FLAC or M4A. Files are copied into your local app data.</p>
              <button className="button primary" onClick={() => void importSounds()}>
                <Plus size={15} /> Add sounds
              </button>
            </div>
          )}
        </section>

        <aside className="soundboard-side">
          <section className="panel soundboard-route">
            <div className="panel-heading">
              <div>
                <span className="panel-label">Routing</span>
                <h2>Virtual mic</h2>
              </div>
              <Mic size={19} className="panel-icon" />
            </div>

            <label className="field">
              <span>Soundboard output</span>
              <div className="soundboard-device-row">
                <select
                  value={data?.selectedDevice ?? ""}
                  onChange={(event) => void changeDevice(event.target.value)}
                  disabled={busy !== null}
                >
                  <option value="">Default output</option>
                  {(data?.outputs ?? []).map((device) => (
                    <option value={device.name} key={device.name}>
                      {device.name}
                      {device.isVirtual ? " · virtual" : ""}
                    </option>
                  ))}
                </select>
                <button className="icon-button" onClick={() => void load()} title="Refresh devices">
                  <RefreshCw size={13} className={busy === "device" ? "spin" : ""} />
                </button>
              </div>
              <small>
                Pick the input of your virtual cable here. VALORANT must listen to the
                matching cable output.
              </small>
            </label>

            <div className={cn("route-status", selectedOutput?.isVirtual ? "ready" : "warning")}>
              {selectedOutput?.isVirtual ? <Check size={15} /> : <AlertTriangle size={15} />}
              <div>
                <strong>
                  {selectedOutput?.isVirtual
                    ? "Virtual cable selected"
                    : selectedOutput
                      ? "This is a normal speaker output"
                      : "No output selected"}
                </strong>
                <span>
                  {selectedOutput?.isVirtual
                    ? "Sounds go into the cable. Set your game mic to its output."
                    : hasVirtualOutput
                      ? "Choose the cable input below to send sounds through your mic."
                      : "Install VB-Cable or VoiceMeeter to create a virtual mic."}
                </span>
              </div>
            </div>

            <div className="soundboard-volume">
              <span className="field-label">
                {muted ? <VolumeX size={13} /> : <Volume2 size={13} />} Volume
              </span>
              <input
                type="range"
                min={0}
                max={100}
                value={Math.round((data?.volume ?? 0) * 100)}
                onChange={(event) => changeVolume(Number(event.target.value) / 100)}
              />
              <strong>{Math.round((data?.volume ?? 0) * 100)}%</strong>
            </div>

            <div className="soundboard-test-row">
              <button
                className="button"
                onClick={() => void testRoute()}
                disabled={busy !== null}
              >
                {busy === "test" ? (
                  <Loader2 size={13} className="spin" />
                ) : (
                  <Headphones size={13} />
                )}
                Test route
              </button>
              <span>
                {selectedOutput?.isVirtual
                  ? "You may not hear this locally unless you monitor the cable."
                  : "Plays a short tone on the selected output."}
              </span>
            </div>

            <div className="route-guide">
              <strong>Send it through your mic</strong>
              <ol>
                <li>
                  Install{" "}
                  <button className="link-button" onClick={() => void openUrl("https://vb-audio.com/Cable/")}>
                    VB-Cable <ExternalLink size={10} />
                  </button>{" "}
                  or{" "}
                  <button className="link-button" onClick={() => void openUrl("https://vb-audio.com/Voicemeeter/")}>
                    VoiceMeeter <ExternalLink size={10} />
                  </button>
                  .
                </li>
                <li>Select the cable input as the soundboard output here.</li>
                <li>
                  In VALORANT, open Settings → Audio → Voice Chat Input and choose the
                  matching cable output.
                </li>
                <li>
                  Want your real voice and sounds together? Use VoiceMeeter: mic →
                  Hardware Input, Valotrack → VoiceMeeter Input, game mic → VoiceMeeter
                  Output.
                </li>
              </ol>
              <p className="route-warning">
                VB-Cable on its own replaces your microphone with the soundboard. Use
                VoiceMeeter if you still want your real voice in game.
              </p>
            </div>

            {defaultInput && (
              <div className="soundboard-default-mic">
                <Mic size={13} />
                <div>
                  <span>Windows default mic</span>
                  <strong>{defaultInput.name}</strong>
                </div>
                <em>{defaultInput.isVirtual ? "virtual" : "hardware"}</em>
              </div>
            )}

            <p className="soundboard-folder" title={data?.folder}>
              Stored locally in {data?.folder ?? "app data"}
            </p>
          </section>
        </aside>
      </div>
    </div>
  );
}
