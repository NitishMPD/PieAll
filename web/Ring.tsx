import { useEffect, useRef, useState } from "react";
import type { Session } from "./native";
import { SIZE, point, sectorAt, wedgePath } from "./geometry";
import { menus, resolve, type MenuItem } from "./menus";
import { native } from "./native";
import { Icon } from "./Icon";

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
  if (session.kind === "text") return "Selected text";
  if (session.kind === "window") return "This window";
  if (session.names.length > 1) return `${session.names.length} items`;
  return session.names[0] ?? "Nothing selected";
}

export function Ring({
  session,
  thumbnail,
  standalone = false,
  onRun,
  onClose,
}: {
  session: Session;
  thumbnail: string | null;
  standalone?: boolean;
  onRun: (action: string, params: Record<string, string>) => void;
  onClose: () => void;
}) {
  const [path, setPath] = useState<string[]>([]);
  const [selected, setSelected] = useState<number | null>(
    standalone ? null : 0,
  );
  const [held, setHeld] = useState(false);
  const pending = useRef<[number, number] | null>(null);
  const container = useRef<HTMLDivElement>(null);
  const svg = useRef<SVGSVGElement>(null);
  const refs = useRef<(SVGGElement | null)[]>([]);

  const reading = session.busy === "capture";
  const blocked = !!session.busy || !!session.editor;
  const root = resolve(menus[session.kind] ?? [], session.actions);
  let items = root;
  let parent: MenuItem | null = null;
  for (const key of path) {
    parent = items.find((i) => i.key === key) ?? null;
    items = parent?.children ?? items;
  }

  function activate(item: MenuItem) {
    if (blocked) return;
    if (item.children) {
      setPath((p) => [...p, item.key]);
      setSelected(0);
      requestAnimationFrame(() => refs.current[0]?.focus());
    } else if (item.action) onRun(item.action, item.params ?? {});
  }
  function back() {
    setPath((p) => p.slice(0, -1));
    setSelected(0);
  }
  // Releasing the held key picks by direction; releasing in the center keeps the
  // ring open for a click. A release during capture waits for the selection.
  function release(x: number, y: number) {
    if (reading) {
      pending.current = [x, y];
      return;
    }
    const index = sectorAt(x, y, items.length);
    if (index !== null && items[index]) activate(items[index]);
    else container.current?.focus();
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
      pending.current = null;
      container.current?.focus();
    }),
  );
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

  function track(clientX: number, clientY: number) {
    const box = svg.current?.getBoundingClientRect();
    if (!box || !items.length) return;
    setSelected(
      sectorAt(
        ((clientX - box.left) * SIZE) / box.width,
        ((clientY - box.top) * SIZE) / box.height,
        items.length,
      ),
    );
  }

  const placeholder = items.length === 0;
  const count = placeholder ? 4 : Math.max(items.length, 2);
  const hint = reading
    ? "Reading your selection"
    : held
      ? "Release on an action"
      : placeholder
        ? "Select a file, text or window first"
        : standalone
          ? "Click an action · Esc closes"
          : "Choose an action";

  return (
    <div
      ref={container}
      tabIndex={-1}
      className={`radial ${standalone ? "radial-standalone" : ""} ${held ? "is-held" : ""}`}
      onPointerMove={(e) => track(e.clientX, e.clientY)}
      onKeyDown={(event) => {
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
        const current = selected ?? -1;
        let next: number;
        if (
          ["ArrowRight", "ArrowDown"].includes(event.key) ||
          (standalone && event.key === "Tab" && !event.shiftKey)
        )
          next = current + 1;
        else if (
          ["ArrowLeft", "ArrowUp"].includes(event.key) ||
          (standalone && event.key === "Tab")
        )
          next = current - 1;
        else if (/^[1-8]$/.test(event.key)) {
          const item = items[Number(event.key) - 1];
          if (item) activate(item);
          return;
        } else if (
          (event.key === "Enter" || event.key === " ") &&
          selected !== null &&
          items[selected]
        ) {
          event.preventDefault();
          activate(items[selected]);
          return;
        } else return;
        event.preventDefault();
        next = (next + items.length) % items.length;
        setSelected(next);
        refs.current[next]?.focus();
      }}
    >
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
        <circle className="ring-rim" cx="220" cy="220" r="202" />
        {Array.from({ length: count }, (_, index) => {
          const item = items[index];
          const [x, y] = point(139, (index * 360) / count);
          const active = !!item && selected === index;
          return (
            <g
              key={item?.key ?? `empty-${index}`}
              className={`sector ${active ? "is-selected" : ""} ${item ? "" : "is-placeholder"}`}
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
              <path className="sector-shape" d={wedgePath(index, count)} />
              {item && (
                <g
                  transform={`translate(${x},${y})`}
                  className="sector-content"
                >
                  <foreignObject x="-15" y="-30" width="30" height="30">
                    <span className="wedge-icon">
                      <Icon name={item.icon} size={24} />
                    </span>
                  </foreignObject>
                  <text y="16" textAnchor="middle">
                    {item.label}
                  </text>
                  {active && (item.hint || item.children) && (
                    <text className="sector-hint" y="31" textAnchor="middle">
                      {item.children ? "Opens more" : item.hint}
                    </text>
                  )}
                </g>
              )}
            </g>
          );
        })}
      </svg>
      <button
        className="ring-center"
        tabIndex={standalone ? -1 : undefined}
        onClick={() => (path.length ? back() : onClose())}
        aria-label={
          path.length ? "Back" : standalone ? "Close action ring" : "Open files"
        }
      >
        {reading ? (
          <span className="spinner" />
        ) : parent ? (
          <Icon name="back" size={24} />
        ) : thumbnail ? (
          <img src={thumbnail} alt="" />
        ) : (
          <Icon
            name={session.kind === "empty" ? "ring" : session.kind}
            size={28}
          />
        )}
        <strong>{parent ? parent.label : targetName(session)}</strong>
        <span>{parent ? "Back" : hint}</span>
      </button>
      <p className="sr-only" aria-live="polite">
        {reading ? "Reading your selection." : ""}
      </p>
    </div>
  );
}
