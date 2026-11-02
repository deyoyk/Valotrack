export type TextScale = "comfortable" | "large" | "xlarge";

export type Preferences = {
  textScale: TextScale;
  reduceMotion: boolean;
};

const KEY = "valotrack-preferences";

const DEFAULTS: Preferences = {
  textScale: "comfortable",
  reduceMotion: false,
};

export function loadPreferences(): Preferences {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Preferences>;
    const textScale =
      raw.textScale === "large" || raw.textScale === "xlarge"
        ? raw.textScale
        : "comfortable";
    return { textScale, reduceMotion: Boolean(raw.reduceMotion) };
  } catch {
    return { ...DEFAULTS };
  }
}

export function applyPreferences(preferences: Preferences): void {
  document.documentElement.dataset.textScale = preferences.textScale;
  document.documentElement.dataset.reduceMotion = String(preferences.reduceMotion);
}

export function savePreferences(preferences: Preferences): void {
  localStorage.setItem(KEY, JSON.stringify(preferences));
  applyPreferences(preferences);
  window.dispatchEvent(new CustomEvent("valotrack:preferences", { detail: preferences }));
}
