import { useState } from "react";
import { native, type Job } from "./native";
import "./jobs.css";

export function Jobs({
  jobs,
  perform,
}: {
  jobs: Job[];
  perform: (action: () => Promise<unknown>) => Promise<void>;
}) {
  if (!jobs.length) return null;
  const running = jobs.filter((job) =>
    ["running", "cancelling"].includes(job.state),
  );
  return (
    <section className="jobs" aria-label="Jobs">
      <h2 aria-live="polite">Jobs · {running.length} running</h2>
      <ul className="jobs-list">
        {jobs.map((job) => {
          const active = ["running", "cancelling"].includes(job.state);
          return (
            <li key={job.id}>
              <div className="job-summary">
                <strong>{job.title}</strong>
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
              <p>{job.detail}</p>
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
      <summary>Result · {job.title}</summary>
      {loading && <p role="status">Loading result…</p>}
      {error && <p role="alert">{error}</p>}
      {report && <pre>{report}</pre>}
    </details>
  );
}
