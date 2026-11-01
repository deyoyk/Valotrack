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
      if (stopped || document.hidden) return;
      await run();
      if (!stopped) timer = window.setTimeout(tick, ms);
    }

    function onVisibility() {
      if (document.hidden || stopped) return;
      if (timer) window.clearTimeout(timer);
      void tick();
    }

    timer = window.setTimeout(tick, ms);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      stopped = true;
      if (timer) window.clearTimeout(timer);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [active, ms, run]);

  return run;
}
