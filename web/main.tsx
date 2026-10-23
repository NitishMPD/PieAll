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
import { Splash } from "./Splash";
import { WindowControls } from "./TitleBar";
import { Icon, iconFor } from "./Icon";
import { Customize, applyTrigger } from "./Customize";
import { Engines } from "./Engines";
import { Jobs } from "./Jobs";
import { BrowserTools } from "./BrowserTools";
import { TRIGGER_KEYS, usePrefs } from "./prefs";
import {
  fieldVisible,
  frameDimensions,
  initialParams,
  previewNumber,
  resizeDimensions,
} from "./editor-fields";
import "./styles.css";
import "./dashboard.css";
import "./customize.css";

const surface =
  typeof location !== "undefined"
    ? new URLSearchParams(location.search).get("surface")
    : null;
const standalone = surface === "ring";
if (typeof document !== "undefined")
  document.documentElement.dataset.surface =
    surface === "ring" || surface === "toast" || surface === "splash"
      ? surface
      : "dashboard";

export function App({
  initialSession = null,
}: {
  initialSession?: Session | null;
}) {
  if (surface === "toast") return <Toast />;
  if (surface === "splash") return <Splash />;
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
  const [view, setView] = useState<
    "actions" | "customize" | "updates" | "engines" | "browser"
  >("actions");
  const [customizeOpened, setCustomizeOpened] = useState(false);
  useEffect(() => {
    if (view === "customize") setCustomizeOpened(true);
  }, [view]);
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
      <header className="app-header" data-tauri-drag-region>
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
          <span>PieAll</span>
        </a>
        <div className="header-actions">
          <button
            className="quiet"
            disabled={disabled}
            onClick={() => void perform(() => native.surface("ring"))}
          >
            <Icon name="ring" size={17} />
            Show ring <kbd>Hold {triggerLabel}</kbd>
          </button>
          <WindowControls />
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
            className={`nav-item ${view === "browser" ? "active" : ""}`}
            aria-current={view === "browser" ? "page" : undefined}
            onClick={() => setView("browser")}
          >
            <Icon name="download" />
            Browser downloads
          </button>
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
              className={`nav-item ${view === "engines" ? "active" : ""}`}
              aria-current={view === "engines" ? "page" : undefined}
              onClick={() => setView("engines")}
            >
              <Icon name="sliders" />
              Tool setup
            </button>
            <button
              className={`nav-item ${updates ? "active" : ""}`}
              aria-current={updates ? "page" : undefined}
              onClick={() => setUpdates(true)}
            >
              <Icon name="update" />
              Updates{session.update.version && <span className="new-dot" />}
            </button>
          </div>
        </nav>
        <main className="workspace">
          {view !== "customize" && view !== "engines" && view !== "browser" && (
            <section className="selection-bar" aria-label="Current selection">
              <div className="selection-icon">
                <Icon
                  name={session.kind === "empty" ? "files" : session.kind}
                />
              </div>
              <div className="selection-name">
                <h1 title={session.target}>
                  {session.kind === "empty"
                    ? "Nothing selected"
                    : session.target}
                </h1>
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
          )}
          {(view === "customize" || customizeOpened) && (
            <div className="customize-host" hidden={view !== "customize"}>
              <Customize session={session} active={view === "customize"} />
            </div>
          )}
          {view === "browser" ? (
            <BrowserTools session={session} />
          ) : view === "engines" ? (
            <Engines busy={disabled} />
          ) : view === "customize" ? null : updates ? (
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
              {session.update.busy && (
                <progress
                  max="100"
                  value={session.update.progress}
                  aria-label="Update download progress"
                />
              )}
              <div className="button-row">
                <button
                  disabled={
                    disabled ||
                    session.update.busy ||
                    !session.update.configured
                  }
                  onClick={() => void perform(() => native.update("check"))}
                >
                  Check for updates
                </button>
                {session.update.version && !session.update.ready && (
                  <button
                    className="primary"
                    disabled={disabled || session.update.busy}
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
                    disabled={disabled || session.update.busy}
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
                      session.update.busy ||
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
                      ? "Select something in Windows, or drop files onto this window."
                      : "A few useful moves, right at your fingertips."}
                  </p>
                </div>
                {session.kind === "empty" ? (
                  <ol className="steps" aria-label="How PieAll works">
                    <li>
                      <b>
                        <Icon name="files" size={15} />
                      </b>
                      <span>
                        <strong>Select</strong>
                        Files in Explorer, text, or any app window.
                      </span>
                    </li>
                    <li>
                      <b>
                        <Icon name="ring" size={15} />
                      </b>
                      <span>
                        <strong>
                          Hold <kbd>{triggerLabel}</kbd>
                        </strong>
                        The ring opens at your cursor.
                      </span>
                    </li>
                    <li>
                      <b>
                        <Icon name="open" size={15} />
                      </b>
                      <span>
                        <strong>Release on a slice</strong>
                        Results save next to the original.
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
                {session.kind !== "empty" && (
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
                )}
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
                                  action.shortcut
                                    ? iconFor(action.title)
                                    : [
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
                                  : action.unavailable
                                    ? action.unavailable
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
          <Jobs jobs={session.jobs ?? []} perform={perform} />
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
    initialParams(action, preview),
  );
  const first = useRef<HTMLInputElement | HTMLSelectElement | null>(null);
  const cropEdited = useRef(false);
  useEffect(() => {
    if (action.id === "crop" && preview && !cropEdited.current) {
      setParams((p) => ({
        ...p,
        width: String(preview.width),
        height: String(preview.height),
      }));
    }
  }, [action.id, preview]);
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
        <ImagePreview
          preview={preview}
          action={action.id}
          params={params}
          onCrop={(crop) => {
            cropEdited.current = true;
            setParams((p) => ({ ...p, ...crop }));
          }}
        />
      )}
      {action.id === "crop" && preview && (
        <div className="button-row" aria-label="Crop aspect presets">
          {[
            ["Original", preview.width / preview.height],
            ["Square", 1],
            ["16:9", 16 / 9],
            ["4:5", 4 / 5],
          ].map(([label, value]) => (
            <button
              key={String(label)}
              type="button"
              onClick={() => {
                cropEdited.current = true;
                const ratio = Number(value),
                  w = Math.min(
                    preview.width,
                    Math.floor(preview.height * ratio),
                  ),
                  h = Math.min(preview.height, Math.floor(w / ratio));
                setParams({
                  ...params,
                  x: String(Math.floor((preview.width - w) / 2)),
                  y: String(Math.floor((preview.height - h) / 2)),
                  width: String(w),
                  height: String(h),
                });
              }}
            >
              {label}
            </button>
          ))}
        </div>
      )}
      {action.fields
        .filter((f) => fieldVisible(action.id, f.key, params))
        .map((field, index) => (
          <label className="field" key={field.key}>
            <span>{field.label}</span>
            {field.options?.length || field.key === "format" ? (
              <select
                ref={(el) => {
                  if (index === 0) first.current = el;
                }}
                value={params[field.key]}
                onChange={(e) => {
                  cropEdited.current = true;
                  setParams({ ...params, [field.key]: e.target.value });
                }}
              >
                {(field.options?.length
                  ? field.options
                  : [
                      ["png", "PNG"],
                      ["jpg", "JPEG"],
                      ["webp", "WebP (lossless)"],
                    ]
                ).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            ) : (
              <input
                ref={(el) => {
                  if (index === 0) first.current = el;
                }}
                required
                maxLength={
                  field.key === "pages"
                    ? 256
                    : ["background", "secondary"].includes(field.key)
                      ? 7
                      : 10
                }
                type={
                  ["background", "secondary", "pages"].includes(field.key)
                    ? "text"
                    : "number"
                }
                min={0}
                step={
                  ["start", "duration", "gain", "fps"].includes(field.key)
                    ? "any"
                    : 1
                }
                value={params[field.key]}
                onChange={(e) => {
                  cropEdited.current = true;
                  setParams({ ...params, [field.key]: e.target.value });
                }}
              />
            )}
          </label>
        ))}
      <p className="editor-note">
        {session.names.length > 1
          ? `Applies to all ${session.names.length} selected files. `
          : ""}
        {action.id === "images_pdf"
          ? "Pages follow the selected file order. Images use a white background."
          : action.id.startsWith("pdf_")
            ? "Page ranges keep document order. Rewriting refuses forms and signed PDFs. Extracting pages or removing document metadata is not secure redaction. Text extraction does not perform OCR."
            : /^(video_|audio_|media_)/.test(action.id)
              ? "Requires configured FFmpeg and FFprobe. Outputs keep the first selected media streams; video is re-encoded. Originals stay untouched."
              : "Preview uses the first image. Final dimensions and validation are applied by the native worker."}
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
  onCrop,
}: {
  preview: Preview;
  action: string;
  params: Record<string, string>;
  onCrop?: (crop: Record<string, string>) => void;
}) {
  const drag = useRef<{ x: number; y: number } | null>(null);
  const { width, height, url } = preview;
  const x = Number(params.x || 0),
    y = Number(params.y || 0);
  const w = Number(params.width || 0),
    h = Number(params.height || 0);
  const valid =
    x >= 0 && y >= 0 && w > 0 && h > 0 && x + w <= width && y + h <= height;
  const padding = action === "frame" ? previewNumber(params.padding, 2048) : 0;
  const background = /^[0-9a-f]{6}$/i.test(params.background || "")
    ? `#${params.background}`
    : "#F3EBDD";
  const resized = resizeDimensions(width, height, params);
  const [fw, fh] = frameDimensions(
    width,
    height,
    padding,
    params.ratio || "natural",
  );
  const ox = (fw - width) / 2,
    oy = (fh - height) / 2;
  const secondary = /^[0-9a-f]{6}$/i.test(params.secondary || "")
    ? `#${params.secondary}`
    : "#D3B8A0";
  return (
    <figure className="image-preview">
      <svg
        viewBox={`${-ox} ${-oy} ${fw} ${fh}`}
        className={action === "crop" ? "crop-canvas" : undefined}
        onPointerDown={(event) => {
          if (action !== "crop" || !onCrop || event.button !== 0) return;
          const matrix = event.currentTarget.getScreenCTM();
          if (!matrix) return;
          const point = new DOMPoint(
            event.clientX,
            event.clientY,
          ).matrixTransform(matrix.inverse());
          drag.current = {
            x: Math.max(0, Math.min(width - 1, Math.round(point.x))),
            y: Math.max(0, Math.min(height - 1, Math.round(point.y))),
          };
          event.currentTarget.setPointerCapture(event.pointerId);
        }}
        onPointerMove={(event) => {
          if (!drag.current || !onCrop) return;
          const matrix = event.currentTarget.getScreenCTM();
          if (!matrix) return;
          const point = new DOMPoint(
            event.clientX,
            event.clientY,
          ).matrixTransform(matrix.inverse());
          const dx = Math.max(0, Math.min(width, Math.round(point.x))),
            dy = Math.max(0, Math.min(height, Math.round(point.y)));
          onCrop({
            x: String(Math.min(dx, drag.current.x)),
            y: String(Math.min(dy, drag.current.y)),
            width: String(Math.max(1, Math.abs(dx - drag.current.x))),
            height: String(Math.max(1, Math.abs(dy - drag.current.y))),
          });
        }}
        onPointerUp={() => {
          drag.current = null;
        }}
        onPointerCancel={() => {
          drag.current = null;
        }}
        role="img"
        aria-label={`Approximate ${action} preview of first selected image`}
      >
        <defs>
          <linearGradient id="frame-gradient" x1="0" y1="0" x2="1" y2="1">
            <stop stopColor={background} />
            <stop offset="1" stopColor={secondary} />
          </linearGradient>
          <filter
            id="frame-shadow"
            x="-100%"
            y="-100%"
            width="300%"
            height="300%"
          >
            <feDropShadow
              dx="0"
              dy={previewNumber(params.shadow, 64) / 2}
              stdDeviation={previewNumber(params.shadow, 64) / 2}
              floodOpacity="0.33"
            />
          </filter>
          <clipPath id="image-corners">
            <rect
              width={width}
              height={height}
              rx={
                action === "frame"
                  ? previewNumber(params.corners, Math.min(width, height) / 2)
                  : 0
              }
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
            x={-ox}
            y={-oy}
            width={fw}
            height={fh}
            fill={
              params.style === "gradient" ? "url(#frame-gradient)" : background
            }
          />
        )}
        <image
          href={url}
          width={width}
          height={height}
          clipPath="url(#image-corners)"
          opacity={action === "crop" ? 0.25 : 1}
          filter={
            action === "frame" && Number(params.shadow) > 0
              ? "url(#frame-shadow)"
              : undefined
          }
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
            ? `${resized[0]} x ${resized[1]} px - no upscaling`
            : action === "frame"
              ? `${fw} x ${fh} px framed output`
              : `${width} x ${height} px original`}
      </figcaption>
    </figure>
  );
}
