import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface Field {
  key: string;
  label: string;
  default: string;
  options?: [string, string][];
}
export interface Preview {
  url: string;
  width: number;
  height: number;
}
export interface Action {
  id: string;
  title: string;
  group: string;
  eligible: boolean;
  unavailable?: string | null;
  fields: Field[];
  shortcut?: string | null;
  category?: string | null;
  profile?: string;
  appName?: string;
  priority?: number | null;
}
export interface ShortcutLibrary {
  profiles: { id: string; name: string; source: string }[];
  actions: {
    id: string;
    profile: string;
    title: string;
    keys: string;
    category: string;
    priority?: number | null;
  }[];
}
export interface Session {
  job?: Job | null;
  revision: number;
  target: string;
  names: string[];
  kind: "images" | "files" | "text" | "window" | "empty";
  app?: string | null;
  pinned?: boolean;
  busy: string | null;
  editor: string | null;
  status: string;
  result: string | null;
  actions: Action[];
  update: {
    configured: boolean;
    version: string | null;
    notes: string;
    ready: boolean;
    message: string;
    progress: number;
  };
}
export const native = {
  setupTools: () => invoke<void>("setup_tools"),
  downloadSettings: () => invoke<DownloadSettings>("download_settings"),
  configureDownloads: (
    preferences: DownloadPreferences,
    chooseFolder = false,
  ) =>
    invoke<DownloadSettings>("configure_downloads", {
      preferences,
      chooseFolder,
    }),
  downloadUrl: (
    id: "download_video" | "download_audio",
    url: string,
    preferences: DownloadPreferences,
  ) => invoke<void>("download_url", { id, url, preferences }),
  engines: () => invoke<Engine[]>("tool_engines"),
  configureEngine: (name: string, remove = false) =>
    invoke<void>("configure_engine", { name, remove }),
  dismissToast: (id: number, job?: number) =>
    invoke<void>("dismiss_toast", { id, job: job ?? null }),
  openDownloadLocation: (id: number) =>
    invoke<void>("open_download_location", { id }),
  cancelJob: (id: number) => invoke<void>("cancel_job", { id }),
  session: () => invoke<Session>("session"),
  shortcutLibrary: () => invoke<ShortcutLibrary>("shortcut_library"),
  setTrigger: (vk: number) => invoke<void>("set_trigger_key", { vk }),
  autostart: () => invoke<boolean>("autostart_enabled"),
  setAutostart: (enabled: boolean) =>
    invoke<void>("set_autostart", { enabled }),
  surface: (action: "dashboard" | "ring" | "dismiss" | "search") =>
    invoke<void>("surface", { action }),
  load: (kind: "files" | "clipboard") => invoke<void>("load_context", { kind }),
  prepare: (id: string | null, revision: number) =>
    invoke<void>("prepare", { id, revision }),
  run: (id: string, revision: number, params: Record<string, string> = {}) =>
    invoke<void>("run_action", { id, revision, params }),
  preview: (revision: number) =>
    invoke<Preview | null>("preview", { revision }),
  copy: () => invoke<void>("copy_result"),
  update: (
    action: "check" | "download" | "install",
    expectedVersion?: string,
  ) => invoke<void>("update_action", { action, expectedVersion }),
  onSearch: (fn: () => void) => listen("search-actions", fn),
  onChange: (fn: () => void) => listen("session-changed", fn),
  /** The ring opened; `held` is true while the backtick key is still down. */
  onRing: (fn: (held: boolean) => void) =>
    listen<boolean>("ring-opened", (e) => fn(e.payload)),
  /** The held key came up with the cursor at this ring viewBox point. */
  onRelease: (fn: (x: number, y: number) => void) =>
    listen<[number, number]>("ring-release", (e) => fn(...e.payload)),
  onToast: (fn: (toast: Toast) => void) =>
    listen<Toast>("toast", (e) => fn(e.payload)),
  /** The native side is about to hide the ring: play the exit. */
  onClosing: (fn: () => void) => listen("ring-close", fn),
  onToastHide: (fn: () => void) => listen("toast-hide", fn),
};
export interface Engine {
  name: string;
  path: string | null;
  sha256: string | null;
}
export interface DownloadPreferences {
  quality: string;
  container: string;
  audio: string;
}
export interface DownloadSettings {
  destination: string;
  preferences: DownloadPreferences;
}
export interface Toast {
  id: number;
  job?: Job | null;
  tone: "ok" | "info" | "error" | "busy";
  title: string;
  detail: string;
}
export interface Job {
  output?: string | null;
  id: number;
  title: string;
  detail: string;
  completed: number;
  total: number;
  fraction?: number | null;
  state:
    "running" | "cancelling" | "complete" | "partial" | "failed" | "cancelled";
}
