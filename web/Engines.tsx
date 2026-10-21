import { useEffect, useState } from "react";
import { native, type Engine } from "./native";
import { Icon } from "./Icon";

export function Engines({ busy }: { busy: boolean }) {
  const [engines, setEngines] = useState<Engine[]>([]);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    void native
      .engines()
      .then((rows) => {
        if (active) setEngines(rows);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, []);
  async function configure(name: string, remove = false) {
    setPending(true);
    setError("");
    try {
      await native.configureEngine(name, remove);
      setEngines(await native.engines());
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(false);
    }
  }
  return (
    <section className="engine-panel">
      <h1>Tool engines</h1>
      <p>
        Choose FFmpeg and FFprobe from a trusted Windows installation to enable
        audio and video tools. Processing stays on your computer. PieAll does
        not download engines automatically.
      </p>
      <p className="muted">
        Use both executables from the same distribution. PieAll records their
        file hashes. After updating them, choose them again here. Available
        formats depend on the codecs in your installation.
      </p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <div className="engine-list">
        {engines.map((engine) => (
          <article className="engine-card" key={engine.name}>
            <div>
              <h2>{engine.name === "ffmpeg" ? "FFmpeg" : "FFprobe"}</h2>
              <span className="muted">
                {engine.path
                  ? "Configured · checked before each use"
                  : "Not configured"}
              </span>
            </div>
            <p className="engine-path">
              {engine.path ??
                (engine.name === "ffmpeg"
                  ? "Converts and edits media."
                  : "Reads media details and conversion duration.")}
            </p>
            {engine.sha256 && (
              <details>
                <summary>Recorded SHA-256</summary>
                <code className="engine-hash">{engine.sha256}</code>
              </details>
            )}
            <div className="button-row">
              <button
                disabled={busy || pending}
                onClick={() => void configure(engine.name)}
              >
                <Icon name="open" size={17} />
                {engine.path ? "Choose again" : "Choose executable"}
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
          </article>
        ))}
      </div>
      <p className="editor-note">
        One job runs at a time. Video/audio jobs can run for up to one hour,
        with a 2 GiB output limit. Close hides a notification; Cancel stops the
        job and keeps completed outputs. Loudness normalization uses a single
        pass at −16 LUFS. Video conversion re-encodes the first video and
        optional first audio stream; subtitles and additional tracks are not
        copied.
      </p>
    </section>
  );
}
