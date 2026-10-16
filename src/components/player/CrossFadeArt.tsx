import { useEffect, useRef, useState } from "react";

/** P6.2 now-playing art cross-fade (DESIGN track-change timing, 500 ms):
    the outgoing cover fades out under the incoming one — opacity only.
    P6.4: `npThumb` carries the shared View-Transition name while the
    expanded hero is closed (toggled inside the transition callback). */
export function CrossFadeArt({
  src,
  npThumb = false,
}: {
  src: string | null;
  npThumb?: boolean;
}) {
  const [prev, setPrev] = useState<string | null>(null);
  const last = useRef(src);
  const vtName = npThumb ? "np-art" : "none";

  useEffect(() => {
    if (last.current !== src) {
      setPrev(last.current);
      last.current = src;
    }
  }, [src]);

  return (
    <>
      {prev && prev !== src && (
        <img
          key={`out-${prev}`}
          className="pb-art pb-art--out"
          src={prev}
          alt=""
          style={{ viewTransitionName: vtName }}
        />
      )}
      {src ? (
        <img
          key={`in-${src}`}
          className="pb-art pb-art--in"
          src={src}
          alt=""
          style={{ viewTransitionName: vtName }}
        />
      ) : (
        <span>♪</span>
      )}
    </>
  );
}
