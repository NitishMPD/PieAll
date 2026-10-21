import { useSyncExternalStore } from "react";

/** User choices, shared by the dashboard and the ring through localStorage. */
export interface Prefs {
  /** `KeyboardEvent.code` of the hold key. */
  trigger: string;
  /** Ring layout per context (`images`, `app:<profile>`…): ordered menu item keys. */
  layouts: Record<string, string[]>;
}

/** Keys the native hook accepts, as `KeyboardEvent.code` → Windows virtual key. */
export const TRIGGER_KEYS: Record<string, { vk: number; label: string }> = {
  Backquote: { vk: 0xc0, label: "`" },
  IntlBackslash: { vk: 0xe2, label: "\\ (ISO)" },
  CapsLock: { vk: 0x14, label: "Caps Lock" },
  Pause: { vk: 0x13, label: "Pause" },
  ScrollLock: { vk: 0x91, label: "Scroll Lock" },
  ...Object.fromEntries(
    Array.from({ length: 12 }, (_, i) => [
      `F${i + 1}`,
      { vk: 0x70 + i, label: `F${i + 1}` },
    ]),
  ),
};

const STORAGE_KEY = "pieall.prefs";
const defaults: Prefs = { trigger: "Backquote", layouts: {} };

function read(): Prefs {
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
    return {
      trigger: raw.trigger in TRIGGER_KEYS ? raw.trigger : defaults.trigger,
      layouts:
        raw.layouts && typeof raw.layouts === "object" ? raw.layouts : {},
    };
  } catch {
    return defaults;
  }
}

let cache = read();
const listeners = new Set<() => void>();
function emit() {
  cache = read();
  listeners.forEach((fn) => fn());
}
if (typeof window !== "undefined") window.addEventListener("storage", emit);

export function getPrefs(): Prefs {
  return cache;
}
export function savePrefs(next: Prefs) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  emit();
}
export function usePrefs(): Prefs {
  return useSyncExternalStore(
    (fn) => {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
    () => cache,
    () => defaults,
  );
}

/** Saves one context's layout; `null` returns it to the built-in menu. */
export function setLayout(context: string, keys: string[] | null) {
  const layouts = { ...cache.layouts };
  if (keys) layouts[context] = keys;
  else delete layouts[context];
  savePrefs({ ...cache, layouts });
}
