import { useEffect, useRef, useState } from "react";
import { native, type Toast as Notice } from "./native";
import { Icon } from "./Icon";

function useEvent(subscribe: () => Promise<() => void>) {
  useEffect(() => {
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
  }, []);
}

/** A quiet result pill where the ring was: what was saved, copied or refused. */
export function Toast() {
  const [notice, setNotice] = useState<Notice | null>(null);
  const [leaving, setLeaving] = useState(false);
  const [serial, setSerial] = useState(0);
  const [error, setError] = useState("");
  const identity = useRef("");
  useEvent(() =>
    native.onToast((next) => {
      const key = next.job ? `job-${next.job.id}` : `notice-${next.id}`;
      if (identity.current !== key) setSerial((n) => n + 1);
      identity.current = key;
      setNotice(next);
      setError("");
      setLeaving(false);
    }),
  );
  useEvent(() => native.onToastHide(() => setLeaving(true)));
  if (!notice) return null;
  return (
    <main className="toast-stage">
      <div
        key={serial}
        className={`toast toast-${notice.tone} ${leaving ? "is-leaving" : ""}`}
        role={notice.tone === "error" ? "alert" : "status"}
      >
        <span className="toast-mark">
          {notice.tone === "busy" ? (
            <span className="spinner" />
          ) : (
            <Icon
              name={
                notice.tone === "error"
                  ? "alert"
                  : notice.tone === "ok"
                    ? "check"
                    : "info"
              }
              size={15}
            />
          )}
        </span>
        <strong>{notice.title}</strong>
        <button
          className="toast-close"
          aria-label="Close notification"
          title="Close notification (job continues)"
          onClick={() =>
            void native
              .dismissToast(notice.id, notice.job?.id)
              .catch((e) => setError(String(e)))
          }
        >
          <Icon name="close" size={14} />
        </button>
        {notice.detail && <span className="toast-detail">{notice.detail}</span>}
        {notice.job && (
          <div className="toast-job">
            <progress
              aria-label="Job progress"
              max={notice.job.total || 1}
              value={notice.job.completed + (notice.job.fraction ?? 0)}
            />
            <div>
              <span>
                {notice.job.state === "running"
                  ? `${notice.job.completed} of ${notice.job.total} processed${notice.job.fraction != null ? ` · current file ${Math.floor(notice.job.fraction * 100)}%` : ""}`
                  : notice.job.state === "cancelling"
                    ? "Cancelling…"
                    : notice.job.state}
              </span>
              {(notice.job.state === "running" ||
                notice.job.state === "cancelling") && (
                <button
                  disabled={notice.job.state === "cancelling"}
                  onClick={() =>
                    void native
                      .cancelJob(notice.job!.id)
                      .catch((e) => setError(String(e)))
                  }
                >
                  Cancel
                </button>
              )}
            </div>
          </div>
        )}
        {error && (
          <span className="toast-detail" role="alert">
            {error}
          </span>
        )}
      </div>
    </main>
  );
}
