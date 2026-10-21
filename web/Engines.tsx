import { useEffect, useState } from "react";
import { native, type Engine } from "./native";
import { Icon } from "./Icon";

export function Engines({ busy }: { busy: boolean }) {
  const [engines, setEngines] = useState<Engine[]>([]);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    const refresh = () =>
      void native
        .engines()
        .then((rows) => {
          if (active) setEngines(rows);
        })
        .catch((e) => {
          if (active) setError(String(e));
        });
    refresh();
    const unlisten = native.onChange(refresh);
    return () => {
      active = false;
      void unlisten.then((off) => off());
    };
  }, []);
  async function configure(name?: string, remove = false) {
    setPending(true);
    setError("");
    try {
      if (name) await native.configureEngine(name, remove);
      else await native.setupTools();
      setEngines(await native.engines());
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(false);
    }
  }
  const ready = (names: string[]) =>
    names.every((name) => engines.some((e) => e.name === name && e.path));
  const allReady = ready(["ffmpeg", "ffprobe", "yt-dlp", "deno"]);
  return (
    <section className="engine-panel" aria-labelledby="tools-title">
      <h1 id="tools-title">Tool setup</h1>
      <p>
        Just choose a tool. PieAll downloads and configures what it needs on
        first use.
      </p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <div className="tool-setup-summary">
        <div>
          <h2>
            {allReady ? "Your tools are ready" : "Ready when you need them"}
          </h2>
          <p className="muted">
            Set up everything now, or let PieAll handle it as you go. Files are
            processed on your computer.
          </p>
        </div>
        <button
          className="primary"
          disabled={busy || pending || allReady}
          onClick={() => void configure()}
        >
          <Icon name={allReady ? "check" : "download"} size={17} />
          {pending ? "Setting up…" : allReady ? "All set" : "Set up tools"}
        </button>
      </div>
      <div className="tool-setup-rows">
        {[
          {
            title: "Video & audio",
            detail: "Convert, trim, compress and merge media",
            names: ["ffmpeg", "ffprobe"],
          },
          {
            title: "Browser downloads",
            detail: "Save videos and audio from supported links",
            names: ["yt-dlp", "deno"],
          },
        ].map((group) => (
          <div className="tool-setup-row" key={group.title}>
            <div>
              <h2>{group.title}</h2>
              <p className="muted">{group.detail}</p>
            </div>
            <span className="tool-setup-state">
              {ready(group.names) ? "Ready" : "Sets up automatically"}
            </span>
          </div>
        ))}
      </div>
      <p className="editor-note">
        Setup progress appears in your usual job notice. Close hides it; Cancel
        stops setup. An internet connection is needed the first time.
      </p>
      <details className="tool-advanced">
        <summary>Advanced · use your own tools</summary>
        <p className="muted">
          Choose executables from a trusted Windows installation. Keep FFmpeg
          and FFprobe in the same folder. After replacing a file, choose it
          again to update its recorded hash.
        </p>
        {engines.map((engine) => (
          <div className="tool-manual-row" key={engine.name}>
            <div>
              <strong>{engine.name}</strong>
              <p className="engine-path muted">
                {engine.path ?? "Automatic on first use"}
              </p>
              {engine.sha256 && (
                <details>
                  <summary>SHA-256</summary>
                  <code className="engine-hash">{engine.sha256}</code>
                </details>
              )}
            </div>
            <div className="button-row">
              <button
                disabled={busy || pending}
                onClick={() => void configure(engine.name)}
              >
                Choose file
              </button>
              {engine.path && (
                <button
                  disabled={busy || pending}
                  onClick={() => void configure(engine.name, true)}
                >
                  Remove
                </button>
              )}
            </div>
          </div>
        ))}
      </details>
    </section>
  );
}
