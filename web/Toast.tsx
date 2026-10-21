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

/** A quiet screen-corner notice: what was saved, copied or refused. */
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
  const running =
    notice.job?.state === "running" || notice.job?.state === "cancelling";
  const job = notice.job;
  // Unknown for one item with no reported fraction: the bar then sweeps instead.
  const percent =
    !job || (job.total === 1 && job.fraction == null)
      ? undefined
      : ((job.completed + (job.fraction ?? 0)) / Math.max(job.total, 1)) * 100;
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
        {(running || notice.tone === "error") && (
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
        )}
        {/* Title, then what is happening, then the bar. A finished job reads as a
            plain result notice and leaves on its own. */}
        {notice.detail && (
          <span className="toast-detail">
            {running && notice.job
              ? [
                  notice.job.state === "cancelling"
                    ? "Cancelling…"
                    : notice.detail,
                  notice.job.total > 1 &&
                    `${notice.job.completed} of ${notice.job.total}`,
                  percent !== undefined && `${Math.floor(percent)}%`,
                ]
                  .filter(Boolean)
                  .join(" · ")
              : notice.detail}
          </span>
        )}
        {notice.job && running && (
          <div className="toast-job">
            <div
              className={`toast-bar ${percent === undefined ? "is-indeterminate" : ""}`}
              role="progressbar"
              aria-label="Progress"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={
                percent === undefined ? undefined : Math.floor(percent)
              }
            >
              <span style={{ width: `${percent ?? 100}%` }} />
            </div>
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
