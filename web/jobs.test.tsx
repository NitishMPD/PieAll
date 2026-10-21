import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Jobs } from "./Jobs";
import type { Job } from "./native";

const job: Job = {
  id: 1,
  title: "Download video",
  detail: "Downloading first video",
  state: "running",
  completed: 0,
  total: 1,
};
const perform = async (action: () => Promise<unknown>) => {
  await action();
};

describe("parallel job controls", () => {
  it("keeps separate cancellation and progress controls for overlapping jobs", () => {
    const html = renderToStaticMarkup(
      <Jobs
        jobs={[
          job,
          { ...job, id: 2, title: "Convert video", fraction: 0.5 },
          { ...job, id: 3, state: "cancelling" },
        ]}
        perform={perform}
      />,
    );
    expect(html).toContain("Jobs · 3 running");
    expect(html).toContain('aria-label="Cancel Download video, job 1"');
    expect(html).toContain('aria-label="Cancel Convert video, job 2"');
    expect(html).toMatch(/<progress[^>]*job 1[^>]*><\/progress>/);
    expect(html).toMatch(/<progress[^>]*job 2[^>]*value="0.5"/);
    expect(html).toMatch(/aria-label="Cancel Download video, job 3" disabled/);
  });

  it("keeps finished results and output locations accessible alongside running work", () => {
    const html = renderToStaticMarkup(
      <Jobs
        jobs={[
          job,
          {
            ...job,
            id: 2,
            state: "complete",
            output: "C:/second.mkv",
            report: "Saved C:/second.mkv",
          },
          { ...job, id: 3, state: "failed", report: "The engine failed." },
        ]}
        perform={perform}
      />,
    );
    expect(html).toContain("Jobs · 1 running");
    expect(html).toContain("Open location");
    expect(html).toContain("Saved C:/second.mkv");
    expect(html).toContain("The engine failed.");
    expect(html.match(/aria-label="Cancel /g)).toHaveLength(1);
  });

  it("offers results loaded on demand without requiring full reports in progress snapshots", () => {
    const html = renderToStaticMarkup(
      <Jobs
        jobs={[{ ...job, state: "complete", has_report: true }]}
        perform={perform}
      />,
    );
    expect(html).toContain("<summary>Result · Download video</summary>");
    expect(html).not.toContain("<pre>");
  });
});
