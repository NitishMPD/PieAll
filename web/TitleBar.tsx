import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "./Icon";

/** Window buttons for the frameless dashboard; the parent header is the drag region. */
export function WindowControls() {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    let remove: (() => void) | undefined;
    let active = true;
    const win = getCurrentWindow();
    const sync = () =>
      void win.isMaximized().then((m) => active && setMaximized(m));
    sync();
    void win.onResized(sync).then((fn) => (active ? (remove = fn) : fn()));
    return () => {
      active = false;
      remove?.();
    };
  }, []);
  const win = getCurrentWindow();
  return (
    <div className="window-controls">
      <button aria-label="Minimize" onClick={() => void win.minimize()}>
        <Icon name="minimize" size={15} />
      </button>
      <button
        aria-label={maximized ? "Restore" : "Maximize"}
        onClick={() => void win.toggleMaximize()}
      >
        <Icon name={maximized ? "restore" : "maximize"} size={14} />
      </button>
      <button
        className="close"
        aria-label="Close to tray"
        onClick={() => void win.close()}
      >
        <Icon name="close" size={16} />
      </button>
    </div>
  );
}
