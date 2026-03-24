import React, { useCallback, useRef, useEffect } from "react";
import type { PipelineStatus } from "stadium-eq";
import { useStadiumEQ } from "./use-stadium-eq.js";
import type { UseStadiumEQOptions } from "./use-stadium-eq.js";

export interface StadiumEqualizerProps extends UseStadiumEQOptions {
  /** Show the spectrum visualizer canvas. Defaults to true. */
  showSpectrum?: boolean;
  /** Custom class name for the root container. */
  className?: string;
  /** Custom inline styles for the root container. */
  style?: React.CSSProperties;
}

const STATUS_COLORS: Record<PipelineStatus, string> = {
  idle: "#555",
  loading: "#888",
  calibrating: "#f0a500",
  processing: "#0dff72",
  bypassed: "#ff6b6b",
  error: "#ff0000",
};

/**
 * Pre-built React component with full EQ controls.
 *
 * @example
 * ```tsx
 * import { StadiumEqualizer } from "stadium-eq-react";
 *
 * function App() {
 *   return <StadiumEqualizer wasmUrl="/stadium_eq.wasm" />;
 * }
 * ```
 */
export function StadiumEqualizer({
  showSpectrum = true,
  className,
  style,
  ...eqOptions
}: StadiumEqualizerProps) {
  const eq = useStadiumEQ(eqOptions);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<number>(0);

  // Spectrum visualization loop
  useEffect(() => {
    if (!showSpectrum || !eq.isRunning) {
      if (animRef.current) cancelAnimationFrame(animRef.current);
      animRef.current = 0;
      return;
    }

    const draw = () => {
      animRef.current = requestAnimationFrame(draw);
      const canvas = canvasRef.current;
      const analyser = eq.instance?.analyser;
      if (!canvas || !analyser) return;

      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const freqData = new Uint8Array(analyser.frequencyBinCount);
      analyser.getByteFrequencyData(freqData);

      const w = canvas.width;
      const h = canvas.height;

      ctx.fillStyle = "#0f0f23";
      ctx.fillRect(0, 0, w, h);

      const barWidth = w / freqData.length;
      for (let i = 0; i < freqData.length; i++) {
        const pct = freqData[i] / 255;
        const barHeight = pct * h;
        const r = Math.min(255, Math.floor(pct * 2 * 255));
        const g = Math.min(255, Math.floor((1 - pct) * 2 * 255));
        ctx.fillStyle = `rgb(${r}, ${g}, 40)`;
        ctx.fillRect(
          i * barWidth,
          h - barHeight,
          barWidth > 1 ? barWidth - 0.5 : barWidth,
          barHeight
        );
      }
    };

    draw();
    return () => {
      if (animRef.current) cancelAnimationFrame(animRef.current);
      animRef.current = 0;
    };
  }, [showSpectrum, eq.isRunning, eq.instance]);

  const handleToggle = useCallback(async () => {
    if (eq.isRunning) {
      eq.stop();
    } else {
      await eq.start();
    }
  }, [eq]);

  return (
    <div className={className} style={{ fontFamily: "sans-serif", color: "#e0e0e0", ...style }}>
      {/* Status */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
        <div
          style={{
            width: 10,
            height: 10,
            borderRadius: "50%",
            background: STATUS_COLORS[eq.status],
          }}
        />
        <span style={{ fontSize: 14, textTransform: "uppercase", letterSpacing: "0.05em" }}>
          {eq.status}
        </span>
        {eq.error && (
          <span style={{ fontSize: 12, color: "#ff6b6b", marginLeft: 8 }}>{eq.error}</span>
        )}
      </div>

      {/* Buttons */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <button
          onClick={handleToggle}
          style={{
            flex: 1,
            padding: "8px 16px",
            border: "none",
            borderRadius: 6,
            fontWeight: 600,
            cursor: "pointer",
            background: eq.isRunning ? "#ff6b6b" : "#0dff72",
            color: eq.isRunning ? "#fff" : "#1a1a2e",
          }}
        >
          {eq.isRunning ? "Stop" : "Start"}
        </button>
        <button
          onClick={eq.calibrate}
          disabled={!eq.isRunning}
          style={{
            flex: 1,
            padding: "8px 16px",
            border: "none",
            borderRadius: 6,
            fontWeight: 600,
            cursor: eq.isRunning ? "pointer" : "not-allowed",
            background: eq.isRunning ? "#f0a500" : "#555",
            color: eq.isRunning ? "#1a1a2e" : "#999",
          }}
        >
          Calibrate
        </button>
      </div>

      {/* Sliders */}
      <Slider
        label="Crowd"
        min={-1}
        max={1}
        step={0.01}
        value={eq.mix.crowd}
        format={(v) => v.toFixed(2)}
        onChange={(v) => eq.setMix({ crowd: v })}
      />
      <Slider
        label="Speaker"
        min={-1}
        max={1}
        step={0.01}
        value={eq.mix.speaker}
        format={(v) => v.toFixed(2)}
        onChange={(v) => eq.setMix({ speaker: v })}
      />
      <Slider
        label="Music"
        min={-1}
        max={1}
        step={0.01}
        value={eq.mix.music}
        format={(v) => v.toFixed(2)}
        onChange={(v) => eq.setMix({ music: v })}
      />
      <Slider
        label="Gain (dB)"
        min={-20}
        max={20}
        step={0.5}
        value={eq.mix.gainDb}
        format={(v) => `${v.toFixed(1)} dB`}
        onChange={(v) => eq.setMix({ gainDb: v })}
      />

      {/* Bypass */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 12px",
          background: "#16213e",
          borderRadius: 6,
          marginBottom: 16,
        }}
      >
        <label style={{ fontSize: 14, fontWeight: 600 }}>Bypass</label>
        <input
          type="checkbox"
          onChange={(e) => eq.setBypass(e.target.checked)}
        />
      </div>

      {/* Spectrum */}
      {showSpectrum && (
        <div style={{ background: "#16213e", borderRadius: 6, padding: 8 }}>
          <div
            style={{
              fontSize: 12,
              textTransform: "uppercase",
              letterSpacing: "0.05em",
              color: "#888",
              marginBottom: 6,
            }}
          >
            Spectrum
          </div>
          <canvas
            ref={canvasRef}
            width={256}
            height={128}
            style={{ display: "block", width: "100%", height: 128, borderRadius: 4, background: "#0f0f23" }}
          />
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Internal slider component
// ---------------------------------------------------------------------------

interface SliderProps {
  label: string;
  min: number;
  max: number;
  step: number;
  value: number;
  format: (v: number) => string;
  onChange: (v: number) => void;
}

function Slider({ label, min, max, step, value, format, onChange }: SliderProps) {
  return (
    <div style={{ display: "flex", alignItems: "center", marginBottom: 10 }}>
      <span style={{ width: 70, fontSize: 13, fontWeight: 600, flexShrink: 0 }}>{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        style={{ flex: 1 }}
      />
      <span
        style={{
          width: 55,
          textAlign: "right",
          fontSize: 13,
          fontVariantNumeric: "tabular-nums",
          flexShrink: 0,
        }}
      >
        {format(value)}
      </span>
    </div>
  );
}
