import { describe, expect, it } from "vitest";
import { mediaTitles, readableName } from "./media-titles";
import type { MediaItem } from "./native";

const item = (
  url: string,
  kind: MediaItem["kind"],
  name: string | null = null,
): MediaItem => ({ url, kind, name, preview: null });

describe("media titles", () => {
  it("rejects numeric IDs and hashes", () => {
    expect(readableName("1795016117468")).toBeNull();
    expect(readableName("bn39hirwzjqj18ej1fkz55671")).toBeNull();
    expect(readableName("asiqslyf4ooq7ggllg4fyo4o2.jpg")).toBeNull();
    expect(readableName("")).toBeNull();
  });
  it("keeps readable names without extensions or separators", () => {
    expect(readableName("rally-launch_trailer.mp4")).toBe(
      "rally launch trailer",
    );
    expect(readableName("Team photo")).toBe("Team photo");
  });
  it("numbers fallbacks per type in scan order", () => {
    const titles = mediaTitles([
      item("https://x.test/v/1795016117468", "video"),
      item("https://x.test/i/1731790370623.jpg", "image"),
      item("https://x.test/i/sunset-beach.jpg", "image"),
      item("https://x.test/i/2tzoeodxy0zug4455msr0oq0v", "image"),
      item("https://x.test/g/a.gif", "gif", "Party parrot"),
    ]);
    expect([...titles.values()]).toEqual([
      "Video 1",
      "Image 1",
      "sunset beach",
      "Image 3",
      "Party parrot",
    ]);
  });
});
