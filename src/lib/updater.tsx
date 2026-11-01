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

export function UpdaterProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<UpdateState>({ status: "idle" });
  const found = useRef<Update | null>(null);
  const busy = useRef(false);

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
    const first = window.setTimeout(() => void checkNow(true), 5000);
    const timer = window.setInterval(() => void checkNow(true), 6 * 60 * 60 * 1000);
    return () => {
      window.clearTimeout(first);
      window.clearInterval(timer);
    };
  }, [checkNow]);

  const install = useCallback(async () => {
    const update = found.current;
    if (!update) {
      await checkNow();
      return;
    }
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
    } catch (cause) {
      setState({ status: "error", error: message(cause) });
    }
  }, [checkNow]);

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
