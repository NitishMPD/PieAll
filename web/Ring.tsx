import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import type { Session } from "./native";
import {
  INNER,
  OUTER,
  SIZE,
  arcPath,
  point,
  sectorAt,
  wedgePath,
} from "./geometry";
import { contextMenu, labelLines, resolve, type MenuItem } from "./menus";
import { native } from "./native";
import { Icon } from "./Icon";
import { usePrefs } from "./prefs";

/** Resting on a "More" slice (or the center, inside a submenu) this long moves level. */
export const DWELL_MS = 550;
/** Matches the hub: inside it nothing is selected. */
const DEAD_ZONE = 82;

function useListener(enabled: boolean, subscribe: () => Promise<() => void>) {
  useEffect(() => {
    if (!enabled) return;
    let active = true;
    let remove: (() => void) | undefined;
    subscribe()
      .then((unlisten) => {
        if (active) remove = unlisten;
        else unlisten();
      })
      .catch(() => {});
    return () => {
      active = false;
      remove?.();
    };
    // Subscribe once per surface; handlers read the latest state through refs.
  }, [enabled]);
}

function targetName(session: Session): string {
  if (session.app && session.kind === "text") return session.target;
  if (session.kind === "text") return "Selected text";
  if (session.kind === "window") return session.target;
  if (session.names.length > 1) return `${session.names.length} items`;
  return session.names[0] ?? "Nothing selected";
}

type Phase = "closed" | "open" | "closing";

export function Ring({
  session,
  thumbnail,
  standalone = false,
  onRun,
  onBrowse,
  onClose,
}: {
  session: Session;
  thumbnail: string | null;
  standalone?: boolean;
  onRun: (action: string, params: Record<string, string>) => void;
  onClose: () => void;
  onBrowse: () => void;
}) {
  const [path, setPath] = useState<string[]>([]);
  const [selected, setSelected] = useState<number | null>(
    standalone ? null : 0,
  );
  const [centered, setCentered] = useState(false);
  const [held, setHeld] = useState(false);
  const [phase, setPhase] = useState<Phase>(standalone ? "closed" : "open");
  const pending = useRef<[number, number] | null>(null);
  const container = useRef<HTMLDivElement>(null);
  const svg = useRef<SVGSVGElement>(null);
  const refs = useRef<(SVGGElement | null)[]>([]);

  const reading = session.busy === "capture";
  const blocked = !!session.busy || !!session.editor;
  const prefs = usePrefs();
  const root = useMemo(
    () => resolve(contextMenu(session, prefs), session.actions, session.names),
    [session, prefs],
  );
  let items = root;
  let parent: MenuItem | null = null;
  for (const key of path) {
    parent = items.find((i) => i.key === key) ?? null;
    items = parent?.children ?? items;
  }
  const current = selected !== null ? items[selected] : undefined;

  function enter(item: MenuItem) {
    setPath((p) => [...p, item.key]);
    setSelected(null);
    setCentered(false);
  }
  function back() {
    setPath((p) => p.slice(0, -1));
    setSelected(null);
    setCentered(false);
  }
  function activate(item: MenuItem) {
    if (blocked) return;
    if (item.browse) onBrowse();
    else if (item.children) enter(item);
    else if (item.action) onRun(item.action, item.params ?? {});
  }
  // Releasing the held key always ends the ring: it runs the highlighted action
  // slice, otherwise it closes. The highlight wins over the reported point, which
  // only decides when nothing is highlighted (a flick past the rim). Submenus open
  // by resting, not by releasing. A release during capture waits for the selection.
  function release(x: number, y: number) {
    if (reading) {
      pending.current = [x, y];
      return;
    }
    const index = selected ?? sectorAt(x, y, items.length, DEAD_ZONE);
    const item = index !== null ? items[index] : undefined;
    if (item?.browse && !blocked) onBrowse();
    else if (item?.action && !blocked) onRun(item.action, item.params ?? {});
    else onClose();
  }
  const latest = useRef(release);
  latest.current = release;

  useEffect(() => {
    setPath([]);
    setSelected(standalone ? null : 0);
  }, [session.revision, standalone]);
  useListener(standalone, () =>
    native.onRing((isHeld) => {
      setHeld(isHeld);
      setPath([]);
      setSelected(null);
      setCentered(false);
      pending.current = null;
      // Reset without transition, then fade in on the next frame.
      setPhase("closed");
      requestAnimationFrame(() => {
        setPhase("open");
        container.current?.focus();
      });
    }),
  );
  useListener(standalone, () => native.onClosing(() => setPhase("closing")));
  useListener(standalone, () =>
    native.onRelease((x, y) => {
      setHeld(false);
      latest.current(x, y);
    }),
  );
  useEffect(() => {
    if (reading || !pending.current) return;
    const [x, y] = pending.current;
    pending.current = null;
    latest.current(x, y);
  }, [reading, session.revision]);

  // Hover-to-open: rest on a submenu slice, or on the center to go back.
  const dwellTarget = current?.children
    ? `in:${current.key}`
    : centered && path.length
      ? "back"
      : null;
  useEffect(() => {
    if (!dwellTarget || blocked) return;
    const timer = setTimeout(() => {
      if (dwellTarget === "back") back();
      else if (current) enter(current);
    }, DWELL_MS);
    return () => clearTimeout(timer);
    // The target key captures everything the timer depends on.
  }, [dwellTarget, path.join("/"), blocked]);

  function track(clientX: number, clientY: number) {
    const box = svg.current?.getBoundingClientRect();
    if (!box || !items.length || reading) return;
    const x = ((clientX - box.left) * SIZE) / box.width;
    const y = ((clientY - box.top) * SIZE) / box.height;
    setCentered(Math.hypot(x - SIZE / 2, y - SIZE / 2) < DEAD_ZONE);
    setSelected(sectorAt(x, y, items.length, DEAD_ZONE));
  }

  const placeholder = items.length === 0;
  const count = placeholder ? 4 : Math.max(items.length, 2);
  const shapes = useMemo(
    () => Array.from({ length: count }, (_, index) => wedgePath(index, count)),
    [count],
  );
  const caption = reading
    ? "Reading…"
    : placeholder
      ? "Nothing to do here"
      : current?.children
        ? "Rest to open"
        : (current?.hint ??
          (parent ? "Center to go back" : standalone && !held ? "Esc" : ""));

  return (
    <div
      ref={container}
      tabIndex={-1}
      data-phase={phase}
      className={`radial ${standalone ? "radial-standalone" : ""}`}
      onPointerMove={(e) => track(e.clientX, e.clientY)}
      onKeyDown={(event) => {
        if (event.key === "/" && !blocked) {
          event.preventDefault();
          onBrowse();
          return;
        }
        if (
          event.key === "Escape" ||
          (event.key === "Backspace" && path.length)
        ) {
          event.preventDefault();
          event.stopPropagation();
          if (path.length) back();
          else if (standalone) onClose();
          return;
        }
        if (!items.length) return;
        const at = selected ?? -1;
        let next: number;
        if (
          ["ArrowRight", "ArrowDown"].includes(event.key) ||
          (standalone && event.key === "Tab" && !event.shiftKey)
        )
          next = at + 1;
        else if (
          ["ArrowLeft", "ArrowUp"].includes(event.key) ||
          (standalone && event.key === "Tab")
        )
          next = at - 1;
        else if (/^[1-8]$/.test(event.key)) {
          const item = items[Number(event.key) - 1];
          if (item) activate(item);
          return;
        } else if ((event.key === "Enter" || event.key === " ") && current) {
          event.preventDefault();
          activate(current);
          return;
        } else return;
        event.preventDefault();
        next = (next + items.length) % items.length;
        setSelected(next);
        refs.current[next]?.focus();
      }}
    >
      <div className="ring-body">
        <svg
          ref={svg}
          viewBox={`0 0 ${SIZE} ${SIZE}`}
          className="radial-svg"
          role="menu"
          aria-label={
            parent ? parent.label : `Actions for ${targetName(session)}`
          }
          aria-busy={reading}
        >
          <defs>
            <linearGradient
              id="glass-edge"
              gradientUnits="userSpaceOnUse"
              x1="0"
              y1="20"
              x2="0"
              y2="420"
            >
              <stop offset="0" stopColor="#fff" stopOpacity="0.55" />
              <stop offset="0.5" stopColor="#fff" stopOpacity="0.14" />
              <stop offset="1" stopColor="#fff" stopOpacity="0.28" />
            </linearGradient>
            <linearGradient
              id="glass-sheen"
              gradientUnits="userSpaceOnUse"
              x1="0"
              y1="20"
              x2="0"
              y2="420"
            >
              <stop offset="0" stopColor="#fff" stopOpacity="0.13" />
              <stop offset="0.55" stopColor="#fff" stopOpacity="0.02" />
              <stop offset="1" stopColor="#fff" stopOpacity="0.06" />
            </linearGradient>
          </defs>
          <circle className="ring-rim" cx="220" cy="220" r={OUTER + 10} />
          <circle className="ring-well" cx="220" cy="220" r={INNER - 7} />
          {/* Slices appear only once the selection is read: the previous target's
              slices would show first and then reshape, which read as a jump. */}
          {!reading && (
            <g className="sectors" key={path.join("/")}>
              {Array.from({ length: count }, (_, index) => {
                const item = items[index];
                const angle = (index * 360) / count;
                const [x, y] = point((OUTER + INNER) / 2, angle);
                const [px, py] = point(5, angle);
                const active = !!item && selected === index;
                return (
                  <g
                    key={item?.key ?? `empty-${index}`}
                    className="sector-shell"
                  >
                    <g
                      className={`sector ${active ? "is-selected" : ""} ${item ? "" : "is-placeholder"}`}
                      style={
                        {
                          "--pop-x": `${px - SIZE / 2}px`,
                          "--pop-y": `${py - SIZE / 2}px`,
                        } as CSSProperties
                      }
                      role={item ? "menuitem" : undefined}
                      aria-label={
                        item
                          ? [item.label, item.hint].filter(Boolean).join(", ")
                          : undefined
                      }
                      aria-haspopup={item?.children ? "menu" : undefined}
                      aria-disabled={blocked || undefined}
                      tabIndex={
                        item
                          ? active || (selected === null && index === 0)
                            ? 0
                            : -1
                          : undefined
                      }
                      ref={(el) => {
                        refs.current[index] = el;
                      }}
                      onFocus={() => item && setSelected(index)}
                      onClick={() => item && activate(item)}
                    >
                      {item && (
                        <title>
                          {`${item.label}${item.hint ? ` · ${item.hint}` : ""}`}
                        </title>
                      )}
                      <path className="sector-shape" d={shapes[index]} />
                      <path className="sector-sheen" d={shapes[index]} />
                      {active && item?.children && !blocked && (
                        <g key={`dwell-${item.key}`} className="dwell">
                          <path
                            className="dwell-track"
                            d={arcPath(index, count)}
                          />
                          <path
                            className="dwell-fill"
                            d={arcPath(index, count)}
                            pathLength={1}
                            style={
                              { "--dwell": `${DWELL_MS}ms` } as CSSProperties
                            }
                          />
                        </g>
                      )}
                      {item && (
                        <g
                          transform={`translate(${x},${y})`}
                          className="sector-content"
                        >
                          <foreignObject x="-14" y="-29" width="28" height="28">
                            <span className="wedge-icon">
                              <Icon name={item.icon} size={22} />
                            </span>
                          </foreignObject>
                          <text textAnchor="middle">
                            {labelLines(item.label).map(
                              (line, lineIndex, lines) => (
                                <tspan
                                  key={lineIndex}
                                  x="0"
                                  y={
                                    (lines.length === 1 ? 15 : 9) +
                                    lineIndex * 14
                                  }
                                >
                                  {line}
                                </tspan>
                              ),
                            )}
                          </text>
                        </g>
                      )}
                    </g>
                  </g>
                );
              })}
            </g>
          )}
        </svg>
        <button
          className={`ring-center ${centered && path.length ? "is-dwelling" : ""}`}
          tabIndex={standalone ? -1 : undefined}
          onClick={() => (path.length ? back() : onClose())}
          aria-label={
            path.length
              ? "Back"
              : standalone
                ? "Close action ring"
                : "Open files"
          }
        >
          {reading ? (
            <span className="spinner" />
          ) : parent ? (
            <Icon name="back" size={22} />
          ) : thumbnail ? (
            <img src={thumbnail} alt="" />
          ) : (
            <Icon
              name={session.kind === "empty" ? "ring" : session.kind}
              size={24}
            />
          )}
          <strong>{parent ? parent.label : targetName(session)}</strong>
          <span>{caption}</span>
        </button>
      </div>
      <p className="sr-only" aria-live="polite">
        {reading ? "Reading your selection." : ""}
      </p>
    </div>
  );
}
