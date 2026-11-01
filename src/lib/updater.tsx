import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "downloading"
  | "ready"
  | "error";

export type UpdateState = {
  status: UpdateStatus;
  version?: string;
  notes?: string;
  progress?: number;
  error?: string;
};

type UpdaterApi = UpdateState & {
  checkNow: (silent?: boolean) => Promise<void>;
  install: () => Promise<void>;
  restart: () => Promise<void>;
};

const UpdaterContext = createContext<UpdaterApi | null>(null);

function message(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

function desktop() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function developmentBuild() {
  return typeof window !== "undefined" && window.location.port === "1420";
}

export function UpdaterProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<UpdateState>({ status: "idle" });
  const found = useRef<Update | null>(null);
  const busy = useRef(false);
  const installing = useRef(false);

  const installUpdate = useCallback(async (update: Update, auto: boolean) => {
    if (installing.current) return;
    installing.current = true;
    setState({ status: "downloading", version: update.version, progress: 0 });
    try {
      let total = 0;
      let received = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
          setState((previous) => ({ ...previous, progress: 0 }));
        } else if (event.event === "Progress") {
          received += event.data.chunkLength;
          setState((previous) => ({
            ...previous,
            progress: total > 0 ? Math.min(99, Math.round((received / total) * 100)) : 0,
          }));
        } else {
          setState((previous) => ({ ...previous, progress: 100 }));
        }
      });
      setState({ status: "ready", version: update.version, progress: 100 });
      if (auto) {
        await relaunch();
      }
    } catch (cause) {
      setState({ status: "error", error: message(cause), version: update.version });
    } finally {
      installing.current = false;
    }
  }, []);

  const checkNow = useCallback(async (silent = false) => {
    if (busy.current) return;
    if (!desktop()) {
      if (!silent) {
        setState({ status: "error", error: "Updates are available in the desktop app." });
      }
      return;
    }
    busy.current = true;
    if (!silent) setState({ status: "checking" });
    try {
      const update = await check({ timeout: 20000 });
      found.current = update;
      if (update) {
        setState({
          status: "available",
          version: update.version,
          notes: update.body ?? "",
        });
      } else if (!silent) {
        setState({ status: "current" });
      }
    } catch (cause) {
      if (!silent) setState({ status: "error", error: message(cause) });
    } finally {
      busy.current = false;
    }
  }, []);

  useEffect(() => {
    if (!desktop()) return;
    let cancelled = false;
    const startup = window.setTimeout(async () => {
      if (cancelled) return;
      await checkNow(true);
      if (cancelled) return;
      const update = found.current;
      // Dev builds should never install the public release over the working tree.
      if (update && !developmentBuild()) {
        await installUpdate(update, true);
      }
    }, 1500);
    const timer = window.setInterval(() => void checkNow(true), 6 * 60 * 60 * 1000);
    return () => {
      cancelled = true;
      window.clearTimeout(startup);
      window.clearInterval(timer);
    };
  }, [checkNow, installUpdate]);

  const install = useCallback(async () => {
    const update = found.current;
    if (!update) {
      await checkNow();
      return;
    }
    await installUpdate(update, false);
  }, [checkNow, installUpdate]);

  const restart = useCallback(async () => {
    await relaunch();
  }, []);

  return (
    <UpdaterContext.Provider value={{ ...state, checkNow, install, restart }}>
      {children}
    </UpdaterContext.Provider>
  );
}

export function useUpdater() {
  const context = useContext(UpdaterContext);
  if (!context) throw new Error("useUpdater must be used inside UpdaterProvider");
  return context;
}
