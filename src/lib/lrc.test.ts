import { describe, expect, it } from "vitest";
import { activeLineIndex, parseLrc } from "./lrc";

describe("parseLrc", () => {
  it("parses centisecond, millisecond and bare timestamps", () => {
    const lines = parseLrc(
      ["[00:12.34]centi", "[00:12.345]milli", "[01:02]bare", "[00:03.5]tenth"].join("\n"),
    );
    expect(lines).toEqual([
      { timeMs: 3500, text: "tenth" },
      { timeMs: 12340, text: "centi" },
      { timeMs: 12345, text: "milli" },
      { timeMs: 62000, text: "bare" },
    ]);
  });

  it("multi-line: several timestamps on one line share the text", () => {
    const lines = parseLrc("[00:10.00][00:50.00][01:30]chorus\n[00:20.00]verse");
    expect(lines.map((l) => l.timeMs)).toEqual([10000, 20000, 50000, 90000]);
    expect(lines.map((l) => l.text)).toEqual(["chorus", "verse", "chorus", "chorus"]);
  });

  it("offset header shifts every timestamp (clamped at 0)", () => {
    const plus = parseLrc("[offset:+500]\n[00:01.00]a\n[00:02.00]b");
    expect(plus.map((l) => l.timeMs)).toEqual([1500, 2500]);
    const minus = parseLrc("[offset:-200]\n[00:01.00]a");
    expect(minus[0].timeMs).toBe(800);
    const clamp = parseLrc("[offset:-1500]\n[00:01.00]a");
    expect(clamp[0].timeMs).toBe(0);
  });

  it("malformed lines are skipped without throwing", () => {
    const lines = parseLrc(
      [
        "prose with no tags",
        "[aa:bb]bad",
        "[00:99.00]seconds over 59",
        "[12:34.5678]too many frac digits",
        "",
        "[00:05.00]ok",
        "][",
        "[00:06.00",
      ].join("\n"),
    );
    expect(lines).toEqual([{ timeMs: 5000, text: "ok" }]);
  });

  it("metadata tags are skipped but keep same-line lyrics", () => {
    const lines = parseLrc("[ar:Someone][ti:Song]\n[ar:X][00:07.00]hello\n[by:me]");
    expect(lines).toEqual([{ timeMs: 7000, text: "hello" }]);
  });

  it("brackets inside the lyric text stay text", () => {
    const lines = parseLrc("[00:01.00]see [chorus] here");
    expect(lines[0].text).toBe("see [chorus] here");
  });

  it("out-of-order input is sorted by time", () => {
    const lines = parseLrc("[00:30.00]c\n[00:10.00]a\n[00:20.00]b");
    expect(lines.map((l) => l.text)).toEqual(["a", "b", "c"]);
  });
});

describe("activeLineIndex", () => {
  const lines = parseLrc("[00:10.00]a\n[00:20.00]b\n[00:30.00]c");

  it("is -1 before the first timestamp", () => {
    expect(activeLineIndex(lines, 0)).toBe(-1);
    expect(activeLineIndex(lines, 9999)).toBe(-1);
  });

  it("hits exact boundaries", () => {
    expect(activeLineIndex(lines, 10000)).toBe(0);
    expect(activeLineIndex(lines, 20000)).toBe(1);
  });

  it("stays on the last passed line until the next begins", () => {
    expect(activeLineIndex(lines, 29999)).toBe(1);
    expect(activeLineIndex(lines, 30000)).toBe(2);
    expect(activeLineIndex(lines, 999999)).toBe(2);
  });

  it("is -1 for an empty lyric list", () => {
    expect(activeLineIndex([], 123)).toBe(-1);
  });
});
