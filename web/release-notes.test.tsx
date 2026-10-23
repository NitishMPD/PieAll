import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ReleaseNotes } from "./ReleaseNotes";

describe("release notes", () => {
  it("formats GitHub release Markdown instead of displaying its markers", () => {
    const url = "https://github.com/NitishMPD/PieAll/compare/v0.121...v0.122";
    const html = renderToStaticMarkup(
      <ReleaseNotes
        notes={`## Changes\n\n- Fixed **formatting** and \`code\`.\n\n**Full Changelog**: ${url}`}
      />,
    );
    expect(html).toContain("<h2>Changes</h2>");
    expect(html).toContain("<ul>");
    expect(html).toContain("<strong>formatting</strong>");
    expect(html).toContain("<code>code</code>");
    expect(html).toContain(`<strong>Full Changelog</strong>: ${url}`);
    expect(html).not.toContain("**");
  });

  it("keeps remote content inert and preserves labelled link destinations", () => {
    const html = renderToStaticMarkup(
      <ReleaseNotes
        notes={
          "<script>alert(1)</script>\n\n![image](https://example.com/pixel)\n\n[Compare](https://example.com/compare)\n\n[unsafe](javascript:alert%281%29)"
        }
      />,
    );
    expect(html).not.toMatch(/<script|<img|<a\b|javascript:/);
    expect(html).toContain("Compare (https://example.com/compare)");
  });

  it("retains the empty-notes fallback", () => {
    expect(renderToStaticMarkup(<ReleaseNotes notes="" />)).toContain(
      "No release notes supplied.",
    );
  });
});
