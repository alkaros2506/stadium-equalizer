"use client";

import dynamic from "next/dynamic";

// The actual component — loaded client-side only
const CustomEQDemo = dynamic(() => import("@/components/CustomEQDemo"), {
  ssr: false,
});

export default function CustomPage() {
  return (
    <>
      <h1 style={{ fontSize: 24, fontWeight: 700, marginBottom: 8 }}>
        Custom Hook UI
      </h1>
      <p style={{ color: "#888", fontSize: 14, marginBottom: 24, lineHeight: 1.6 }}>
        Use the <code style={{ color: "#0dff72" }}>useStadiumEQ</code> hook to build
        your own UI. Full control over layout, styling, and behavior — the hook manages
        the WASM lifecycle and gives you reactive state.
      </p>
      <CustomEQDemo />

      <details style={{ marginTop: 32, color: "#888", fontSize: 13 }}>
        <summary style={{ cursor: "pointer", fontWeight: 600, marginBottom: 8 }}>
          View code
        </summary>
        <pre
          style={{
            background: "#1a1a2e",
            padding: 16,
            borderRadius: 8,
            overflow: "auto",
            lineHeight: 1.6,
          }}
        >
          <code>{`import { useStadiumEQ } from "stadium-eq-react";

function MyEQ() {
  const eq = useStadiumEQ({ wasmUrl: "/stadium_eq.wasm" });

  return (
    <div>
      <p>Status: {eq.status}</p>
      <button onClick={eq.isRunning ? eq.stop : eq.start}>
        {eq.isRunning ? "Stop" : "Start"}
      </button>
      <button onClick={eq.calibrate} disabled={!eq.isRunning}>
        Calibrate
      </button>
      <label>
        Crowd
        <input
          type="range" min={-1} max={1} step={0.01}
          value={eq.mix.crowd}
          onChange={e => eq.setMix({ crowd: +e.target.value })}
        />
      </label>
    </div>
  );
}`}</code>
        </pre>
      </details>
    </>
  );
}
