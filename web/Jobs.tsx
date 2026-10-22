import { useState } from "react";
import { native, type Job } from "./native";
import { Icon } from "./Icon";
import "./jobs.css";

export function Jobs({
  jobs,
  perform,
}: {
  jobs: Job[];
  perform: (action: () => Promise<unknown>) => Promise<void>;
}) {
  const [dismissed, setDismissed] = useState<Set<number>>(() => new Set());
  const visible = jobs.filter((job) => !dismissed.has(job.id));
  if (!visible.length) return null;
  const running = jobs.filter((job) =>
    ["running", "cancelling"].includes(job.state),
  );
  return (
    <section className="jobs" aria-label="Action notifications">
      <h2 className="sr-only" aria-live="polite">
        Jobs · {running.length} running
      </h2>
      <ul className="jobs-list">
        {visible.map((job) => {
          const active = ["running", "cancelling"].includes(job.state);
          const failed = ["failed", "partial"].includes(job.state);
          return (
            <li key={job.id} className="job-toast">
              <span className="job-mark" aria-hidden="true">
                {active ? (
                  <span className="spinner" />
                ) : (
                  <Icon
                    name={
                      failed
                        ? "alert"
                        : job.state === "complete"
                          ? "check"
                          : "info"
                    }
                    size={16}
                  />
                )}
              </span>
              <div className="job-summary">
                <strong role={failed ? "alert" : "status"}>{job.title}</strong>
                <button
                  className="job-close icon-button"
                  aria-label={`Close notification for ${job.title}, job ${job.id}`}
                  title={
                    active
                      ? "Close notification (job continues)"
                      : "Close notification"
                  }
                  onClick={() =>
                    setDismissed((previous) => new Set(previous).add(job.id))
                  }
                >
                  <Icon name="close" size={14} />
                </button>
              </div>
              {job.detail && job.detail !== job.title && <p>{job.detail}</p>}
              <div className="job-controls">
                <span>
                  {job.state === "partial"
                    ? "Completed with errors"
                    : job.state}
                </span>
                {active && (
                  <button
                    aria-label={`Cancel ${job.title}, job ${job.id}`}
                    disabled={job.state === "cancelling"}
                    onClick={() => void perform(() => native.cancelJob(job.id))}
                  >
                    {job.state === "cancelling" ? "Cancelling…" : "Cancel"}
                  </button>
                )}
                {job.output && job.state === "complete" && (
                  <button
                    onClick={() =>
                      void perform(() => native.openDownloadLocation(job.id))
                    }
                  >
                    Open location
                  </button>
                )}
              </div>
              {active && (
                <progress
                  aria-label={`${job.title}, job ${job.id}: files processed`}
                  max={job.total || 1}
                  value={
                    job.total === 1 && job.fraction == null
                      ? undefined
                      : job.completed + (job.fraction ?? 0)
                  }
                />
              )}
              {!active && (job.has_report || job.report) && (
                <JobResult job={job} />
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

function JobResult({ job }: { job: Job }) {
  const [report, setReport] = useState(job.report ?? "");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  return (
    <details
      onToggle={(event) => {
        if (!event.currentTarget.open || report || loading) return;
        setLoading(true);
        setError("");
        void native
          .jobReport(job.id)
          .then(setReport)
          .catch((error) => setError(String(error)))
          .finally(() => setLoading(false));
      }}
    >
      <summary>View result</summary>
      {loading && <p role="status">Loading result…</p>}
      {error && <p role="alert">{error}</p>}
      {report && <pre tabIndex={0}>{report}</pre>}
    </details>
  );
}
