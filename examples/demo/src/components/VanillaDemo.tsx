"use client";

import React, { useEffect, useRef, useState, useCallback } from "react";
import { StadiumEQ } from "stadium-eq";
import type { PipelineStatus, MixLevels } from "stadium-eq";

const STATUS_COLORS: Record<string, string> = {
  idle: "#555",
  loading: "#888",
  calibrating: "#f0a500",
  processing: "#0dff72",
  bypassed: "#ff6b6b",
  error: "#ff0000",
};

/**
 * Demonstrates the vanilla JS wrapper (no React hooks) — we just manage
 * the StadiumEQ class imperatively and bridge it to React state for display.
 */
export default function VanillaDemo() {
  const eqRef = useRef<StadiumEQ | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<number>(0);

  const [status, setStatus] = useState<PipelineStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [mix, setMix] = useState<MixLevels>({
    crowd: 0,
    speaker: 0,
    music: 0,
    gainDb: 0,
  });
  const [log, setLog] = useState<string[]>([]);

  const appendLog = useCallback((msg: string) => {
    setLog((prev) => [...prev.slice(-19), `[${new Date().toLocaleTimeString()}] ${msg}`]);
  }, []);

  // Create instance once
  useEffect(() => {
    const eq = new StadiumEQ({ wasmUrl: "/stadium_eq.wasm" });

    eq.on("statuschange", (s: PipelineStatus) => {
      setStatus(s);
      appendLog(`status → ${s}`);
    });
    eq.on("error", (msg: string) => {
      setError(msg);
      appendLog(`error: ${msg}`);
    });
    eq.on("ready", () => appendLog("ready"));
    eq.on("destroyed", () => appendLog("destroyed"));

    eqRef.current = eq;
    appendLog("StadiumEQ instance created");

    return () => {
      eq.destroy();
      eqRef.current = null;
    };
  }, [appendLog]);

  // Bar spectrum visualization
  const isRunning = status === "processing" || status === "calibrating" || status === "bypassed";

  useEffect(() => {
    if (!isRunning) {
      if (animRef.current) cancelAnimationFrame(animRef.current);
      animRef.current = 0;
      return;
    }
    const draw = () => {
      animRef.current = requestAnimationFrame(draw);
      const canvas = canvasRef.current;
      const analyser = eqRef.current?.analyser;
      if (!canvas || !analyser) return;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const data = new Uint8Array(analyser.frequencyBinCount);
      analyser.getByteFrequencyData(data);

      const w = canvas.width;
      const h = canvas.height;
      ctx.fillStyle = "#0f0f23";
      ctx.fillRect(0, 0, w, h);

      const barW = w / data.length;
      for (let i = 0; i < data.length; i++) {
        const pct = data[i] / 255;
        const barH = pct * h;
        ctx.fillStyle = `hsl(${160 - pct * 120}, 100%, 50%)`;
        ctx.fillRect(i * barW, h - barH, barW > 1 ? barW - 0.5 : barW, barH);
      }
    };
    draw();
    return () => {
      if (animRef.current) cancelAnimationFrame(animRef.current);
      animRef.current = 0;
    };
  }, [isRunning]);

  const handleStart = async () => {
    setError(null);
    appendLog("calling eq.start()…");
    await eqRef.current?.start();
  };

  const handleStop = () => {
    appendLog("calling eq.stop()");
    eqRef.current?.stop();
  };

  const handleCalibrate = () => {
    appendLog("calling eq.calibrate()");
    eqRef.current?.calibrate();
  };

  const handleMixChange = (key: keyof MixLevels, value: number) => {
    setMix((prev) => {
      const next = { ...prev, [key]: value };
      eqRef.current?.setMix(next);
      return next;
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      {/* Controls card */}
      <div style={{ background: "#16213e", borderRadius: 12, padding: 24 }}>
        {/* Status */}
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 16 }}>
          <div
            style={{
              width: 12,
              height: 12,
              borderRadius: "50%",
              background: STATUS_COLORS[status] ?? "#555",
              boxShadow: isRunning ? `0 0 8px ${STATUS_COLORS[status]}` : "none",
            }}
          />
          <span style={{ fontSize: 13, textTransform: "uppercase", letterSpacing: 1 }}>
            {status}
          </span>
          {error && <span style={{ fontSize: 12, color: "#ff6b6b", marginLeft: 8 }}>{error}</span>}
        </div>

        {/* Spectrum */}
        <canvas
          ref={canvasRef}
          width={512}
          height={100}
          style={{
            display: "block",
            width: "100%",
            height: 100,
            borderRadius: 8,
            background: "#0f0f23",
            marginBottom: 16,
          }}
        />

        {/* Buttons */}
        <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
          <button
            onClick={isRunning ? handleStop : handleStart}
            style={{
              flex: 1,
              padding: "10px 0",
              border: "none",
              borderRadius: 8,
              fontWeight: 700,
              cursor: "pointer",
              background: isRunning ? "#ff6b6b" : "#0dff72",
              color: isRunning ? "#fff" : "#0f0f23",
            }}
          >
            {isRunning ? "Stop" : "Start"}
          </button>
          <button
            onClick={handleCalibrate}
            disabled={!isRunning}
            style={{
              flex: 1,
              padding: "10px 0",
              border: "none",
              borderRadius: 8,
              fontWeight: 700,
              cursor: isRunning ? "pointer" : "not-allowed",
              background: isRunning ? "#f0a500" : "#2a2a4a",
              color: isRunning ? "#0f0f23" : "#666",
            }}
          >
            Calibrate
          </button>
          <button
            onClick={() => eqRef.current?.setBypass(!( status === "bypassed"))}
            disabled={!isRunning}
            style={{
              flex: 1,
              padding: "10px 0",
              border: "none",
              borderRadius: 8,
              fontWeight: 700,
              cursor: isRunning ? "pointer" : "not-allowed",
              background: status === "bypassed" ? "#ff6b6b" : "#2a2a4a",
              color: status === "bypassed" ? "#fff" : "#666",
            }}
          >
            {status === "bypassed" ? "Unbypass" : "Bypass"}
          </button>
        </div>

        {/* Sliders */}
        {(["crowd", "speaker", "music"] as const).map((key) => (
          <div key={key} style={{ display: "flex", alignItems: "center", marginBottom: 8 }}>
            <span style={{ width: 70, fontSize: 13, fontWeight: 600, textTransform: "capitalize" }}>
              {key}
            </span>
            <input
              type="range"
              min={-1}
              max={1}
              step={0.01}
              value={mix[key]}
              onChange={(e) => handleMixChange(key, Number(e.target.value))}
              style={{ flex: 1 }}
            />
            <span style={{ width: 50, textAlign: "right", fontSize: 12, fontVariantNumeric: "tabular-nums" }}>
              {mix[key].toFixed(2)}
            </span>
          </div>
        ))}
        <div style={{ display: "flex", alignItems: "center" }}>
          <span style={{ width: 70, fontSize: 13, fontWeight: 600 }}>Gain</span>
          <input
            type="range"
            min={-20}
            max={20}
            step={0.5}
            value={mix.gainDb}
            onChange={(e) => handleMixChange("gainDb", Number(e.target.value))}
            style={{ flex: 1 }}
          />
          <span style={{ width: 50, textAlign: "right", fontSize: 12, fontVariantNumeric: "tabular-nums" }}>
            {mix.gainDb.toFixed(1)}dB
          </span>
        </div>
      </div>

      {/* Event log */}
      <div
        style={{
          background: "#1a1a2e",
          borderRadius: 12,
          padding: 16,
          fontFamily: "monospace",
          fontSize: 12,
          lineHeight: 1.8,
          maxHeight: 200,
          overflow: "auto",
        }}
      >
        <div style={{ color: "#555", marginBottom: 8, fontWeight: 700 }}>Event Log</div>
        {log.length === 0 ? (
          <div style={{ color: "#444" }}>No events yet…</div>
        ) : (
          log.map((entry, i) => (
            <div key={i} style={{ color: "#888" }}>
              {entry}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
