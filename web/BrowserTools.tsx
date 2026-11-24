import { useEffect, useState } from "react";
import { native, type DownloadSettings, type Session } from "./native";
import { Icon } from "./Icon";
import "./browser-tools.css";

export function BrowserTools({ session }: { session: Session }) {
  const [settings, setSettings] = useState<DownloadSettings | null>(null);
  const [url, setUrl] = useState("");
  const [mode, setMode] = useState<"download_video" | "download_audio">(
    "download_video",
  );
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  useEffect(() => {
    let active = true;
    void native
      .downloadSettings()
      .then((value) => {
        if (active) setSettings(value);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, []);
  const disabled = !!session.busy || !!session.editor || pending;
  const job =
    session.jobs?.find((job) => job.title.startsWith("Download ")) ?? null;
  async function save(chooseFolder: boolean) {
    if (!settings) return;
    setPending(true);
    setError("");
    setMessage("");
    try {
      const next = await native.configureDownloads(
        settings.preferences,
        chooseFolder,
      );
      setSettings(next);
      setMessage(
        chooseFolder
          ? `Download folder: ${next.destination}`
          : "Defaults saved.",
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(false);
    }
  }
  async function download() {
    if (!settings) return;
    setPending(true);
    setError("");
    setMessage("");
    try {
      await native.downloadUrl(mode, url, settings.preferences);
      setMessage(
        "Download started. You can start another or keep using PieAll.",
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(false);
    }
  }
  return (
    <section
      className="engine-panel browser-panel"
      aria-labelledby="browser-title"
    >
      <h1 id="browser-title">Save the media from a post.</h1>
      <p>
        Paste an X post, Pinterest pin, public LinkedIn post (full URL or
        lnkd.in share link) or another media link. PieAll saves discoverable
        images, GIFs and videos together in a folder. Original images and
        animation are preserved.
      </p>
      <p className="muted">
        Or hold your PieAll key in a browser and choose Download media. PieAll
        copies the current page address and downloads in the background. Open
        the exact video or post first when browsing a feed.
      </p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {message && <p role="status">{message}</p>}
      {!settings ? (
        <p role="status">
          {error
            ? "Download settings could not be loaded. Reopen Browser downloads to retry."
            : "Loading download settings…"}
        </p>
      ) : (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void download();
          }}
        >
          <fieldset disabled={disabled}>
            <legend className="sr-only">Download source and format</legend>
            <label htmlFor="download-url">Post, pin or media URL</label>
            <input
              id="download-url"
              type="url"
              required
              maxLength={8192}
              placeholder="https://www.youtube.com/watch?v=…"
              autoComplete="off"
              spellCheck={false}
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              aria-describedby="download-source-note"
            />
            <p id="download-source-note" className="muted">
              Open the exact post or pin first. Login-only, protected and
              dynamically loaded media may be unavailable. Up to 100 files and 2
              GiB per job; feeds and boards are not collected.
            </p>
            <div className="download-options">
              <label>
                Save as
                <select
                  value={mode}
                  onChange={(e) => setMode(e.target.value as typeof mode)}
                >
                  <option value="download_video">
                    Images, GIFs &amp; video
                  </option>
                  <option value="download_audio">Audio</option>
                </select>
              </label>
              {mode === "download_video" ? (
                <>
                  <label>
                    Video quality
                    <select
                      value={settings.preferences.quality}
                      onChange={(e) =>
                        setSettings({
                          ...settings,
                          preferences: {
                            ...settings.preferences,
                            quality: e.target.value,
                          },
                        })
                      }
                    >
                      <option value="best">Highest available</option>
                      {[2160, 1440, 1080, 720, 480].map((n) => (
                        <option key={n} value={String(n)}>
                          Up to {n}p
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    Video container
                    <select
                      value={settings.preferences.container}
                      onChange={(e) =>
                        setSettings({
                          ...settings,
                          preferences: {
                            ...settings.preferences,
                            container: e.target.value,
                          },
                        })
                      }
                    >
                      <option value="mkv">MKV · flexible codecs</option>
                      <option value="mp4">MP4</option>
                      <option value="webm">WebM</option>
                    </select>
                  </label>
                </>
              ) : (
                <label>
                  Audio format
                  <select
                    value={settings.preferences.audio}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        preferences: {
                          ...settings.preferences,
                          audio: e.target.value,
                        },
                      })
                    }
                  >
                    <option value="original">Original · no conversion</option>
                    {["mp3", "m4a", "opus", "flac", "wav"].map((f) => (
                      <option key={f} value={f}>
                        {f.toUpperCase()}
                      </option>
                    ))}
                  </select>
                </label>
              )}
            </div>
            <p className="muted">
              Images, GIFs and directly linked videos keep their original
              format. Video quality and container choices apply to extracted
              streams. If codecs do not fit the container, choose MKV. Audio
              conversion cannot improve the source quality.
            </p>
            <div className="download-folder">
              <span>
                Save to
                <strong title={settings.destination}>
                  {settings.destination}
                </strong>
              </span>
              <button type="button" onClick={() => void save(true)}>
                <Icon name="open" size={17} />
                Choose folder
              </button>
            </div>
            <div className="button-row">
              <button className="primary" type="submit" disabled={!url.trim()}>
                <Icon name="download" size={17} />
                {mode === "download_video"
                  ? "Download media"
                  : "Download audio"}
              </button>
              <button type="button" onClick={() => void save(false)}>
                Save as defaults
              </button>
            </div>
          </fieldset>
        </form>
      )}
      {job && (
        <section className="download-result" aria-label="Download result">
          <h2>
            {job.state === "running" || job.state === "cancelling"
              ? "Download in progress"
              : job.state === "complete"
                ? "Download saved"
                : "Download stopped"}
          </h2>
          <p role="status">{job.detail}</p>
          {session.result && <pre tabIndex={0}>{session.result}</pre>}
        </section>
      )}
      <p className="editor-note">
        Tools set up automatically on first use. Close hides progress; Cancel
        stops the job.
      </p>
    </section>
  );
}
