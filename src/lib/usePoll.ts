import { useCallback, useEffect, useRef } from "react";

export function usePoll(callback: () => void | Promise<void>, ms: number, active = true) {
  const saved = useRef(callback);
  saved.current = callback;

  const run = useCallback(async () => {
    try {
      await saved.current();
    } catch {
      void 0;
    }
  }, []);

  useEffect(() => {
    if (!active || ms <= 0) return;
    let stopped = false;
    let timer: number | undefined;

    async function tick() {
      await run();
      if (!stopped) timer = window.setTimeout(tick, ms);
    }

    timer = window.setTimeout(tick, ms);
    return () => {
      stopped = true;
      if (timer) window.clearTimeout(timer);
    };
  }, [active, ms, run]);

  return run;
}
