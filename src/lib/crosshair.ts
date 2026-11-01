export type LineSettings = {
  thickness: number;
  length: number;
  offset: number;
  opacity: number;
  movement: boolean;
  firing: boolean;
};

export type CrosshairSettings = {
  colorIndex: number;
  customColor: string;
  outlines: boolean;
  outlineOpacity: number;
  dot: boolean;
  dotSize: number;
  dotOpacity: number;
  inner: LineSettings;
  outer: LineSettings;
  sniperColorIndex: number;
  sniperCustomColor: string;
  sniperOutlineOpacity: number;
};

export const COLORS = [
  "#FFFFFF",
  "#00FF00",
  "#BFFF00",
  "#FFFF00",
  "#00FFFF",
  "#FF1493",
  "#FF0000",
  "#B565B5",
  "#FFD700",
];

export function defaultSettings(): CrosshairSettings {
  return {
    colorIndex: 0,
    customColor: "#FFFFFF",
    outlines: true,
    outlineOpacity: 1,
    dot: false,
    dotSize: 2,
    dotOpacity: 1,
    inner: { thickness: 1, length: 4, offset: 2, opacity: 1, movement: false, firing: false },
    outer: { thickness: 1, length: 2, offset: 3, opacity: 0.35, movement: false, firing: false },
    sniperColorIndex: 5,
    sniperCustomColor: "#FF1493",
    sniperOutlineOpacity: 1,
  };
}

export function parseCrosshair(code: string): CrosshairSettings {
  const settings = defaultSettings();
  const tokens = String(code || "")
    .split(";")
    .map((token) => token.trim());
  let section = "P";
  for (let index = 0; index < tokens.length; index += 1) {
    const key = tokens[index];
    if (key === "P" || key === "A" || key === "S") {
      section = key;
      continue;
    }
    const value = tokens[index + 1];
    if (value === undefined) break;
    const number = Number(value);
    const target = section === "S" ? null : section === "A" ? null : settings;
    if (key === "c") {
      const color = Number.isFinite(number) ? number : 0;
      if (section === "S") settings.sniperColorIndex = color;
      else if (target) target.colorIndex = color;
      index += 1;
      continue;
    }
    if (key === "u") {
      if (section === "S") settings.sniperCustomColor = normalizeHex(value);
      else if (target) target.customColor = normalizeHex(value);
      index += 1;
      continue;
    }
    if (key === "h" && target) {
      target.outlines = number === 1;
      index += 1;
      continue;
    }
    if (key === "o") {
      if (section === "S") settings.sniperOutlineOpacity = number;
      else if (target) target.outlineOpacity = number;
      index += 1;
      continue;
    }
    if (key === "d" && target) {
      target.dot = number === 1;
      index += 1;
      continue;
    }
    if (key === "z" && target) {
      target.dotSize = number;
      index += 1;
      continue;
    }
    if (key === "a" && target) {
      target.dotOpacity = number;
      index += 1;
      continue;
    }
    if (key === "m" && target) {
      target.inner.movement = number === 1;
      target.outer.movement = number === 1;
      index += 1;
      continue;
    }
    if (key === "f" && target) {
      target.inner.firing = number === 1;
      target.outer.firing = number === 1;
      index += 1;
      continue;
    }
    if (/^[01][tlomfa]$/.test(key) && target) {
      const line = key[0] === "0" ? target.inner : target.outer;
      switch (key[1]) {
        case "t":
          line.thickness = number;
          break;
        case "l":
          line.length = number;
          break;
        case "o":
          line.offset = number;
          break;
        case "a":
          line.opacity = number;
          break;
        case "m":
          line.movement = number === 1;
          break;
        case "f":
          line.firing = number === 1;
          break;
      }
      index += 1;
      continue;
    }
    index += 1;
  }
  return settings;
}

function normalizeHex(value: string) {
  const raw = value.replace(/[^0-9a-f]/gi, "").toUpperCase();
  if (raw.length >= 8) return `#${raw.slice(0, 6)}`;
  if (raw.length >= 6) return `#${raw.slice(0, 6)}`;
  return "#FF1493";
}

export function crosshairColor(settings: CrosshairSettings): string {
  if (settings.colorIndex === 8 || settings.colorIndex > 8) return settings.customColor;
  return COLORS[settings.colorIndex] ?? COLORS[0];
}

export function buildCrosshair(settings: CrosshairSettings): string {
  const parts = ["0", "P"];
  parts.push("c", String(settings.colorIndex));
  if (settings.colorIndex === 8) {
    parts.push("u", `${settings.customColor.replace("#", "").toUpperCase()}FF`);
  }
  parts.push("h", settings.outlines ? "1" : "0");
  parts.push("o", formatNumber(settings.outlineOpacity));
  parts.push("d", settings.dot ? "1" : "0");
  parts.push("z", formatNumber(settings.dotSize));
  parts.push("a", formatNumber(settings.dotOpacity));
  parts.push("m", settings.inner.movement || settings.outer.movement ? "1" : "0");
  parts.push("f", settings.inner.firing || settings.outer.firing ? "1" : "0");
  for (const [prefix, line] of [
    ["0", settings.inner],
    ["1", settings.outer],
  ] as const) {
    parts.push(`${prefix}t`, formatNumber(line.thickness));
    parts.push(`${prefix}l`, formatNumber(line.length));
    parts.push(`${prefix}o`, formatNumber(line.offset));
    parts.push(`${prefix}a`, formatNumber(line.opacity));
    parts.push(`${prefix}m`, line.movement ? "1" : "0");
    parts.push(`${prefix}f`, line.firing ? "1" : "0");
  }
  parts.push("S", "c", String(settings.sniperColorIndex));
  if (settings.sniperColorIndex === 8) {
    parts.push("u", `${settings.sniperCustomColor.replace("#", "").toUpperCase()}FF`);
  }
  parts.push("o", formatNumber(settings.sniperOutlineOpacity));
  return parts.join(";");
}

function formatNumber(value: number) {
  return Number.isInteger(value) ? String(value) : String(Math.round(value * 1000) / 1000);
}

export function parseColor(value: string): string {
  return value.startsWith("#") ? value : `#${value}`;
}
