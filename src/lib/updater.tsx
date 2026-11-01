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
  checkNow: (silent?: boolean) => Promise<boolean | null>;
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

  const checkNow = useCallback(async (silent = false): Promise<boolean | null> => {
    if (busy.current) return null;
    if (!desktop()) {
      if (!silent) {
        setState({ status: "error", error: "Updates are available in the desktop app." });
      }
      return null;
    }
    busy.current = true;
    if (!silent) setState({ status: "checking" });
    try {
      // GitHub's release redirect can take more than 20 seconds on slow
      // networks. A short timeout made startup checks fail silently.
      const update = await check({ timeout: 60_000 });
      found.current = update;
      if (update) {
        setState({
          status: "available",
          version: update.version,
          notes: update.body ?? "",
        });
        return true;
      }
      if (!silent) setState({ status: "current" });
      return false;
    } catch (cause) {
      if (!silent) setState({ status: "error", error: message(cause) });
      return null;
    } finally {
      busy.current = false;
    }
  }, []);

  useEffect(() => {
    if (!desktop()) return;
    let cancelled = false;
    let retryTimer: number | undefined;

    const startup = async (attempt = 0) => {
      if (cancelled) return;
      const result = await checkNow(true);
      if (cancelled) return;

      if (result === true) {
        if (found.current && !developmentBuild()) {
          await installUpdate(found.current, true);
        }
        return;
      }

      // A failed network check must not wait six hours before trying again.
      if (result === null && attempt < 3) {
        retryTimer = window.setTimeout(
          () => void startup(attempt + 1),
          5_000 * (attempt + 1),
        );
      }
    };

    const startupTimer = window.setTimeout(() => void startup(), 1_500);
    const timer = window.setInterval(() => void checkNow(true), 6 * 60 * 60 * 1000);
    return () => {
      cancelled = true;
      window.clearTimeout(startupTimer);
      if (retryTimer) window.clearTimeout(retryTimer);
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
