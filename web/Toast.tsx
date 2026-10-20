import { useEffect, useState } from "react";
import { native, type Toast as Notice } from "./native";
import { Icon } from "./Icon";

/** Click-through notice: what an in-place action saved, copied or refused. */
export function Toast() {
  const [notice, setNotice] = useState<Notice | null>(null);
  useEffect(() => {
    let active = true;
    let remove: (() => void) | undefined;
    native
      .onToast((next) => setNotice(next))
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
  if (!notice) return null;
  return (
    <main
      className={`toast toast-${notice.tone}`}
      role={notice.tone === "error" ? "alert" : "status"}
      key={`${notice.title}|${notice.detail}`}
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
                  : "ring"
            }
            size={18}
          />
        )}
      </span>
      <span className="toast-text">
        <strong>{notice.title}</strong>
        {notice.detail && <span>{notice.detail}</span>}
      </span>
    </main>
  );
}
