import { beforeEach, describe, expect, it } from "vitest";
import { useLibraryStore } from "./libraryStore";

const base = {
  source: "stream" as const,
  url: "https://example.com/x.m4a",
  artist: "A",
  title: "T",
  addedAt: "2026-10-16T00:00:00.000Z",
};

describe("libraryStore.upsertStreamTrack (P9.5)", () => {
  beforeEach(() => {
    useLibraryStore.setState({ tracks: [] });
  });

  it("inserts with a negative id and playCount 0", () => {
    const id = useLibraryStore
      .getState()
      .upsertStreamTrack({ ...base, mbRecordingId: "mb-1" });
    expect(id).toBeLessThan(0);
    const t = useLibraryStore.getState().tracks.find((x) => x.id === id);
    expect(t?.playCount).toBe(0);
    expect(t?.mbRecordingId).toBe("mb-1");
  });

  it("dedupes by mbRecordingId and refreshes the url (re-pick)", () => {
    const a = useLibraryStore.getState().upsertStreamTrack({ ...base, mbRecordingId: "mb-1" });
    const b = useLibraryStore.getState().upsertStreamTrack({
      ...base,
      url: "https://example.com/fresh.m4a",
      mbRecordingId: "mb-1",
    });
    expect(b).toBe(a);
    const rows = useLibraryStore.getState().tracks;
    expect(rows.filter((t) => t.mbRecordingId === "mb-1")).toHaveLength(1);
    expect(rows.find((t) => t.id === a)?.url).toBe("https://example.com/fresh.m4a");
  });

  it("keeps negative ids unique across multiple picks", () => {
    const a = useLibraryStore.getState().upsertStreamTrack({ ...base, mbRecordingId: "mb-1" });
    const b = useLibraryStore.getState().upsertStreamTrack({ ...base, mbRecordingId: "mb-2" });
    expect(b).toBe(a - 1);
  });

  it("does not collide with positive (library) ids", () => {
    useLibraryStore.setState({
      tracks: [{ ...base, source: "local", path: "C:\\a.mp3", id: 1, playCount: 0 }],
    });
    const id = useLibraryStore
      .getState()
      .upsertStreamTrack({ ...base, mbRecordingId: "mb-9" });
    expect(id).toBeLessThan(0);
  });
});
