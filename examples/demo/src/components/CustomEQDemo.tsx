"use client";

import React, { useRef, useEffect } from "react";
import { useStadiumEQ } from "stadium-eq-react";

const STATUS_COLORS: Record<string, string> = {
  idle: "#555",
  loading: "#888",
  calibrating: "#f0a500",
  processing: "#0dff72",
  bypassed: "#ff6b6b",
  error: "#ff0000",
};

export default function CustomEQDemo() {
  const eq = useStadiumEQ({ wasmUrl: "/stadium_eq.wasm" });
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<number>(0);

  // Custom circular spectrum visualizer
  useEffect(() => {
    if (!eq.isRunning) {
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

      const data = new Uint8Array(analyser.frequencyBinCount);
      analyser.getByteFrequencyData(data);

      const w = canvas.width;
      const h = canvas.height;
      const cx = w / 2;
      const cy = h / 2;
      const baseR = 40;
      const maxR = Math.min(cx, cy) - 10;

      ctx.fillStyle = "#0f0f23";
      ctx.fillRect(0, 0, w, h);

      const bins = 64;
      const step = Math.PI * 2 / bins;
      ctx.beginPath();
      for (let i = 0; i <= bins; i++) {
        const idx = Math.floor((i / bins) * data.length);
        const pct = data[idx % data.length] / 255;
        const r = baseR + pct * (maxR - baseR);
        const angle = i * step - Math.PI / 2;
        const x = cx + Math.cos(angle) * r;
        const y = cy + Math.sin(angle) * r;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.closePath();
      ctx.strokeStyle = "#0dff72";
      ctx.lineWidth = 2;
      ctx.stroke();
      ctx.fillStyle = "rgba(13, 255, 114, 0.05)";
      ctx.fill();
    };
    draw();
    return () => {
      if (animRef.current) cancelAnimationFrame(animRef.current);
      animRef.current = 0;
    };
  }, [eq.isRunning, eq.instance]);

  const sliders: { label: string; key: "crowd" | "speaker" | "music"; icon: string }[] = [
    { label: "Crowd", key: "crowd", icon: "👥" },
    { label: "Speaker", key: "speaker", icon: "🎙" },
    { label: "Music", key: "music", icon: "🎵" },
  ];

  return (
    <div style={{ background: "#16213e", borderRadius: 12, padding: 24 }}>
      {/* Status bar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: 20,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <div
            style={{
              width: 12,
              height: 12,
              borderRadius: "50%",
              background: STATUS_COLORS[eq.status] ?? "#555",
              boxShadow: eq.isRunning ? `0 0 8px ${STATUS_COLORS[eq.status]}` : "none",
            }}
          />
          <span style={{ fontSize: 13, textTransform: "uppercase", letterSpacing: 1 }}>
            {eq.status}
          </span>
        </div>
        {eq.error && (
          <span style={{ fontSize: 12, color: "#ff6b6b" }}>{eq.error}</span>
        )}
      </div>

      {/* Circular visualizer */}
      <div style={{ display: "flex", justifyContent: "center", marginBottom: 20 }}>
        <canvas
          ref={canvasRef}
          width={240}
          height={240}
          style={{
            borderRadius: "50%",
            background: "#0f0f23",
            border: `2px solid ${eq.isRunning ? "#0dff7233" : "#1e1e3a"}`,
          }}
        />
      </div>

      {/* Action buttons */}
      <div style={{ display: "flex", gap: 8, marginBottom: 20 }}>
        <button
          onClick={eq.isRunning ? eq.stop : () => eq.start()}
          style={{
            flex: 1,
            padding: "10px 0",
            border: "none",
            borderRadius: 8,
            fontWeight: 700,
            fontSize: 14,
            cursor: "pointer",
            background: eq.isRunning ? "#ff6b6b" : "#0dff72",
            color: eq.isRunning ? "#fff" : "#0f0f23",
            transition: "background 0.2s",
          }}
        >
          {eq.isRunning ? "Stop" : "Start"}
        </button>
        <button
          onClick={eq.calibrate}
          disabled={!eq.isRunning}
          style={{
            flex: 1,
            padding: "10px 0",
            border: "none",
            borderRadius: 8,
            fontWeight: 700,
            fontSize: 14,
            cursor: eq.isRunning ? "pointer" : "not-allowed",
            background: eq.isRunning ? "#f0a500" : "#2a2a4a",
            color: eq.isRunning ? "#0f0f23" : "#666",
            transition: "background 0.2s",
          }}
        >
          Calibrate
        </button>
      </div>

      {/* Custom card-style sliders */}
      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 16 }}>
        {sliders.map(({ label, key, icon }) => (
          <div
            key={key}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
              background: "#1a1a2e",
              borderRadius: 8,
              padding: "10px 14px",
            }}
          >
            <span style={{ fontSize: 18, width: 24 }}>{icon}</span>
            <span style={{ fontSize: 13, fontWeight: 600, width: 60, flexShrink: 0 }}>
              {label}
            </span>
            <input
              type="range"
              min={-1}
              max={1}
              step={0.01}
              value={eq.mix[key]}
              onChange={(e) => eq.setMix({ [key]: Number(e.target.value) })}
              style={{ flex: 1 }}
            />
            <span
              style={{
                fontSize: 12,
                width: 44,
                textAlign: "right",
                fontVariantNumeric: "tabular-nums",
                color: "#aaa",
              }}
            >
              {eq.mix[key].toFixed(2)}
            </span>
          </div>
        ))}

        {/* Gain slider */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 12,
            background: "#1a1a2e",
            borderRadius: 8,
            padding: "10px 14px",
          }}
        >
          <span style={{ fontSize: 18, width: 24 }}>🔊</span>
          <span style={{ fontSize: 13, fontWeight: 600, width: 60, flexShrink: 0 }}>
            Gain
          </span>
          <input
            type="range"
            min={-20}
            max={20}
            step={0.5}
            value={eq.mix.gainDb}
            onChange={(e) => eq.setMix({ gainDb: Number(e.target.value) })}
            style={{ flex: 1 }}
          />
          <span
            style={{
              fontSize: 12,
              width: 44,
              textAlign: "right",
              fontVariantNumeric: "tabular-nums",
              color: "#aaa",
            }}
          >
            {eq.mix.gainDb.toFixed(1)}dB
          </span>
        </div>
      </div>

      {/* Bypass toggle */}
      <label
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          background: "#1a1a2e",
          borderRadius: 8,
          padding: "10px 14px",
          cursor: "pointer",
          fontSize: 13,
          fontWeight: 600,
        }}
      >
        Bypass
        <input
          type="checkbox"
          onChange={(e) => eq.setBypass(e.target.checked)}
        />
      </label>
    </div>
  );
}
