export const SIZE = 440;
export function point(radius: number, angle: number): [number, number] {
  const a = (angle * Math.PI) / 180;
  return [SIZE / 2 + radius * Math.sin(a), SIZE / 2 - radius * Math.cos(a)];
}

export const OUTER = 190;
export const INNER = 96;
const GAP = 9;

type Pt = [number, number];
const lerp = (a: Pt, b: Pt, t: number): Pt => [
  a[0] + (b[0] - a[0]) * t,
  a[1] + (b[1] - a[1]) * t,
];
/** Drops `d` units of length from the start of a polyline. */
function trim(line: Pt[], d: number): Pt[] {
  for (let i = 1; i < line.length; i++) {
    const length = Math.hypot(
      line[i][0] - line[i - 1][0],
      line[i][1] - line[i - 1][1],
    );
    if (d < length)
      return [lerp(line[i - 1], line[i], d / length), ...line.slice(i)];
    d -= length;
  }
  return line.slice(-1);
}

// Pillow-shaped annular sectors: parallel gaps of GAP units, both arcs sampled,
// and each corner rounded by trimming its two edges and joining them with a curve.
export function wedgePath(index: number, count: number): string {
  if (count < 2 || count > 8 || index < 0 || index >= count)
    throw new RangeError("Invalid radial sector");
  const sweep = 360 / count;
  const arc = (r: number, reverse: boolean): Pt[] => {
    const inset = (Math.asin(GAP / 2 / r) * 180) / Math.PI;
    const a = index * sweep - sweep / 2 + inset;
    const b = index * sweep + sweep / 2 - inset;
    const steps = 24;
    const points = Array.from({ length: steps + 1 }, (_, i) =>
      point(r, a + ((b - a) * i) / steps),
    );
    return reverse ? points.reverse() : points;
  };
  const outer = arc(OUTER, false);
  const inner = arc(INNER, true);
  // Edges in order, each ending where the next begins; radius per corner at its end.
  const edges: [Pt[], number][] = [
    [outer, 22],
    [[outer[outer.length - 1], inner[0]], 14],
    [inner, 14],
    [[inner[inner.length - 1], outer[0]], 22],
  ];
  const parts: string[] = [];
  edges.forEach(([line, radius], i) => {
    const before = edges[(i + 3) % 4][1];
    const body = trim(trim(line, before).reverse(), radius).reverse();
    const corner = line[line.length - 1];
    const next = trim(edges[(i + 1) % 4][0], radius)[0];
    parts.push(
      `${i ? "L" : "M"}${body.map((p) => p.join(",")).join(" L")} Q${corner.join(",")} ${next.join(",")}`,
    );
  });
  return parts.join(" ") + " Z";
}

/** Arc in the tray gutter just outside a sector, for the hover-to-open progress
 * line. It travels with the popped slice and never crosses the slice's label. */
export function arcPath(
  index: number,
  count: number,
  radius = OUTER + 3,
): string {
  const sweep = 360 / count;
  const inset = 9 + (Math.asin(GAP / 2 / radius) * 180) / Math.PI;
  const [x1, y1] = point(radius, index * sweep - sweep / 2 + inset);
  const [x2, y2] = point(radius, index * sweep + sweep / 2 - inset);
  return `M${x1},${y1} A${radius},${radius} 0 0 1 ${x2},${y2}`;
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
