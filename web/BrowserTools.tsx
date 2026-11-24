import { useEffect, useRef, useState } from "react";
import {
  native,
  type DownloadSettings,
  type MediaItem,
  type Session,
} from "./native";
import { Icon } from "./Icon";
import { MediaPickerDialog } from "./MediaPicker";
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
  const [media, setMedia] = useState<MediaItem[] | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [names, setNames] = useState<Record<string, string>>({});
  const [picking, setPicking] = useState(false);
  const [pickerError, setPickerError] = useState("");
  const reviewButton = useRef<HTMLButtonElement>(null);
  const wasPicking = useRef(false);
  useEffect(() => {
    // Return focus to Review media; Scan media may be disabled or gone.
    if (wasPicking.current && !picking) reviewButton.current?.focus();
    wasPicking.current = picking;
  }, [picking]);
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
  function closePicker() {
    setPicking(false);
    setPickerError("");
  }
  async function changeReviewSetting(enabled: boolean) {
    setPending(true);
    setError("");
    try {
      const saved = await native.configureDownloads(null, false, enabled);
      setSettings((current) =>
        current
          ? { ...current, review_multiple: saved.review_multiple }
          : saved,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(false);
    }
  }
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
    setPickerError("");
    setMessage("");
    try {
      await native.downloadUrl(
        mode,
        url,
        settings.preferences,
        mode === "download_video" && media?.length
          ? media
              .filter((item) => selected.includes(item.url))
              .map((item) => ({ url: item.url, name: names[item.url] || null }))
          : null,
      );
      setPicking(false);
      if (
        mode !== "download_video" ||
        !settings.review_multiple ||
        media?.length
      )
        setMessage(
          "Download started. You can start another or keep using PieAll.",
        );
    } catch (e) {
      if (picking) setPickerError(String(e));
      else setError(String(e));
    } finally {
      setPending(false);
    }
  }
  async function scan() {
    setPending(true);
    setError("");
    setMedia(null);
    setSelected([]);
    setNames({});
    setMessage("Scanning the page for media…");
    try {
      const items = await native.scanMedia(url);
      setMedia(items);
      setSelected(items.map((item) => item.url));
      setPicking(items.length > 0);
      setMessage(
        items.length
          ? `Found ${items.length} media ${items.length === 1 ? "item" : "items"}.`
          : "The scan found no media. Download media can still try the installed extractors.",
      );
    } catch (e) {
      setError(String(e));
      setMessage("");
    } finally {
      setPending(false);
    }
  }
  const choosingMedia = mode === "download_video" && !!media?.length;
  return (
    <section
      className="engine-panel browser-panel"
      aria-labelledby="browser-title"
    >
      <h1 id="browser-title">Save the media from a post.</h1>
      <p>
        Paste an X post, Pinterest pin, public LinkedIn post (full URL or
        lnkd.in share link), Playtester game page or another media link. Scan
        the page to choose which items to save in one folder.
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
              onChange={(e) => {
                setUrl(e.target.value);
                setMedia(null);
                setSelected([]);
                setNames({});
                setMessage("");
                setError("");
              }}
              aria-describedby="download-source-note"
            />
            <p id="download-source-note" className="muted">
              Open the exact post or pin first. Login-only, protected and
              dynamically loaded media may be unavailable. Up to 100 files and 2
              GiB per job; feeds and boards are not collected.
            </p>
            {mode === "download_video" && (
              <button
                type="button"
                disabled={!url.trim()}
                onClick={() => void scan()}
              >
                Scan media
              </button>
            )}
            {choosingMedia && (
              <div className="media-summary">
                <p>
                  <strong>{selected.length}</strong> of {media.length} media{" "}
                  {media.length === 1 ? "item" : "items"} selected
                </p>
                <button
                  type="button"
                  ref={reviewButton}
                  onClick={() => setPicking(true)}
                >
                  <Icon name="grid" size={17} />
                  Review media
                </button>
              </div>
            )}
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
              <button
                className="primary"
                type="submit"
                disabled={!url.trim() || (choosingMedia && !selected.length)}
              >
                <Icon name="download" size={17} />
                {mode === "download_video"
                  ? choosingMedia
                    ? `Download selected (${selected.length})`
                    : "Download media"
                  : "Download audio"}
              </button>
              <button type="button" onClick={() => void save(false)}>
                Save as defaults
              </button>
            </div>
            <label className="download-review-setting">
              <input
                type="checkbox"
                checked={settings.review_multiple}
                onChange={(event) =>
                  void changeReviewSetting(event.target.checked)
                }
              />
              Ask which media to download when there are multiple items
            </label>
          </fieldset>
        </form>
      )}
      {choosingMedia && (
        <MediaPickerDialog
          open={picking}
          items={media}
          referer={url}
          selected={selected}
          onSelectedChange={setSelected}
          names={names}
          onRename={(url, name) =>
            setNames((current) => ({ ...current, [url]: name }))
          }
          onConfirm={() => void download()}
          onClose={closePicker}
          closeLabel="Close"
          pending={pending}
          error={pickerError}
          note={settings ? `Saves to ${settings.destination}` : undefined}
        />
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
