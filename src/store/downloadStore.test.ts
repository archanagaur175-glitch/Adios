import { beforeEach, describe, expect, it } from "vitest";
import { useDownloadStore } from "./downloadStore";

describe("downloadStore (P12)", () => {
  beforeEach(() => {
    useDownloadStore.setState({ active: {}, cachedIds: {}, done: [] });
  });

  it("setProgress tracks active downloads per track", () => {
    useDownloadStore.getState().setProgress({ trackId: 1, received: 500, total: 1000 });
    useDownloadStore.getState().setProgress({ trackId: 2, received: 10, total: null });
    const s = useDownloadStore.getState();
    expect(s.active[1]).toEqual({ trackId: 1, received: 500, total: 1000 });
    expect(s.active[2]?.total).toBeNull();
  });

  it("clearActive removes only the finished track", () => {
    useDownloadStore.getState().setProgress({ trackId: 1, received: 1, total: 2 });
    useDownloadStore.getState().setProgress({ trackId: 2, received: 1, total: 2 });
    useDownloadStore.getState().clearActive(1);
    const s = useDownloadStore.getState();
    expect(s.active[1]).toBeUndefined();
    expect(s.active[2]).toBeDefined();
  });

  it("setCachedIds replaces the badge set", () => {
    useDownloadStore.getState().setCachedIds([3, 7]);
    useDownloadStore.getState().setCachedIds([9]);
    const s = useDownloadStore.getState();
    expect(s.cachedIds[9]).toBe(true);
    expect(s.cachedIds[3]).toBeUndefined();
  });

  it("addCachedIds merges into the badge set", () => {
    useDownloadStore.getState().setCachedIds([3]);
    useDownloadStore.getState().addCachedIds([7, 9]);
    const s = useDownloadStore.getState();
    expect(s.cachedIds[3]).toBe(true);
    expect(s.cachedIds[7]).toBe(true);
    expect(s.cachedIds[9]).toBe(true);
  });

  it("markDone records finished downloads, newest last", () => {
    useDownloadStore.getState().markDone({ trackId: 1, title: "A" });
    useDownloadStore.getState().markDone({ trackId: 2, title: "B" });
    expect(useDownloadStore.getState().done).toEqual([
      { trackId: 1, title: "A" },
      { trackId: 2, title: "B" },
    ]);
  });

  it("markDone replaces an earlier entry for the same track", () => {
    useDownloadStore.getState().markDone({ trackId: 1, title: "old" });
    useDownloadStore.getState().markDone({ trackId: 1, title: "new" });
    expect(useDownloadStore.getState().done).toEqual([{ trackId: 1, title: "new" }]);
  });

  it("clearDone removes only the given track", () => {
    useDownloadStore.getState().markDone({ trackId: 1, title: "A" });
    useDownloadStore.getState().markDone({ trackId: 2, title: "B" });
    useDownloadStore.getState().clearDone(1);
    expect(useDownloadStore.getState().done).toEqual([{ trackId: 2, title: "B" }]);
  });
});
