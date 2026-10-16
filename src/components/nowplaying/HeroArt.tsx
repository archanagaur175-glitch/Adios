import { useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";
import { prefersReducedMotion } from "@/lib/viewTransition";
import { usePlayerStore } from "@/store/playerStore";
import "./hero.css";

const TILT_MAX_X = 10;
const TILT_MAX_Y = 12;

/** P6.4 hero art (DESIGN L3): pointer-tracked tilt + glare over the cover,
    a vinyl disc peeking out behind it that spins while playing. Reduced
    motion: no tilt (JS) + no spin/slide (CSS media query). */
export function HeroArt({ coverPath }: { coverPath: string | null }) {
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const [tilt, setTilt] = useState({ x: 0, y: 0 });
  const ref = useRef<HTMLDivElement>(null);

  const onMove = (e: React.PointerEvent) => {
    if (prefersReducedMotion() || !ref.current) return;
    const r = ref.current.getBoundingClientRect();
    const px = (e.clientX - r.left) / r.width - 0.5;
    const py = (e.clientY - r.top) / r.height - 0.5;
    setTilt({ x: -py * TILT_MAX_X, y: px * TILT_MAX_Y });
  };

  const src = inTauri && coverPath ? convertFileSrc(coverPath) : null;
  const glareOpacity = Math.min(1, Math.hypot(tilt.x, tilt.y) / 5);

  return (
    <div
      className="hero"
      ref={ref}
      onPointerMove={onMove}
      onPointerLeave={() => setTilt({ x: 0, y: 0 })}
    >
      <div
        className={`hero__vinyl${isPlaying ? " hero__vinyl--spin" : ""}`}
        aria-hidden="true"
      />
      <div
        className="hero__art"
        style={{
          transform: `perspective(900px) rotateX(${tilt.x}deg) rotateY(${tilt.y}deg)`,
        }}
      >
        {src ? (
          <img
            key={src}
            src={src}
            alt=""
            draggable={false}
            style={{ viewTransitionName: "np-art" }}
          />
        ) : (
          <span className="hero__note">♪</span>
        )}
      </div>
      <div
        className="hero__glare"
        aria-hidden="true"
        style={
          {
            opacity: glareOpacity,
            "--gx": `${tilt.y * 4 + 50}%`,
            "--gy": `${-tilt.x * 5 + 50}%`,
          } as React.CSSProperties
        }
      />
    </div>
  );
}
