import { useCallback, useEffect, useRef, useState } from "react";
import { native, type DownloadPreferences, type Session } from "./native";
import { MediaPicker } from "./MediaPicker";

function preferenceNote(preferences: DownloadPreferences | undefined) {
  if (!preferences) return null;
  const quality =
    preferences.quality === "best"
      ? "highest quality"
      : `up to ${preferences.quality}p`;
  return `Videos save at ${quality} as ${preferences.container.toUpperCase()}.`;
}

/** The compact window (?surface=media-review) that the browser ring's
    Download media opens when a page has several items. */
export function MediaReviewWindow() {
  const [review, setReview] = useState<Session["media_review"]>(null);
  const [loaded, setLoaded] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [names, setNames] = useState<Record<string, string>>({});
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const shownId = useRef<number | null>(null);
  const scanSequence = useRef(0);
  const refresh = useCallback(async () => {
    const sequence = ++scanSequence.current;
    const session = await native.session();
    if (sequence !== scanSequence.current) return;
    const next = session.media_review ?? null;
    setReview(next);
    setLoaded(true);
    if (next && next.id !== shownId.current) {
      shownId.current = next.id;
      setSelected(next.items.map((item) => item.url));
      setNames({});
      setPending(false);
      setError("");
    }
  }, []);
  useEffect(() => {
    let live = true;
    let remove: (() => void) | undefined;
    native
      .onChange(() => {
        if (live) void refresh().catch((e) => setError(String(e)));
      })
      .then((unlisten) => {
        if (!live) return unlisten();
        remove = unlisten;
        return refresh();
      })
      .catch((e) => setError(String(e)));
    void refresh().catch((e) => {
      setLoaded(true);
      setError(String(e));
    });
    return () => {
      live = false;
      scanSequence.current++;
      remove?.();
    };
  }, [refresh]);
  const dismiss = useCallback(() => {
    if (!review) return;
    void native.dismissMediaReview(review.id).catch((e) => setError(String(e)));
  }, [review]);
  useEffect(() => {
    function key(event: KeyboardEvent) {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      event.preventDefault();
      dismiss();
    }
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [dismiss]);
  const reviewId = review?.id;
  useEffect(() => {
    if (reviewId !== undefined)
      document
        .querySelector<HTMLButtonElement>(".media-review-window .primary")
        ?.focus();
  }, [reviewId]);
  async function confirm() {
    if (!review) return;
    setPending(true);
    setError("");
    try {
      await native.confirmMediaReview(
        review.id,
        selected.map((url) => ({ url, name: names[url] || null })),
      );
    } catch (e) {
      setError(String(e));
      setPending(false);
    }
  }
  return (
    <main className="media-review-window">
      {review ? (
        <MediaPicker
          key={review.id}
          items={review.items}
          referer={review.url}
          selected={selected}
          onSelectedChange={setSelected}
          names={names}
          onRename={(url, name) =>
            setNames((current) => ({ ...current, [url]: name }))
          }
          onConfirm={() => void confirm()}
          onClose={dismiss}
          closeLabel="Cancel"
          pending={pending}
          error={error}
          note={preferenceNote(review.preferences)}
        />
      ) : (
        <p className="media-review-empty" role="status">
          {error ||
            (loaded ? "No media is waiting for review." : "Loading media…")}
        </p>
      )}
    </main>
  );
}
