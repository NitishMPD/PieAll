export const SIZE = 440;
export function point(radius: number, angle: number): [number, number] {
  const a = (angle * Math.PI) / 180;
  return [SIZE / 2 + radius * Math.sin(a), SIZE / 2 - radius * Math.cos(a)];
}

// Rounded annular sectors: sample both circular edges, then round every join.
// This keeps the inner/outer corners soft without rounding away the radial gap.
export function wedgePath(index: number, count: number): string {
  if (count < 2 || count > 8 || index < 0 || index >= count)
    throw new RangeError("Invalid radial sector");
  const sweep = 360 / count;
  const start = index * sweep - sweep / 2 + 1.7;
  const end = index * sweep + sweep / 2 - 1.7;
  const steps = 20;
  const points: [number, number][] = [];
  for (let i = 0; i <= steps; i++)
    points.push(point(196, start + ((end - start) * i) / steps));
  for (let i = steps; i >= 0; i--)
    points.push(point(82, start + ((end - start) * i) / steps));
  const near = (a: [number, number], b: [number, number], d: number) => {
    const length = Math.hypot(b[0] - a[0], b[1] - a[1]);
    const t = Math.min(d / length, 0.45);
    return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
  };
  return (
    points
      .map((p, i) => {
        const corner =
          i === 0 || i === steps || i === steps + 1 || i === points.length - 1;
        const before = near(
          p,
          points[(i - 1 + points.length) % points.length],
          corner ? 12 : 1,
        );
        const after = near(p, points[(i + 1) % points.length], corner ? 12 : 1);
        return `${i ? "L" : "M"}${before.join(",")} Q${p.join(",")} ${after.join(",")}`;
      })
      .join(" ") + " Z"
  );
}

/** Sector under a viewBox point, by angle, so a flick past the rim still counts. */
export function sectorAt(
  x: number,
  y: number,
  count: number,
  dead = 66,
): number | null {
  const dx = x - SIZE / 2;
  const dy = y - SIZE / 2;
  if (count < 1 || Math.hypot(dx, dy) < dead) return null;
  const angle = ((Math.atan2(dx, -dy) * 180) / Math.PI + 360) % 360;
  return Math.round(angle / (360 / count)) % count;
}

const ranks: Record<string, string[]> = {
  images: ["convert", "crop", "resize", "frame", "compress"],
  files: [
    "copy_paths",
    "reveal",
    "create_zip",
    "extract_zip",
    "sha256",
    "duplicate",
    "terminal",
  ],
  text: ["trim", "json_format", "count", "upper", "lower", "sort"],
  window: ["center_window", "always_top", "fit_window", "window_details"],
};
export function ranked<T extends { id: string; eligible: boolean }>(
  actions: T[],
  kind: string,
): T[] {
  const order = ranks[kind] ?? [];
  const score = (id: string) => (order.includes(id) ? order.indexOf(id) : 100);
  return actions
    .filter((a) => a.eligible)
    .sort((a, b) => score(a.id) - score(b.id));
}
