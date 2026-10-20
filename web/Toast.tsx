import { useEffect, useState } from "react";
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
  useEvent(() =>
    native.onToast((next) => {
      setNotice(next);
      setLeaving(false);
      setSerial((n) => n + 1);
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
        {notice.detail && <span className="toast-detail">{notice.detail}</span>}
      </div>
    </main>
  );
}
