/** OQ4/P9.4: WCAG relative luminance + on-accent text pick. Dynamic album
    accents are unbounded — white text can vanish on a pale accent. Threshold
    math: white (#fff) needs L <= 0.1833 for 4.5:1; near-black (#07080c)
    needs L >= 0.186. 0.185 sits in that hairline band, biased to white
    (most accents are dark). The token default pairs with the static
    fallback accent #7c5cff (dark text = 4.8:1). */

export function relativeLuminance(hex: string): number | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  const ch = (shift: number): number => {
    const c = ((n >> shift) & 0xff) / 255;
    return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * ch(16) + 0.7152 * ch(8) + 0.0722 * ch(0);
}

/** Readable text color for a given accent background. */
export function onAccentFor(hex: string | null | undefined): string {
  if (!hex) return "#07080c";
  const L = relativeLuminance(hex);
  if (L == null) return "#07080c";
  return L <= 0.185 ? "#ffffff" : "#07080c";
}
