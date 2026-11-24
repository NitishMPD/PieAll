import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { native, type MediaItem } from "./native";
import { Icon } from "./Icon";
import { mediaKinds, mediaTitles } from "./media-titles";
import "./media-picker.css";

/* Thumbnails come only from the native helper, a few at a time, and only for
   tiles that scroll into view. Results are cached for the page's lifetime. */
const thumbCache = new Map<string, Promise<string>>();
const waiting: (() => void)[] = [];
let active = 0;
const maxActive = 4;

function queued<T>(task: () => Promise<T>) {
  return new Promise<T>((resolve, reject) => {
    const run = () => {
      active++;
      task()
        .then(resolve, reject)
        .finally(() => {
          active--;
          waiting.shift()?.();
        });
    };
    if (active < maxActive) run();
    else waiting.push(run);
  });
}

function thumbnail(source: string, referer: string) {
  if (source.startsWith("data:image/")) return Promise.resolve(source);
  const key = `${referer}\n${source}`;
  let request = thumbCache.get(key);
  if (!request) {
    if (thumbCache.size >= 24) {
      const oldest = thumbCache.keys().next().value;
      if (oldest !== undefined) thumbCache.delete(oldest);
    }
    request = queued(() => native.mediaThumbnail(source, referer));
    request.catch(() => {
      if (thumbCache.get(key) === request) thumbCache.delete(key);
    });
    thumbCache.set(key, request);
  }
  return request;
}

function previewSource(item: MediaItem) {
  return item.preview ?? (item.kind === "video" ? null : item.url);
}

function MediaThumb({ item, referer }: { item: MediaItem; referer: string }) {
  const frame = useRef<HTMLSpanElement>(null);
  const source = previewSource(item);
  const [src, setSrc] = useState<string | null>(null);
  const [state, setState] = useState<"waiting" | "ready" | "failed">(
    source ? "waiting" : "failed",
  );
  useEffect(() => {
    setSrc(null);
    setState(source ? "waiting" : "failed");
    const node = frame.current;
    if (!source || !node) return;
    let live = true;
    const load = () =>
      void thumbnail(source, referer).then(
        (url) => {
          if (!live) return;
          if (url.startsWith("data:image/")) setSrc(url);
          else setState("failed");
        },
        () => {
          if (live) setState("failed");
        },
      );
    if (typeof IntersectionObserver === "undefined") {
      load();
      return () => {
        live = false;
      };
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          observer.disconnect();
          load();
        }
      },
      { rootMargin: "160px 0px" },
    );
    observer.observe(node);
    return () => {
      live = false;
      observer.disconnect();
    };
  }, [source, referer]);
  const kindIcon = item.kind === "video" ? "video" : "images";
  return (
    <span className="media-thumb" data-state={state} ref={frame}>
      {src && state !== "failed" && (
        <img
          src={src}
          alt=""
          decoding="async"
          draggable={false}
          onLoad={() => setState("ready")}
          onError={() => setState("failed")}
        />
      )}
      {state !== "ready" && (
        <span className="media-thumb-fallback" aria-hidden="true">
          <Icon name={kindIcon} size={22} />
        </span>
      )}
      {item.kind !== "image" && (
        <span className="media-kind" aria-hidden="true">
          {item.kind === "video" && <Icon name="play" size={11} />}
          {item.kind === "video" ? "Video" : "GIF"}
        </span>
      )}
    </span>
  );
}

function GroupToggle({
  label,
  count,
  total,
  onChange,
}: {
  label: string;
  count: number;
  total: number;
  onChange: (checked: boolean) => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (input.current) input.current.indeterminate = count > 0 && count < total;
  }, [count, total]);
  return (
    <label className="media-filter">
      <input
        ref={input}
        type="checkbox"
        checked={count === total}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span>{label}</span>
      <span className="media-filter-count">
        {count}/{total}
      </span>
    </label>
  );
}

export interface MediaPickerProps {
  items: MediaItem[];
  /** The page the media came from; sent as the thumbnail referer. */
  referer: string;
  selected: string[];
  onSelectedChange: (selected: string[]) => void;
  names: Record<string, string>;
  onRename: (url: string, name: string) => void;
  onConfirm: () => void;
  onClose: () => void;
  closeLabel: string;
  pending?: boolean;
  error?: string;
  note?: ReactNode;
  titleId?: string;
}

/** Preview grid, type toggles and a fixed action footer. Layout only: the
    caller places it in a dialog or a window. */
export function MediaPicker({
  items,
  referer,
  selected,
  onSelectedChange,
  names,
  onRename,
  onConfirm,
  onClose,
  closeLabel,
  pending = false,
  error,
  note,
  titleId,
}: MediaPickerProps) {
  const fallbackId = useId();
  const headingId = titleId ?? fallbackId;
  const titles = useMemo(() => mediaTitles(items), [items]);
  const chosen = useMemo(() => new Set(selected), [selected]);
  const grid = useRef<HTMLUListElement>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const settled = useRef(false);
  const tileId = useId();
  let host = "";
  try {
    host = new URL(referer).hostname.replace(/^www\./, "");
  } catch {
    // Show no source when the referer is not a URL.
  }
  function setItems(subset: MediaItem[], checked: boolean) {
    const urls = new Set(subset.map((item) => item.url));
    const next = checked
      ? new Set([...chosen, ...urls])
      : new Set([...chosen].filter((url) => !urls.has(url)));
    // Keep scan order so downloads number files the way the post shows them.
    onSelectedChange(
      items.map((item) => item.url).filter((url) => next.has(url)),
    );
  }
  function moveFocus(event: KeyboardEvent<HTMLUListElement>) {
    const keys = [
      "ArrowLeft",
      "ArrowRight",
      "ArrowUp",
      "ArrowDown",
      "Home",
      "End",
    ];
    if (!keys.includes(event.key) || !grid.current) return;
    const boxes = Array.from(
      grid.current.querySelectorAll<HTMLInputElement>("input[type=checkbox]"),
    );
    const index = boxes.indexOf(event.target as HTMLInputElement);
    if (index < 0) return;
    const columns = getComputedStyle(grid.current)
      .gridTemplateColumns.split(" ")
      .filter(Boolean).length;
    const step = {
      ArrowLeft: -1,
      ArrowRight: 1,
      ArrowUp: -columns,
      ArrowDown: columns,
      Home: -index,
      End: boxes.length - 1 - index,
    }[event.key]!;
    const target = boxes[index + step];
    if (!target) return;
    event.preventDefault();
    target.focus();
    target.closest("li")?.scrollIntoView({ block: "nearest" });
  }
  const groups = mediaKinds
    .map((group) => ({
      ...group,
      items: items.filter((item) => item.kind === group.kind),
    }))
    .filter((group) => group.items.length);
  const count = selected.length;
  return (
    <div className="media-review">
      <header className="media-review-header">
        <div>
          <h2 id={headingId}>Choose media to download</h2>
          {host && <p className="media-review-source">From {host}</p>}
        </div>
        <button
          type="button"
          className="icon-button media-review-close"
          aria-label={closeLabel}
          title={closeLabel}
          onClick={onClose}
        >
          <Icon name="close" size={18} />
        </button>
      </header>
      <div
        className="media-filters"
        role="group"
        aria-label="Select media by type"
      >
        <GroupToggle
          label="All"
          count={count}
          total={items.length}
          onChange={(checked) => setItems(items, checked)}
        />
        {groups.length > 1 &&
          groups.map((group) => (
            <GroupToggle
              key={group.kind}
              label={group.label}
              count={group.items.filter((item) => chosen.has(item.url)).length}
              total={group.items.length}
              onChange={(checked) => setItems(group.items, checked)}
            />
          ))}
      </div>
      <div className="media-review-body">
        <ul
          className="media-grid"
          ref={grid}
          aria-labelledby={headingId}
          onKeyDown={moveFocus}
        >
          {items.map((item, index) => {
            const title = titles.get(item.url)!;
            const renamed = Boolean(names[item.url]);
            const displayName = names[item.url] || title;
            const on = chosen.has(item.url);
            const boxId = `${tileId}-${index}`;
            const editing = renaming === item.url;
            return (
              <li key={item.url} className="media-item" data-selected={on}>
                <label className="media-tile" htmlFor={boxId}>
                  <input
                    id={boxId}
                    type="checkbox"
                    checked={on}
                    onChange={(event) => setItems([item], event.target.checked)}
                  />
                  <MediaThumb item={item} referer={referer} />
                </label>
                <div className="media-name-row" data-editing={editing}>
                  {editing ? (
                    <input
                      autoFocus
                      className="media-name-input"
                      aria-label={`File name without extension for ${title}`}
                      placeholder={title}
                      maxLength={120}
                      value={draft}
                      onChange={(event) => setDraft(event.target.value)}
                      onFocus={(event) => event.currentTarget.select()}
                      onBlur={() => {
                        if (settled.current) return;
                        onRename(item.url, draft.trim());
                        setRenaming(null);
                      }}
                      onKeyDown={(event) => {
                        if (event.key !== "Escape" && event.key !== "Enter")
                          return;
                        event.preventDefault();
                        event.stopPropagation();
                        const tile = event.currentTarget.closest("li");
                        // Keep the unmount blur from applying a cancelled draft.
                        settled.current = true;
                        if (event.key === "Enter")
                          onRename(item.url, draft.trim());
                        setRenaming(null);
                        requestAnimationFrame(() =>
                          tile
                            ?.querySelector<HTMLButtonElement>(
                              ".media-rename-button",
                            )
                            ?.focus(),
                        );
                      }}
                    />
                  ) : (
                    <>
                      <label
                        className="media-title"
                        htmlFor={boxId}
                        title={displayName}
                        data-renamed={renamed}
                      >
                        {displayName}
                      </label>
                      <button
                        type="button"
                        className="media-rename-button"
                        aria-label={`Rename ${title}`}
                        title="Rename file"
                        disabled={pending}
                        onClick={() => {
                          settled.current = false;
                          setDraft(names[item.url] || "");
                          setRenaming(item.url);
                        }}
                      >
                        <Icon name="edit" size={14} />
                      </button>
                    </>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      </div>
      <footer className="media-review-footer">
        {error && (
          <p role="alert" className="error media-review-error">
            {error}
          </p>
        )}
        <div className="media-review-actions">
          <p className="media-review-count" role="status">
            <strong>{count}</strong> of {items.length} selected
            {note && <span className="media-review-note">{note}</span>}
          </p>
          <button type="button" onClick={onClose}>
            {closeLabel}
          </button>
          <button
            type="button"
            className="primary"
            disabled={!count || pending}
            onClick={onConfirm}
          >
            <Icon name="download" size={17} />
            {pending ? "Starting…" : `Download ${count}`}
          </button>
        </div>
      </footer>
    </div>
  );
}

/** The picker inside a modal <dialog>. Escape and the backdrop close it. */
export function MediaPickerDialog({
  open,
  ...props
}: MediaPickerProps & { open: boolean }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => {
    const node = dialog.current;
    if (!node) return;
    if (open && !node.open) {
      node.showModal();
      node.querySelector<HTMLButtonElement>(".primary")?.focus();
    } else if (!open && node.open) node.close();
  }, [open]);
  return (
    <dialog
      ref={dialog}
      className="media-dialog"
      aria-labelledby={titleId}
      onCancel={(event) => {
        event.preventDefault();
        props.onClose();
      }}
      onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        // Keep the dashboard's own Escape handling from also navigating away.
        event.preventDefault();
        event.stopPropagation();
        props.onClose();
      }}
      onClick={(event) => {
        if (event.target === event.currentTarget) props.onClose();
      }}
    >
      {open && <MediaPicker {...props} titleId={titleId} />}
    </dialog>
  );
}
