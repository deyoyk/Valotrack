export function displayVersion(version: string | null | undefined): string {
  if (!version) return "";
  const trimmed = version.trim().replace(/^v/, "");
  const build = /^1\.0\.(\d+)$/.exec(trimmed);
  if (build) return `v${build[1]}`;
  return `v${trimmed}`;
}
