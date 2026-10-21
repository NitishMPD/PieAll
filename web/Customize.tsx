import { useEffect, useMemo, useState } from "react";
import { native, type Session, type ShortcutLibrary } from "./native";
import { libraryActions } from "./action-search";
import { INNER, OUTER, SIZE, point, wedgePath } from "./geometry";
import {
  appPool,
  defaultLayout,
  kindPool,
  labelLines,
  paginate,
  type MenuItem,
} from "./menus";
import { Icon } from "./Icon";
import {
  TRIGGER_KEYS,
  savePrefs,
  setLayout,
  usePrefs,
  type Prefs,
} from "./prefs";

const KINDS = [
  { key: "images", label: "Images" },
  { key: "files", label: "Files & folders" },
  { key: "text", label: "Selected text" },
  { key: "window", label: "Windows" },
];
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

/** Monogram tile: a stable color per app, since Windows exposes no icon to the UI. */
export function AppIcon({ id, name }: { id: string; name: string }) {
  let hash = 0;
  for (const ch of id) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
  const words = name.replace(/[()]/g, "").split(/\s+/).filter(Boolean);
  const letters = (
    words.length > 1 ? words[0][0] + words[1][0] : name.slice(0, 2)
  ).toUpperCase();
  return (
    <span
      className="app-icon"
      style={{ background: `hsl(${hash % 360} 42% 40%)` }}
      aria-hidden="true"
    >
      {letters}
    </span>
  );
}

function HoldKey({ prefs }: { prefs: Prefs }) {
  const [listening, setListening] = useState(false);
  const [note, setNote] = useState("");
  useEffect(() => {
    if (!listening) return;
    async function key(event: KeyboardEvent) {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") return setListening(false);
      if (!(event.code in TRIGGER_KEYS))
        return setNote("Use `, Caps Lock, Pause, Scroll Lock or F1–F12.");
      const error = await applyTrigger(event.code);
      if (error) return setNote(error);
      savePrefs({ ...prefs, trigger: event.code });
      setNote("");
      setListening(false);
    }
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [listening, prefs]);
  return (
    <div className="cz-hold">
      <span>Hold key</span>
      <kbd className={listening ? "listening" : ""}>
        {listening
          ? "Press a key"
          : (TRIGGER_KEYS[prefs.trigger]?.label ?? "`")}
      </kbd>
      <button onClick={() => setListening(!listening)}>
        {listening ? "Cancel" : "Change"}
      </button>
      {note && (
        <p role="alert" className="cz-note">
          {note}
        </p>
      )}
    </div>
  );
}

/** The ring exactly as it will open, with a ghost slice to add into. */
function Pie({
  items,
  selected,
  onSelect,
  onAdd,
  title,
  badge,
}: {
  items: MenuItem[];
  selected: number | null;
  onSelect: (index: number) => void;
  onAdd: () => void;
  title: string;
  badge: React.ReactNode;
}) {
  const shown = paginate(items);
  const count = Math.max(shown.length + (items.length < RING_SLOTS ? 1 : 0), 2);
  return (
    <svg
      className="cz-pie"
      viewBox={`0 0 ${SIZE} ${SIZE}`}
      role="group"
      aria-label={`${title} ring, ${items.length} slices`}
    >
      <circle className="ring-rim" cx="220" cy="220" r={OUTER + 10} />
      <circle className="ring-well" cx="220" cy="220" r={INNER - 7} />
      {Array.from({ length: count }, (_, index) => {
        const item = shown[index];
        const ghost = !item && index === shown.length && items.length < 8;
        const [x, y] = point((OUTER + INNER) / 2, (index * 360) / count);
        const interactive = item || ghost;
        return (
          <g
            key={item?.key ?? `empty-${index}`}
            className={`cz-slice ${selected === index ? "is-selected" : ""} ${interactive ? "" : "is-placeholder"} ${ghost ? "is-ghost" : ""}`}
            role={interactive ? "button" : undefined}
            tabIndex={interactive ? 0 : undefined}
            aria-pressed={item ? selected === index : undefined}
            aria-label={item ? item.label : ghost ? "Add a slice" : undefined}
            onClick={() => (item ? onSelect(index) : ghost && onAdd())}
            onKeyDown={(event) => {
              if (event.key !== "Enter" && event.key !== " ") return;
              event.preventDefault();
              if (item) onSelect(index);
              else if (ghost) onAdd();
            }}
          >
            <path d={wedgePath(index, count)} />
            {interactive && (
              <g transform={`translate(${x},${y})`}>
                <foreignObject x="-14" y="-29" width="28" height="28">
                  <span className="wedge-icon">
                    <Icon name={item ? item.icon : "plus"} size={22} />
                  </span>
                </foreignObject>
                <text textAnchor="middle">
                  {(item ? labelLines(item.label) : ["Add"]).map(
                    (line, i, lines) => (
                      <tspan
                        key={i}
                        x="0"
                        y={(lines.length === 1 ? 15 : 9) + i * 14}
                      >
                        {line}
                      </tspan>
                    ),
                  )}
                </text>
              </g>
            )}
          </g>
        );
      })}
      <foreignObject
        x={SIZE / 2 - 80}
        y={SIZE / 2 - 80}
        width="160"
        height="160"
      >
        <div className="cz-hub">
          {badge}
          <strong>{title}</strong>
          <span>
            {items.length} {items.length === 1 ? "slice" : "slices"}
          </span>
        </div>
      </foreignObject>
    </svg>
  );
}

function Editor({
  context,
  title,
  badge,
  pool,
  fallback,
}: {
  context: string;
  title: string;
  badge: React.ReactNode;
  pool: MenuItem[];
  fallback: string[];
}) {
  const prefs = usePrefs();
  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<"slices" | "add">("slices");
  const [selected, setSelected] = useState<number | null>(null);
  const [dragging, setDragging] = useState<number | null>(null);
  const saved = prefs.layouts[context];
  const byKey = useMemo(() => new Map(pool.map((i) => [i.key, i])), [pool]);
  const items = (saved ?? fallback).flatMap((key) => byKey.get(key) ?? []);
  const live = items.map((i) => i.key);
  const matches = useMemo(() => {
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    return pool.filter(
      (i) =>
        !live.includes(i.key) &&
        words.every((w) =>
          `${i.label} ${i.hint ?? ""}`.toLowerCase().includes(w),
        ),
    );
  }, [pool, query, live.join("|")]);
  const pick = Math.min(selected ?? -1, items.length - 1);
  function reorder(from: number, to: number) {
    if (from === to || to < 0 || to >= live.length) return;
    const next = [...live];
    next.splice(to, 0, next.splice(from, 1)[0]);
    setLayout(context, next);
    setSelected(to);
  }
  function remove(index: number) {
    setLayout(
      context,
      live.filter((_, i) => i !== index),
    );
    setSelected(null);
  }
  return (
    <>
      <section className="cz-stage" aria-label={`${title} ring`}>
        <header>
          <h2>{title}</h2>
          <button
            disabled={!saved}
            onClick={() => {
              setLayout(context, null);
              setSelected(null);
            }}
          >
            <Icon name="rotate-left" size={16} />
            Reset to default
          </button>
        </header>
        <Pie
          items={items}
          selected={pick >= 0 ? pick : null}
          onSelect={(i) => {
            setSelected(i);
            setTab("slices");
          }}
          onAdd={() => setTab("add")}
          title={title}
          badge={badge}
        />
      </section>
      <aside className="cz-panel">
        <div className="cz-tabs" role="tablist">
          {(["slices", "add"] as const).map((name) => (
            <button
              key={name}
              role="tab"
              aria-selected={tab === name}
              className={tab === name ? "active" : ""}
              onClick={() => setTab(name)}
            >
              {name === "slices" ? `Slices · ${items.length}` : "Add action"}
            </button>
          ))}
        </div>
        {tab === "slices" ? (
          <ol className="cz-slots" aria-label={`${title} slice order`}>
            {items.map((item, index) => (
              <li
                key={item.key}
                draggable
                className={`${pick === index ? "is-selected" : ""} ${dragging === index ? "is-dragging" : ""}`}
                onClick={() => setSelected(index)}
                onDragStart={() => setDragging(index)}
                onDragEnd={() => setDragging(null)}
                onDragOver={(e) => e.preventDefault()}
                onDrop={() => dragging !== null && reorder(dragging, index)}
              >
                <span className="cz-num">{index + 1}</span>
                <span className="cz-icon">
                  <Icon name={item.icon} size={17} />
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
                  onClick={(e) => {
                    e.stopPropagation();
                    reorder(index, index - 1);
                  }}
                >
                  <Icon name="up" size={15} />
                </button>
                <button
                  className="icon-button"
                  aria-label={`Move ${item.label} later`}
                  disabled={index === items.length - 1}
                  onClick={(e) => {
                    e.stopPropagation();
                    reorder(index, index + 1);
                  }}
                >
                  <Icon name="down" size={15} />
                </button>
                <button
                  className="icon-button"
                  aria-label={`Remove ${item.label}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    remove(index);
                  }}
                >
                  <Icon name="close" size={15} />
                </button>
              </li>
            ))}
            {!items.length && (
              <li className="cz-empty">
                <button onClick={() => setTab("add")}>
                  <Icon name="plus" size={16} />
                  Add the first action
                </button>
              </li>
            )}
          </ol>
        ) : (
          <>
            <label className="search">
              <Icon name="search" size={17} />
              <input
                autoFocus
                aria-label={`Search actions for ${title}`}
                placeholder={`Search ${pool.length} actions`}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
            </label>
            <ul className="cz-pool" aria-label="Available actions">
              {matches.map((item) => (
                <li key={item.key}>
                  <button
                    onClick={() => {
                      setLayout(context, [...live, item.key]);
                      setSelected(live.length);
                    }}
                    aria-label={`Add ${item.label}`}
                  >
                    <span className="cz-icon">
                      <Icon name={item.icon} size={17} />
                    </span>
                    <span className="cz-label">
                      <strong>{item.label}</strong>
                      <small>{item.hint ?? ""}</small>
                    </span>
                    <Icon name="plus" size={16} />
                  </button>
                </li>
              ))}
              {!matches.length && (
                <li className="cz-empty">
                  {query ? `No match for “${query}”` : "All actions are added"}
                </li>
              )}
            </ul>
          </>
        )}
      </aside>
    </>
  );
}

export function Customize({ session }: { session: Session }) {
  const prefs = usePrefs();
  const [library, setLibrary] = useState<ShortcutLibrary | null>(null);
  const [error, setError] = useState("");
  const [context, setContext] = useState(
    session.app
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
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [library, appQuery]);
  const active = library?.profiles.find((p) => p.id === session.app);
  const isApp = context.startsWith("app:");
  const profile = isApp ? context.slice(4) : "";
  const appActions = useMemo(
    () => (library && isApp ? libraryActions(library, profile) : []),
    [library, isApp, profile],
  );
  const pool = useMemo(
    () => (isApp ? appPool(appActions) : kindPool(context, session.actions)),
    [isApp, appActions, context, session.actions],
  );
  const current = library?.profiles.find((p) => p.id === profile);
  const title = isApp
    ? (current?.name ?? profile)
    : (KINDS.find((k) => k.key === context)?.label ?? context);
  const edited = (key: string) => key in prefs.layouts;
  const row = (key: string, name: string, icon: React.ReactNode, tag = "") => (
    <button
      key={key}
      className={context === key ? "active" : ""}
      aria-current={context === key ? "true" : undefined}
      onClick={() => setContext(key)}
    >
      {icon}
      <span>{name}</span>
      {tag && <em>{tag}</em>}
      {edited(key) && <i title="Customized" />}
    </button>
  );
  return (
    <section className="cz" aria-label="Customize ring">
      <nav className="cz-rail" aria-label="Rings">
        <HoldKey prefs={prefs} />
        {active && (
          <div className="cz-group">
            <h3>Active app</h3>
            {row(
              `app:${active.id}`,
              active.name,
              <AppIcon id={active.id} name={active.name} />,
            )}
          </div>
        )}
        <div className="cz-group">
          <h3>File types</h3>
          {KINDS.map((k) =>
            row(
              k.key,
              k.label,
              <span className="cz-kind">
                <Icon name={k.key} size={17} />
              </span>,
            ),
          )}
        </div>
        <div className="cz-group cz-apps">
          <h3>Apps</h3>
          <label className="search">
            <Icon name="search" size={16} />
            <input
              aria-label="Find an app"
              placeholder="Find an app"
              value={appQuery}
              onChange={(e) => setAppQuery(e.target.value)}
            />
          </label>
          <div className="cz-app-list">
            {profiles.map((p) =>
              row(`app:${p.id}`, p.name, <AppIcon id={p.id} name={p.name} />),
            )}
            {library && !profiles.length && (
              <p className="cz-hint">No app matches</p>
            )}
            {!library && <p className="cz-hint">{error || "Loading apps…"}</p>}
          </div>
        </div>
      </nav>
      {isApp && !library ? (
        <p className="cz-hint cz-wait" role="status">
          {error || "Loading shortcuts…"}
        </p>
      ) : (
        <Editor
          key={context}
          context={context}
          title={title}
          badge={
            isApp ? (
              <AppIcon id={profile} name={title} />
            ) : (
              <span className="cz-kind">
                <Icon name={context} size={20} />
              </span>
            )
          }
          pool={pool}
          fallback={defaultLayout(context, appActions)}
        />
      )}
    </section>
  );
}
