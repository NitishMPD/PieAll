import { useEffect, useState, type CSSProperties } from "react";
import { INNER, OUTER, SIZE, point, wedgePath } from "./geometry";
import { Icon, type IconName } from "./Icon";

const FEATURES: [IconName, string][] = [
  ["images", "Images"],
  ["video", "Video"],
  ["audio", "Audio"],
  ["pdf", "PDF"],
  ["download", "Downloads"],
  ["keyboard", "Shortcuts"],
];
const START_MS = 250;
const STEP_MS = 170;

// A mini ring in the logo colors: a quick swipe highlights each feature slice while the center names it, then it lands on PieAll.
export function Splash() {
  const [step, setStep] = useState(-1);
  useEffect(() => {
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setStep(FEATURES.length);
      return;
    }
    // One timer per feature plus a final one that lands on PieAll.
    const timers = Array.from({ length: FEATURES.length + 1 }, (_, i) =>
      setTimeout(() => setStep(i), START_MS + i * STEP_MS),
    );
    return () => timers.forEach(clearTimeout);
  }, []);
  const done = step >= FEATURES.length;
  const label = done ? "PieAll" : (FEATURES[step]?.[1] ?? "");
  const count = FEATURES.length;
  return (
    <div className="splash" aria-hidden="true">
      <div className={`splash-ring ${done ? "is-done" : ""}`}>
        <svg viewBox={`0 0 ${SIZE} ${SIZE}`}>
          {FEATURES.map(([icon, name], i) => {
            const angle = (i * 360) / count;
            const [x, y] = point((OUTER + INNER) / 2, angle);
            const [px, py] = point(6, angle);
            return (
              <g
                key={name}
                className={`splash-slice ${i === step ? "is-selected" : ""}`}
                style={
                  {
                    "--pop-x": `${px - SIZE / 2}px`,
                    "--pop-y": `${py - SIZE / 2}px`,
                  } as CSSProperties
                }
              >
                <path d={wedgePath(i, count)} />
                <g transform={`translate(${x - 15} ${y - 15})`}>
                  <Icon name={icon} size={30} />
                </g>
              </g>
            );
          })}
        </svg>
        <div className="splash-center">
          <strong key={label}>{label}</strong>
        </div>
      </div>
    </div>
  );
}
