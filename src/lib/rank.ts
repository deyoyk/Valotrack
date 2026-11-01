const RANKS = [
  "Unranked",
  "Iron 1",
  "Iron 2",
  "Iron 3",
  "Bronze 1",
  "Bronze 2",
  "Bronze 3",
  "Silver 1",
  "Silver 2",
  "Silver 3",
  "Gold 1",
  "Gold 2",
  "Gold 3",
  "Platinum 1",
  "Platinum 2",
  "Platinum 3",
  "Diamond 1",
  "Diamond 2",
  "Diamond 3",
  "Ascendant 1",
  "Ascendant 2",
  "Ascendant 3",
  "Immortal 1",
  "Immortal 2",
  "Immortal 3",
  "Radiant",
];

const COLORS = [
  "#4A4A4A",
  "#5A5751",
  "#5A5751",
  "#5A5751",
  "#BB8F5A",
  "#BB8F5A",
  "#BB8F5A",
  "#AEB2B2",
  "#AEB2B2",
  "#AEB2B2",
  "#C5BA3F",
  "#C5BA3F",
  "#C5BA3F",
  "#18A7B9",
  "#18A7B9",
  "#18A7B9",
  "#D864C7",
  "#D864C7",
  "#D864C7",
  "#189452",
  "#189452",
  "#189452",
  "#DD4444",
  "#DD4444",
  "#DD4444",
  "#FFFDCD",
];

export function rankName(tier: number): string {
  return RANKS[Math.max(0, Math.min(25, tier))] ?? "Unranked";
}

export function rankColor(tier: number): string {
  return COLORS[Math.max(0, Math.min(25, tier))] ?? "#4A4A4A";
}

export function rankIcon(tier: number): string {
  return `https://media.valorant-api.com/competitivetiers/03621f52-342b-cf4e-4f86-9350a49c6d04/${Math.max(
    0,
    Math.min(25, tier),
  )}/smallicon.png`;
}
