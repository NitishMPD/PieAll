import type { Action, Preview } from "./native";

export function previewNumber(
  value: string | undefined,
  max: number,
  fallback = 0,
) {
  const n = Number(value);
  return Number.isFinite(n) ? Math.max(0, Math.min(max, n)) : fallback;
}

export function initialParams(
  action: Action,
  preview: Preview | null,
): Record<string, string> {
  const p = Object.fromEntries(action.fields.map((f) => [f.key, f.default]));
  if (action.id === "crop" && preview) {
    p.width = String(preview.width);
    p.height = String(preview.height);
  }
  return p;
}
export function fieldVisible(
  action: string,
  key: string,
  p: Record<string, string>,
) {
  if (action === "resize") {
    if (key === "percent") return p.mode === "percent";
    if (key === "longest") return p.mode === "longest";
    if (key === "width" || key === "height")
      return !p.mode || p.mode === "bounds";
  }
  if (action === "frame" && key === "secondary") return p.style === "gradient";
  if (action === "create_zip" && key === "level") return p.method !== "store";
  if (action === "images_pdf" && key === "orientation")
    return p.page_size !== "image";
  if (action === "convert" && (key === "quality" || key === "background"))
    return p.format === "jpg";
  return true;
}
export function resizeDimensions(
  width: number,
  height: number,
  p: Record<string, string>,
) {
  if (p.mode === "percent") {
    const factor = previewNumber(p.percent, 100, 100) / 100;
    return [
      Math.max(1, Math.floor(width * factor)),
      Math.max(1, Math.floor(height * factor)),
    ];
  }
  const scale =
    p.mode === "longest"
      ? Number(p.longest) / Math.max(width, height)
      : Math.min(Number(p.width) / width, Number(p.height) / height);
  const factor = Number.isFinite(scale) ? Math.max(0, Math.min(1, scale)) : 1;
  return [
    Math.max(1, Math.round(width * factor)),
    Math.max(1, Math.round(height * factor)),
  ];
}
export function frameDimensions(
  width: number,
  height: number,
  padding: number,
  ratio: string,
) {
  let w = width + padding * 2,
    h = height + padding * 2;
  const aspect =
    ratio === "16:9"
      ? [16, 9]
      : ratio === "4:5"
        ? [4, 5]
        : ratio === "1:1"
          ? [1, 1]
          : null;
  if (aspect) {
    w = Math.max(w, Math.ceil((h * aspect[0]) / aspect[1]));
    h = Math.max(h, Math.ceil((w * aspect[1]) / aspect[0]));
  }
  return [w, h];
}
