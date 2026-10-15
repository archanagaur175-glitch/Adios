import { useEffect, useRef, useState } from "react";

/** P6.2 now-playing art cross-fade (DESIGN track-change timing, 500 ms):
    the outgoing cover fades out under the incoming one — opacity only. */
export function CrossFadeArt({ src }: { src: string | null }) {
  const [prev, setPrev] = useState<string | null>(null);
  const last = useRef(src);

  useEffect(() => {
    if (last.current !== src) {
      setPrev(last.current);
      last.current = src;
    }
  }, [src]);

  return (
    <>
      {prev && prev !== src && (
        <img key={`out-${prev}`} className="pb-art pb-art--out" src={prev} alt="" />
      )}
      {src ? (
        <img key={`in-${src}`} className="pb-art pb-art--in" src={src} alt="" />
      ) : (
        <span>♪</span>
      )}
    </>
  );
}
