import { cn } from "../lib/format";
import { CrosshairSettings, crosshairColor } from "../lib/crosshair";

export const SCENES = [
  { id: "ascent", name: "Ascent", art: "/maps/ascent-attack.svg" },
  { id: "bind", name: "Bind", art: "/maps/bind-attack.svg" },
  { id: "haven", name: "Haven", art: "/maps/haven-attack.svg" },
  { id: "split", name: "Split", art: "/maps/split-attack.svg" },
  { id: "icebox", name: "Icebox", art: "/maps/icebox-attack.svg" },
  { id: "lotus", name: "Lotus", art: "/maps/lotus-attack.svg" },
  { id: "sunset", name: "Sunset", art: "/maps/sunset-attack.svg" },
  { id: "fracture", name: "Fracture", art: "/maps/fracture-attack.svg" },
  { id: "breeze", name: "Breeze", art: "/maps/breeze-attack.svg" },
];

function OutlineRect({
  x,
  y,
  width,
  height,
  color,
  opacity,
  outline,
  outlineOpacity,
  rotate,
}: {
  x: number;
  y: number;
  width: number;
  height: number;
  color: string;
  opacity: number;
  outline: boolean;
  outlineOpacity: number;
  rotate: number;
}) {
  return (
    <g transform={`rotate(${rotate})`}>
      {outline && (
        <rect
          x={x - 0.7}
          y={y - 0.7}
          width={width + 1.4}
          height={height + 1.4}
          fill="#050608"
          opacity={outlineOpacity}
        />
      )}
      <rect x={x} y={y} width={width} height={height} fill={color} opacity={opacity} />
    </g>
  );
}

export function CrosshairPreview({
  settings,
  size = 210,
  scene = "ascent",
  state = "resting",
  className,
}: {
  settings: CrosshairSettings;
  size?: number;
  scene?: string;
  state?: "resting" | "moving" | "firing";
  className?: string;
}) {
  const color = crosshairColor(settings);
  const unit = size / 52;
  const center = size / 2;
  const inner = settings.inner;
  const outer = settings.outer;
  const innerGap = inner.offset * unit;
  const innerLength = inner.length * unit;
  const innerThickness = Math.max(0.6, inner.thickness * unit * 0.42);
  const outerGap = innerGap + innerLength + outer.offset * unit;
  const outerLength = outer.length * unit;
  const outerThickness = Math.max(0.6, outer.thickness * unit * 0.42);
  const jitter = state === "moving" ? 1.6 : state === "firing" ? 0.7 : 0;
  const sceneArt = SCENES.find((item) => item.id === scene)?.art ?? SCENES[0].art;

  return (
    <div
      className={cn("crosshair-preview", `state-${state}`, className)}
      style={{ width: size, height: size }}
    >
      <img src={sceneArt} alt="" className="crosshair-scene" draggable={false} />
      <span className="scene-vignette" />
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-label="Crosshair preview">
        <g transform={`translate(${center + jitter}, ${center + jitter * 0.4})`}>
          {inner.thickness > 0 && inner.length > 0 && (
            <>
              <OutlineRect
                x={innerGap}
                y={-innerThickness / 2}
                width={innerLength}
                height={innerThickness}
                color={color}
                opacity={inner.opacity}
                outline={settings.outlines}
                outlineOpacity={settings.outlineOpacity}
                rotate={0}
              />
              <OutlineRect
                x={-innerGap - innerLength}
                y={-innerThickness / 2}
                width={innerLength}
                height={innerThickness}
                color={color}
                opacity={inner.opacity}
                outline={settings.outlines}
                outlineOpacity={settings.outlineOpacity}
                rotate={0}
              />
              <OutlineRect
                x={-innerThickness / 2}
                y={innerGap}
                width={innerThickness}
                height={innerLength}
                color={color}
                opacity={inner.opacity}
                outline={settings.outlines}
                outlineOpacity={settings.outlineOpacity}
                rotate={0}
              />
              <OutlineRect
                x={-innerThickness / 2}
                y={-innerGap - innerLength}
                width={innerThickness}
                height={innerLength}
                color={color}
                opacity={inner.opacity}
                outline={settings.outlines}
                outlineOpacity={settings.outlineOpacity}
                rotate={0}
              />
            </>
          )}
          {outer.thickness > 0 && outer.length > 0 && (
            <>
              <OutlineRect
                x={outerGap}
                y={-outerThickness / 2}
                width={outerLength}
                height={outerThickness}
                color={color}
                opacity={outer.opacity}
                outline={settings.outlines}
                outlineOpacity={settings.outlineOpacity}
                rotate={0}
              />
              <OutlineRect
                x={-outerGap - outerLength}
                y={-outerThickness / 2}
                width={outerLength}
                height={outerThickness}
                color={color}
                opacity={outer.opacity}
                outline={settings.outlines}
                outlineOpacity={settings.outlineOpacity}
                rotate={0}
              />
              <OutlineRect
                x={-outerThickness / 2}
                y={outerGap}
                width={outerThickness}
                height={outerLength}
                color={color}
                opacity={outer.opacity}
                outline={settings.outlines}
                outlineOpacity={settings.outlineOpacity}
                rotate={0}
              />
              <OutlineRect
                x={-outerThickness / 2}
                y={-outerGap - outerLength}
                width={outerThickness}
                height={outerLength}
                color={color}
                opacity={outer.opacity}
                outline={settings.outlines}
                outlineOpacity={settings.outlineOpacity}
                rotate={0}
              />
            </>
          )}
          {settings.dot && (
            <g>
              {settings.outlines && (
                <circle
                  r={Math.max(1.3, settings.dotSize * unit * 0.28 + 1)}
                  fill="#050608"
                  opacity={settings.outlineOpacity}
                />
              )}
              <circle
                r={Math.max(1, settings.dotSize * unit * 0.28)}
                fill={color}
                opacity={settings.dotOpacity}
              />
            </g>
          )}
        </g>
      </svg>
      {state !== "resting" && (
        <span className="crosshair-state">{state}</span>
      )}
    </div>
  );
}
