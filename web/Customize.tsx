import { useEffect, useMemo, useState } from "react";
import { native, type Session, type ShortcutLibrary } from "./native";
import { libraryActions } from "./action-search";
import { SIZE, wedgePath } from "./geometry";
import { appItems, defaultLayout, kindPool, type MenuItem } from "./menus";
import { Icon } from "./Icon";
import {
  TRIGGER_KEYS,
  savePrefs,
  setLayout,
  usePrefs,
  type Prefs,
} from "./prefs";

const KINDS = [
  { key: "images", label: "Images", hint: "PNG, JPG, WebP, HEIC…" },
  { key: "files", label: "Files & folders", hint: "Any file, ZIP, folder" },
  { key: "text", label: "Selected text", hint: "Text in any app" },
  { key: "window", label: "Windows", hint: "Apps without a selection" },
];
/** Slices the ring shows before the rest move into "More". */
const RING_SLOTS = 8;

/** Pushes the saved hold key to the native hook; returns an error message or "". */
export async function applyTrigger(code: string): Promise<string> {
  try {
    await native.setTrigger(TRIGGER_KEYS[code].vk);
    return "";
  } catch (e) {
    return String(e);
  }
}

function TriggerKey({ prefs }: { prefs: Prefs }) {
  const [listening, setListening] = useState(false);
  const [note, setNote] = useState("");
  useEffect(() => {
    if (!listening) return;
    async function key(event: KeyboardEvent) {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") return setListening(false);
      if (!(event.code in TRIGGER_KEYS)) {
        setNote(
          "That key can’t open the ring. Try `, Caps Lock, Pause or F1–F12.",
        );
        return;
      }
      const error = await applyTrigger(event.code);
      if (error) return setNote(error);
      savePrefs({ ...prefs, trigger: event.code });
      setNote("");
      setListening(false);
    }
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [listening, prefs]);
  const label = TRIGGER_KEYS[prefs.trigger]?.label ?? "`";
  return (
    <section className="cz-card" aria-labelledby="cz-key">
      <div>
        <h2 id="cz-key">Hold key</h2>
        <p>
          Hold this key anywhere in Windows to open the ring. A quick tap still
          types it.
        </p>
      </div>
      <div className="cz-key-box">
        <kbd className={listening ? "listening" : ""}>
          {listening ? "Press a key…" : label}
        </kbd>
        <button onClick={() => setListening(!listening)}>
          {listening ? "Cancel" : "Change key"}
        </button>
        {prefs.trigger !== "Backquote" && !listening && (
          <button
            className="text-button"
            onClick={async () => {
              setNote(await applyTrigger("Backquote"));
              savePrefs({ ...prefs, trigger: "Backquote" });
            }}
          >
            Use `
          </button>
        )}
      </div>
      {note && (
        <p className="cz-note" role="alert">
          {note}
        </p>
      )}
    </section>
  );
}

function LayoutPreview({ items }: { items: MenuItem[] }) {
  const shown = items.slice(0, RING_SLOTS);
  const count = Math.max(shown.length, 2);
  return (
    <svg
      className="cz-ring"
      viewBox={`0 0 ${SIZE} ${SIZE}`}
      role="img"
      aria-label={`Ring preview with ${shown.length} slices`}
    >
      {shown.length === 0 && (
        <text x={SIZE / 2} y={SIZE / 2} textAnchor="middle">
          Empty
        </text>
      )}
      {shown.map((item, index) => (
        <g key={item.key}>
          <path d={wedgePath(index, count)} />
          <title>{item.label}</title>
        </g>
      ))}
      <circle cx={SIZE / 2} cy={SIZE / 2} r="82" className="cz-hub" />
      <text x={SIZE / 2} y={SIZE / 2 + 12} textAnchor="middle" className="cz-n">
        {shown.length}
      </text>
    </svg>
  );
}

function Editor({
  context,
  title,
  pool,
  fallback,
}: {
  context: string;
  title: string;
  pool: MenuItem[];
  fallback: string[];
}) {
  const prefs = usePrefs();
  const [query, setQuery] = useState("");
  const saved = prefs.layouts[context];
  const keys = saved ?? fallback;
  const byKey = useMemo(() => new Map(pool.map((i) => [i.key, i])), [pool]);
  const items = keys.flatMap((key) => byKey.get(key) ?? []);
  const live = items.map((i) => i.key);
  const matches = useMemo(() => {
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    return pool
      .filter(
        (i) =>
          !live.includes(i.key) &&
          words.every((w) =>
            `${i.label} ${i.hint ?? ""}`.toLowerCase().includes(w),
          ),
      )
      .slice(0, 40);
  }, [pool, query, live.join("|")]);
  function move(index: number, by: -1 | 1) {
    const next = [...live];
    [next[index], next[index + by]] = [next[index + by], next[index]];
    setLayout(context, next);
  }
  return (
    <div className="cz-editor">
      <header>
        <div>
          <h2>{title}</h2>
          <p>
            {saved ? "Customized" : "Default layout"} · {items.length}{" "}
            {items.length === 1 ? "slice" : "slices"}
            {items.length > RING_SLOTS &&
              ` · ${items.length - RING_SLOTS + 1} move into More`}
          </p>
        </div>
        <button
          disabled={!saved}
          onClick={() => setLayout(context, null)}
          title="Return to the built-in layout"
        >
          <Icon name="rotate-left" size={16} />
          Reset
        </button>
      </header>
      <div className="cz-split">
        <LayoutPreview items={items} />
        <ol className="cz-slots" aria-label={`${title} ring order`}>
          {items.map((item, index) => (
            <li key={item.key}>
              <span className="cz-num">{index < 9 ? index + 1 : ""}</span>
              <span className="cz-icon">
                <Icon name={item.icon} size={18} />
              </span>
              <span className="cz-label">
                <strong>{item.label}</strong>
                <small>
                  {item.children
                    ? `${item.children.length} choices`
                    : (item.hint ?? "")}
                </small>
              </span>
              <button
                className="icon-button"
                aria-label={`Move ${item.label} earlier`}
                disabled={index === 0}
                onClick={() => move(index, -1)}
              >
                <Icon name="up" size={16} />
              </button>
              <button
                className="icon-button"
                aria-label={`Move ${item.label} later`}
                disabled={index === items.length - 1}
                onClick={() => move(index, 1)}
              >
                <Icon name="down" size={16} />
              </button>
              <button
                className="icon-button"
                aria-label={`Remove ${item.label}`}
                onClick={() =>
                  setLayout(
                    context,
                    live.filter((key) => key !== item.key),
                  )
                }
              >
                <Icon name="close" size={16} />
              </button>
            </li>
          ))}
          {!items.length && (
            <li className="cz-empty">
              Nothing here yet. Add actions below, or reset.
            </li>
          )}
        </ol>
      </div>
      <div className="cz-add">
        <label className="search">
          <Icon name="search" size={18} />
          <input
            aria-label={`Add an action to ${title}`}
            placeholder="Add an action to this ring…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </label>
        <ul aria-label="Available actions">
          {matches.map((item) => (
            <li key={item.key}>
              <button
                onClick={() => setLayout(context, [...live, item.key])}
                aria-label={`Add ${item.label}`}
              >
                <Icon name="plus" size={15} />
                <span>{item.label}</span>
                {item.hint && <kbd>{item.hint}</kbd>}
              </button>
            </li>
          ))}
          {!matches.length && (
            <li className="cz-empty">
              {query
                ? `Nothing matches “${query}”.`
                : "Everything is on the ring."}
            </li>
          )}
        </ul>
      </div>
    </div>
  );
}

export function Customize({ session }: { session: Session }) {
  const prefs = usePrefs();
  const [library, setLibrary] = useState<ShortcutLibrary | null>(null);
  const [error, setError] = useState("");
  const [context, setContext] = useState(
    session.kind === "window" && session.app
      ? `app:${session.app}`
      : session.kind === "empty"
        ? "images"
        : session.kind,
  );
  const [appQuery, setAppQuery] = useState("");
  useEffect(() => {
    native
      .shortcutLibrary()
      .then(setLibrary)
      .catch((e) => setError(String(e)));
  }, []);
  const profiles = useMemo(() => {
    const words = appQuery.toLowerCase().split(/\s+/).filter(Boolean);
    return (library?.profiles ?? [])
      .filter((p) => words.every((w) => p.name.toLowerCase().includes(w)))
      .sort(
        (a, b) =>
          Number(b.id === session.app) - Number(a.id === session.app) ||
          a.name.localeCompare(b.name),
      );
  }, [library, appQuery, session.app]);
  const isApp = context.startsWith("app:");
  const profile = isApp ? context.slice(4) : "";
  const appActions = useMemo(
    () => (library && isApp ? libraryActions(library, profile) : []),
    [library, isApp, profile],
  );
  const pool = useMemo(
    () => (isApp ? appItems(appActions) : kindPool(context, session.actions)),
    [isApp, appActions, context, session.actions],
  );
  const title = isApp
    ? (library?.profiles.find((p) => p.id === profile)?.name ?? profile)
    : (KINDS.find((k) => k.key === context)?.label ?? context);
  const edited = (key: string) => key in prefs.layouts;
  return (
    <section className="customize" aria-label="Customize PieAll">
      <div className="cz-intro">
        <h2>Make the ring yours.</h2>
        <p>
          Choose the hold key, then decide which actions each kind of file and
          each app puts on the ring. Changes apply the next time you open it.
        </p>
      </div>
      <TriggerKey prefs={prefs} />
      <div className="cz-layouts">
        <nav className="cz-contexts" aria-label="Ring layouts">
          <h3>File types</h3>
          {KINDS.map((kind) => (
            <button
              key={kind.key}
              className={context === kind.key ? "active" : ""}
              aria-current={context === kind.key ? "true" : undefined}
              onClick={() => setContext(kind.key)}
            >
              <Icon name={kind.key} size={18} />
              <span>
                {kind.label}
                <small>{kind.hint}</small>
              </span>
              {edited(kind.key) && <i title="Customized" />}
            </button>
          ))}
          <h3>Apps</h3>
          <label className="search">
            <Icon name="search" size={16} />
            <input
              aria-label="Find an app"
              placeholder="Find an app…"
              value={appQuery}
              onChange={(e) => setAppQuery(e.target.value)}
            />
          </label>
          <div className="cz-apps">
            {profiles.map((p) => (
              <button
                key={p.id}
                className={context === `app:${p.id}` ? "active" : ""}
                aria-current={context === `app:${p.id}` ? "true" : undefined}
                onClick={() => setContext(`app:${p.id}`)}
              >
                <Icon name="apps" size={18} />
                <span>
                  {p.name}
                  {p.id === session.app && <small>Current app</small>}
                </span>
                {edited(`app:${p.id}`) && <i title="Customized" />}
              </button>
            ))}
            {library && !profiles.length && (
              <p className="cz-empty">No app matches “{appQuery}”.</p>
            )}
            {!library && !error && <p className="cz-empty">Loading apps…</p>}
          </div>
        </nav>
        {isApp && !library ? (
          <p className="cz-empty" role="status">
            {error || "Loading shortcuts…"}
          </p>
        ) : (
          <Editor
            key={context}
            context={context}
            title={title}
            pool={pool}
            fallback={defaultLayout(context, appActions)}
          />
        )}
      </div>
    </section>
  );
}
