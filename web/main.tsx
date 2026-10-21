import React, {
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { native, type Action, type Session, type Preview } from "./native";
import {
  indexActions,
  contextualActions,
  PAGE_SIZE,
  searchActions,
} from "./action-search";
import { Ring } from "./Ring";
import { Toast } from "./Toast";
import { Icon } from "./Icon";
import { Customize, applyTrigger } from "./Customize";
import { TRIGGER_KEYS, usePrefs } from "./prefs";
import "./styles.css";
import "./dashboard.css";

const surface =
  typeof location !== "undefined"
    ? new URLSearchParams(location.search).get("surface")
    : null;
const standalone = surface === "ring";
if (typeof document !== "undefined")
  document.documentElement.dataset.surface =
    surface === "ring" || surface === "toast" ? surface : "dashboard";

export function App({
  initialSession = null,
}: {
  initialSession?: Session | null;
}) {
  if (surface === "toast") return <Toast />;
  return <Shell initialSession={initialSession} />;
}

function Shell({ initialSession }: { initialSession: Session | null }) {
  const [session, setSession] = useState<Session | null>(initialSession);
  const [error, setError] = useState("");
  const [thumbnail, setThumbnail] = useState<Preview | null>(null);
  const [group, setGroup] = useState("All actions");
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const deferredQuery = useDeferredValue(query);
  const [view, setView] = useState<"actions" | "customize" | "updates">(
    "actions",
  );
  const updates = view === "updates";
  const setUpdates = useCallback(
    (open: boolean) => setView(open ? "updates" : "actions"),
    [],
  );
  const prefs = usePrefs();
  const triggerLabel = TRIGGER_KEYS[prefs.trigger]?.label ?? "`";
  const [confirmInstall, setConfirmInstall] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const sequence = useRef(0);
  const actionIndex = useMemo(
    () => indexActions(contextualActions(session?.actions ?? [], session?.app)),
    [session?.actions, session?.app],
  );
  const counts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const { action } of actionIndex)
      counts.set(action.group, (counts.get(action.group) ?? 0) + 1);
    return counts;
  }, [actionIndex]);
  const actions = useMemo(
    () => searchActions(actionIndex, deferredQuery, group),
    [actionIndex, deferredQuery, group],
  );
  const currentPage = Math.min(
    page,
    Math.max(0, Math.ceil(actions.length / PAGE_SIZE) - 1),
  );
  const visibleActions = actions.slice(
    currentPage * PAGE_SIZE,
    (currentPage + 1) * PAGE_SIZE,
  );
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    list.current?.scrollTo(0, 0);
  }, [currentPage, deferredQuery, group]);
  useEffect(() => {
    setQuery("");
    setPage(0);
  }, [session?.app]);
  useEffect(() => {
    // The native hook starts on the backtick; restore a saved choice.
    if (!standalone && prefs.trigger !== "Backquote")
      void applyTrigger(prefs.trigger).then(
        (message) => message && setError(message),
      );
  }, [prefs.trigger]);
  const refresh = useCallback(async () => {
    const seq = ++sequence.current;
    const next = await native.session();
    if (seq === sequence.current) setSession(next);
  }, []);
  const perform = useCallback(
    async (fn: () => Promise<unknown>) => {
      setError("");
      try {
        await fn();
      } catch (e) {
        setError(String(e));
      }
      try {
        await refresh();
      } catch (e) {
        setError(String(e));
      }
    },
    [refresh],
  );
  const browseActions = useCallback(() => {
    if (standalone) {
      void perform(() => native.surface("search"));
    } else {
      setUpdates(false);
      setGroup(session?.app ? "Apps" : "All actions");
      setQuery("");
      setPage(0);
      requestAnimationFrame(() =>
        document.getElementById("action-search")?.focus(),
      );
    }
  }, [perform, session?.app]);
  useEffect(() => {
    if (standalone) return;
    let active = true;
    const subscription = native.onSearch(() => {
      if (active) browseActions();
    });
    void subscription.catch((e) => {
      if (active) setError(String(e));
    });
    return () => {
      active = false;
      void subscription.then((remove) => remove()).catch(() => {});
    };
  }, [browseActions]);
  useEffect(() => {
    let active = true;
    let remove: (() => void) | undefined;
    native
      .onChange(() => {
        if (active) void refresh().catch((e) => setError(String(e)));
      })
      .then((unlisten) => {
        if (!active) {
          unlisten();
          return;
        }
        remove = unlisten;
        void refresh().catch((e) => setError(String(e)));
      })
      .catch((e) => setError(`Unable to connect to PieAll: ${String(e)}`));
    return () => {
      active = false;
      remove?.();
      sequence.current++;
    };
  }, [refresh]);
  useEffect(() => {
    let active = true;
    setThumbnail(null);
    if (session?.kind === "images")
      native
        .preview(session.revision)
        .then((image) => {
          if (active) setThumbnail(image);
        })
        .catch(() => {});
    return () => {
      active = false;
    };
  }, [session?.revision, session?.kind]);
  useEffect(() => {
    function key(event: KeyboardEvent) {
      if (
        event.key === "/" &&
        !standalone &&
        !session?.editor &&
        !(event.target instanceof HTMLInputElement) &&
        !(event.target instanceof HTMLTextAreaElement) &&
        !(event.target instanceof HTMLSelectElement)
      ) {
        event.preventDefault();
        setUpdates(false);
        document.getElementById("action-search")?.focus();
        return;
      }
      if (event.key !== "Escape") return;
      if (session?.editor) {
        event.preventDefault();
        void perform(() => native.prepare(null, session.revision));
      } else if (view !== "actions") setView("actions");
      else if (!standalone) void perform(() => native.surface("dismiss"));
    }
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [session?.editor, session?.revision, view, perform]);
  function runPreset(id: string, params: Record<string, string>) {
    const action = session?.actions.find((a) => a.id === id);
    if (action) activate(action, params);
  }
  function activate(action: Action, params?: Record<string, string>) {
    if (!session) return;
    void perform(() =>
      action.fields.length && !params
        ? native.prepare(action.id, session.revision)
        : native.run(action.id, session.revision, params),
    );
  }
  if (!session)
    return (
      <main className="connection" aria-live="polite">
        <img src="/brand/pieall.svg" width={48} height={48} alt="" />
        <h1>PieAll</h1>
        <p>{error || "Connecting to your local tools…"}</p>
        {error && (
          <button
            onClick={() => void refresh().catch((e) => setError(String(e)))}
          >
            Try again
          </button>
        )}
      </main>
    );
  const disabled = !!session.busy || !!session.editor;
  if (standalone)
    return (
      <main className="floating-ring">
        <Ring
          session={session}
          thumbnail={thumbnail?.url ?? null}
          standalone
          onBrowse={browseActions}
          onRun={runPreset}
          onClose={() => void perform(() => native.surface("dismiss"))}
        />
        {error && (
          <div className="ring-error" role="alert">
            {error}
          </div>
        )}
      </main>
    );
  const editor = session.actions.find((a) => a.id === session.editor);
  return (
    <div className="app-shell">
      <header className="app-header">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setGroup("All actions");
            setView("actions");
          }}
        >
          <img
            className="brand-symbol"
            src="/brand/pieall.svg"
            width={40}
            height={40}
            alt=""
          />
          <span>
            PieAll<span className="brand-sub">A little less work.</span>
          </span>
        </a>
        <div className="header-actions">
          <span className="local-label">
            <span /> Runs on this PC
          </span>
          <button
            className="quiet"
            disabled={disabled}
            onClick={() => void perform(() => native.surface("ring"))}
          >
            <Icon name="ring" size={17} />
            Show ring <kbd>Hold {triggerLabel}</kbd>
          </button>
        </div>
      </header>
      <div className="body-layout">
        <nav className="sidebar" aria-label="Sections">
          <h2 className="nav-heading">Actions</h2>
          {[
            "All actions",
            "Images",
            "Files",
            "Archives",
            "Text",
            "PDF",
            "Window",
            "Apps",
          ].map((name) => (
            <button
              key={name}
              className={
                group === name && view === "actions"
                  ? "nav-item active"
                  : "nav-item"
              }
              onClick={() => {
                setGroup(name);
                setPage(0);
                setView("actions");
              }}
              aria-current={
                group === name && view === "actions" ? "page" : undefined
              }
            >
              <Icon name={name === "All actions" ? "ring" : name} />
              {name}
              <span>
                {name === "All actions"
                  ? actionIndex.length
                  : (counts.get(name) ?? 0)}
              </span>
            </button>
          ))}
          <h2 className="nav-heading">Personalize</h2>
          <button
            className={`nav-item ${view === "customize" ? "active" : ""}`}
            aria-current={view === "customize" ? "page" : undefined}
            onClick={() => setView("customize")}
          >
            <Icon name="sliders" />
            Customize ring
          </button>
          <div className="sidebar-bottom">
            <button
              className={`nav-item ${updates ? "active" : ""}`}
              aria-current={updates ? "page" : undefined}
              onClick={() => setUpdates(true)}
            >
              <Icon name="update" />
              Updates{session.update.version && <span className="new-dot" />}
            </button>
            <small>PieAll · on your PC</small>
          </div>
        </nav>
        <main className="workspace">
          <section className="selection-bar" aria-label="Current selection">
            <div className="selection-icon">
              <Icon name={session.kind === "empty" ? "files" : session.kind} />
            </div>
            <div className="selection-name">
              <span>Working with</span>
              <h1 title={session.target}>{session.target}</h1>
            </div>
            <div className="selection-actions">
              <button
                disabled={disabled}
                onClick={() => void perform(() => native.load("files"))}
              >
                <Icon name="open" size={17} />
                Open files
              </button>
              <button
                className="icon-button"
                disabled={disabled}
                title="Use clipboard text"
                aria-label="Use clipboard text"
                onClick={() => void perform(() => native.load("clipboard"))}
              >
                <Icon name="clipboard" size={19} />
              </button>
            </div>
          </section>
          {view === "customize" ? (
            <Customize session={session} />
          ) : updates ? (
            <section className="updates-panel">
              <button className="text-button" onClick={() => setUpdates(false)}>
                <Icon name="back" size={18} />
                Back to actions
              </button>
              <h2>Keep PieAll up to date.</h2>
              <p>{session.update.message}</p>
              {!session.update.configured && (
                <p className="muted">
                  Update delivery is available in signed release builds. This
                  development copy does not check the network.
                </p>
              )}
              {session.update.version && (
                <>
                  <h3>Version {session.update.version}</h3>
                  <pre className="release-notes">
                    {session.update.notes || "No release notes supplied."}
                  </pre>
                </>
              )}
              {session.busy === "update" && (
                <progress
                  max="100"
                  value={session.update.progress}
                  aria-label="Update download progress"
                />
              )}
              <div className="button-row">
                <button
                  disabled={disabled || !session.update.configured}
                  onClick={() => void perform(() => native.update("check"))}
                >
                  Check for updates
                </button>
                {session.update.version && !session.update.ready && (
                  <button
                    className="primary"
                    disabled={disabled}
                    onClick={() =>
                      void perform(() => native.update("download"))
                    }
                  >
                    Download update
                  </button>
                )}
                {session.update.ready && (
                  <button
                    className="primary"
                    disabled={disabled}
                    onClick={() => setConfirmInstall(session.update.version)}
                  >
                    Install update…
                  </button>
                )}
              </div>
              {confirmInstall && (
                <div
                  className="install-confirm"
                  role="group"
                  aria-label="Confirm update installation"
                >
                  <p>
                    Install {confirmInstall} and close PieAll? Your files stay
                    in place. Reopen PieAll after the installer finishes.
                  </p>
                  <button onClick={() => setConfirmInstall(null)}>
                    Cancel
                  </button>
                  <button
                    className="primary"
                    disabled={
                      disabled ||
                      !session.update.ready ||
                      session.update.version !== confirmInstall
                    }
                    onClick={() =>
                      void perform(() =>
                        native.update("install", confirmInstall),
                      )
                    }
                  >
                    Install and close
                  </button>
                </div>
              )}
            </section>
          ) : (
            <div className="work-columns">
              <section className="radial-panel" aria-label="Quick actions">
                <div className="panel-heading">
                  <h2>
                    {session.kind === "empty"
                      ? "Start anywhere."
                      : "What’s next?"}
                  </h2>
                  <p>
                    {session.kind === "empty"
                      ? "Select something in Windows, or drop files here."
                      : "A few useful moves, right at your fingertips."}
                  </p>
                </div>
                {session.kind === "empty" ? (
                  <ol className="steps" aria-label="How PieAll works">
                    <li>
                      <b>1</b>
                      <span>
                        <strong>Select</strong>
                        Files in Explorer, text anywhere, or just an app window.
                      </span>
                    </li>
                    <li>
                      <b>2</b>
                      <span>
                        <strong>
                          Hold <kbd>{triggerLabel}</kbd>
                        </strong>
                        The ring opens right at your cursor.
                      </span>
                    </li>
                    <li>
                      <b>3</b>
                      <span>
                        <strong>Release on an action</strong>
                        Results save beside the original.
                      </span>
                    </li>
                    <li className="steps-cta">
                      <button
                        className="primary"
                        disabled={disabled}
                        onClick={() => void perform(() => native.load("files"))}
                      >
                        <Icon name="open" size={17} />
                        Choose files
                      </button>
                      <button onClick={() => setView("customize")}>
                        <Icon name="sliders" size={17} />
                        Customize ring
                      </button>
                    </li>
                  </ol>
                ) : (
                  <Ring
                    session={session}
                    thumbnail={thumbnail?.url ?? null}
                    onBrowse={browseActions}
                    onRun={runPreset}
                    onClose={() => void perform(() => native.load("files"))}
                  />
                )}
                <div className="radial-caption">
                  <span className="shortcut-mark">
                    <Icon name="ring" size={16} />
                  </span>
                  <p>
                    Anywhere in Windows, hold <kbd>{triggerLabel}</kbd>
                    <br />
                    <span>
                      {session.app
                        ? "App shortcuts run in the captured app."
                        : "Release on an action. Results save beside the original."}
                    </span>
                  </p>
                </div>
                {session.result !== null && (
                  <section className="result-panel" aria-label="Action result">
                    <div>
                      <h3>Done &amp; ready.</h3>
                      <button
                        className="text-button"
                        onClick={() =>
                          void perform(async () => {
                            await native.copy();
                            setCopied(true);
                            setTimeout(() => setCopied(false), 1800);
                          })
                        }
                      >
                        <Icon name={copied ? "check" : "copy"} size={16} />
                        {copied ? "Copied" : "Copy result"}
                      </button>
                    </div>
                    <pre tabIndex={0}>{session.result}</pre>
                  </section>
                )}
              </section>
              <section
                className="actions-panel"
                aria-label={editor ? "Action editor" : "Action browser"}
              >
                {editor ? (
                  <Editor
                    preview={thumbnail}
                    key={`${editor.id}-${session.revision}`}
                    action={editor}
                    session={session}
                    thumbnail={thumbnail?.url ?? null}
                    onCancel={() =>
                      void perform(() => native.prepare(null, session.revision))
                    }
                    onRun={(params) => activate(editor, params)}
                  />
                ) : (
                  <>
                    <div className="actions-heading">
                      <h2>{group}</h2>
                      <span>{actions.length}</span>
                    </div>
                    <label className="search">
                      <Icon name="search" size={18} />
                      <input
                        id="action-search"
                        placeholder="Find an action…"
                        aria-label="Search actions"
                        value={query}
                        onChange={(e) => {
                          setQuery(e.target.value);
                          setPage(0);
                        }}
                      />
                      <kbd>/</kbd>
                    </label>
                    {group === "Apps" && (
                      <div className="app-filter">
                        <p>
                          {session.app
                            ? "Shortcuts for the captured app. Windows US defaults; editing mode and custom keymaps can change their behavior."
                            : "Hold backtick in a supported app to see its shortcuts."}
                        </p>
                      </div>
                    )}
                    <div
                      className="action-list"
                      ref={list}
                      aria-busy={query !== deferredQuery}
                    >
                      {actions.length ? (
                        visibleActions.map((action) => (
                          <button
                            className="action-row"
                            key={action.id}
                            disabled={!action.eligible || disabled}
                            onClick={() => activate(action)}
                          >
                            <span
                              className={`action-icon icon-${action.group.toLowerCase()}`}
                            >
                              <Icon
                                name={
                                  [
                                    "convert",
                                    "crop",
                                    "resize",
                                    "frame",
                                    "compress",
                                  ].includes(action.id)
                                    ? action.id
                                    : action.group
                                }
                                size={20}
                              />
                            </span>
                            <span>
                              <strong>{action.title}</strong>
                              <small>
                                {action.shortcut
                                  ? `${action.appName ?? "Current app"} · ${action.category ?? "Shortcut"}${action.eligible ? "" : " · invoke from this app"}`
                                  : action.eligible
                                    ? action.group
                                    : `Needs ${action.group === "Window" ? "a captured window" : action.group === "Text" ? "clipboard text" : "compatible files"}`}
                              </small>
                            </span>
                            {action.shortcut ? (
                              <kbd className="action-keys">
                                {action.shortcut}
                              </kbd>
                            ) : (
                              <Icon name="arrow" size={16} />
                            )}
                          </button>
                        ))
                      ) : (
                        <p className="no-actions">
                          No actions match “{query}”. Try a different name.
                        </p>
                      )}
                    </div>
                    {actions.length > PAGE_SIZE && (
                      <nav
                        className="action-pages"
                        aria-label="Action result pages"
                      >
                        <button
                          disabled={currentPage === 0}
                          onClick={() => setPage(currentPage - 1)}
                        >
                          Previous
                        </button>
                        <span role="status">
                          {currentPage * PAGE_SIZE + 1}–
                          {Math.min(
                            (currentPage + 1) * PAGE_SIZE,
                            actions.length,
                          )}{" "}
                          of {actions.length}
                        </span>
                        <button
                          disabled={
                            (currentPage + 1) * PAGE_SIZE >= actions.length
                          }
                          onClick={() => setPage(currentPage + 1)}
                        >
                          Next
                        </button>
                      </nav>
                    )}
                  </>
                )}
              </section>
            </div>
          )}
          <footer
            className={`status-bar ${session.busy ? "is-busy" : ""}`}
            role="status"
            aria-live="polite"
          >
            {session.busy ? (
              <span className="spinner" />
            ) : (
              <Icon name="check" size={16} />
            )}
            <span>{session.status}</span>
          </footer>
          {error && (
            <div className="error-banner" role="alert">
              <span>{error}</span>
              <button
                className="icon-button"
                aria-label="Dismiss error"
                onClick={() => setError("")}
              >
                <Icon name="close" size={16} />
              </button>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

function Editor({
  action,
  session,
  thumbnail,
  preview,
  onCancel,
  onRun,
}: {
  action: Action;
  session: Session;
  thumbnail: string | null;
  preview: Preview | null;
  onCancel: () => void;
  onRun: (params: Record<string, string>) => void;
}) {
  const [params, setParams] = useState<Record<string, string>>(() =>
    Object.fromEntries(action.fields.map((f) => [f.key, f.default])),
  );
  const first = useRef<HTMLInputElement | HTMLSelectElement | null>(null);
  useEffect(() => {
    first.current?.focus();
  }, []);
  return (
    <form
      className="editor"
      onSubmit={(event) => {
        event.preventDefault();
        onRun(params);
      }}
    >
      <button type="button" className="text-button" onClick={onCancel}>
        <Icon name="back" size={18} />
        All actions
      </button>
      <h2>{action.title}</h2>
      <p className="muted">A new file is saved beside the original.</p>
      {preview && (
        <ImagePreview preview={preview} action={action.id} params={params} />
      )}
      {action.fields.map((field, index) => (
        <label className="field" key={field.key}>
          <span>{field.label}</span>
          {field.key === "format" ? (
            <select
              ref={(el) => {
                if (index === 0) first.current = el;
              }}
              value={params[field.key]}
              onChange={(e) =>
                setParams({ ...params, [field.key]: e.target.value })
              }
            >
              <option value="png">PNG</option>
              <option value="jpg">JPEG</option>
              <option value="webp">WebP (lossless)</option>
            </select>
          ) : (
            <input
              ref={(el) => {
                if (index === 0) first.current = el;
              }}
              required
              maxLength={field.key === "background" ? 7 : 10}
              type={field.key === "background" ? "text" : "number"}
              min={0}
              step={1}
              value={params[field.key]}
              onChange={(e) =>
                setParams({ ...params, [field.key]: e.target.value })
              }
            />
          )}
        </label>
      ))}
      <p className="editor-note">
        {session.names.length > 1
          ? `Applies to all ${session.names.length} selected files. `
          : ""}
        Preview uses the first image. Final dimensions and validation are
        applied by the native worker.
      </p>
      <div className="button-row">
        <button type="button" onClick={onCancel} disabled={!!session.busy}>
          Cancel
        </button>
        <button className="primary" type="submit" disabled={!!session.busy}>
          Run action
          <Icon name="arrow" size={17} />
        </button>
      </div>
    </form>
  );
}

function ImagePreview({
  preview,
  action,
  params,
}: {
  preview: Preview;
  action: string;
  params: Record<string, string>;
}) {
  const { width, height, url } = preview;
  const x = Number(params.x || 0),
    y = Number(params.y || 0);
  const w = Number(params.width || 0),
    h = Number(params.height || 0);
  const valid =
    x >= 0 && y >= 0 && w > 0 && h > 0 && x + w <= width && y + h <= height;
  const padding =
    action === "frame" ? Math.max(0, Number(params.padding || 0)) : 0;
  const background = /^[0-9a-f]{6}$/i.test(params.background || "")
    ? `#${params.background}`
    : "#F3EBDD";
  const resultScale =
    action === "resize" ? Math.min(1, w / width, h / height) : 1;
  return (
    <figure className="image-preview">
      <svg
        viewBox={`${-padding} ${-padding} ${width + padding * 2} ${height + padding * 2}`}
        role="img"
        aria-label={`Approximate ${action} preview of first selected image`}
      >
        <defs>
          <clipPath id="image-corners">
            <rect
              width={width}
              height={height}
              rx={action === "frame" ? Number(params.corners || 0) : 0}
            />
          </clipPath>
          {action === "crop" && valid && (
            <clipPath id="crop-preview">
              <rect x={x} y={y} width={w} height={h} />
            </clipPath>
          )}
        </defs>
        {action === "frame" && (
          <rect
            x={-padding}
            y={-padding}
            width={width + padding * 2}
            height={height + padding * 2}
            fill={background}
          />
        )}
        <image
          href={url}
          width={width}
          height={height}
          clipPath="url(#image-corners)"
          opacity={action === "crop" ? 0.25 : 1}
        />
        {action === "crop" && valid && (
          <>
            <image
              href={url}
              width={width}
              height={height}
              clipPath="url(#crop-preview)"
            />
            <rect
              x={x}
              y={y}
              width={w}
              height={h}
              fill="none"
              stroke="#9b4f29"
              strokeWidth="2"
              vectorEffect="non-scaling-stroke"
            />
          </>
        )}
      </svg>
      <figcaption>
        {action === "crop"
          ? valid
            ? `${w} x ${h} px crop`
            : "Crop extends outside this image"
          : action === "resize"
            ? `${Math.round(width * resultScale)} x ${Math.round(height * resultScale)} px - no upscaling`
            : `${width} x ${height} px original`}
      </figcaption>
    </figure>
  );
}
