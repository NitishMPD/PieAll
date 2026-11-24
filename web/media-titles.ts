import type { MediaItem } from "./native";

export const mediaKinds = [
  { kind: "video", label: "Videos", single: "Video" },
  { kind: "image", label: "Images", single: "Image" },
  { kind: "gif", label: "GIFs", single: "GIF" },
] as const;

/** A human-readable label, or null when the text looks like an ID or hash. */
export function readableName(raw: string | null | undefined) {
  if (!raw) return null;
  let text = raw.trim();
  try {
    text = decodeURIComponent(text);
  } catch {
    // Keep the raw text when it is not valid percent-encoding.
  }
  text = text.replace(/\.[a-z0-9]{2,5}$/i, "").trim();
  const words = text.split(/[\s_\-+.]+/).filter(Boolean);
  if (!words.length) return null;
  const hashLike = words.some(
    (word) => word.length >= 6 && /\d/.test(word) && /\p{L}/u.test(word),
  );
  const hasWord = words.some((word) => /^\p{L}{3,}$/u.test(word));
  if (hashLike || !hasWord) return null;
  return words.join(" ");
}

function fileName(url: string) {
  try {
    return new URL(url).pathname.split("/").filter(Boolean).at(-1) ?? null;
  } catch {
    return null;
  }
}

/** Display titles keyed by URL; IDs fall back to "Video 1", "Image 3", … */
export function mediaTitles(items: MediaItem[]) {
  const counts = { video: 0, image: 0, gif: 0 };
  const titles = new Map<string, string>();
  for (const item of items) {
    const number = ++counts[item.kind];
    const single = mediaKinds.find((k) => k.kind === item.kind)!.single;
    titles.set(
      item.url,
      readableName(item.name) ??
        readableName(fileName(item.url)) ??
        `${single} ${number}`,
    );
  }
  return titles;
}
