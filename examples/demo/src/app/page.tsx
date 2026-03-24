"use client";

import dynamic from "next/dynamic";

// StadiumEqualizer uses browser APIs (AudioContext, WASM) — disable SSR
const StadiumEqualizer = dynamic(
  () => import("stadium-eq-react").then((m) => ({ default: m.StadiumEqualizer })),
  { ssr: false }
);

export default function DropInDemo() {
  return (
    <>
      <h1 style={{ fontSize: 24, fontWeight: 700, marginBottom: 8 }}>
        Drop-in Component
      </h1>
      <p style={{ color: "#888", fontSize: 14, marginBottom: 24, lineHeight: 1.6 }}>
        One import, one line. The <code style={{ color: "#0dff72" }}>&lt;StadiumEqualizer&gt;</code>{" "}
        component renders a full EQ UI with spectrum visualizer, sliders, and status
        indicator — ready to go.
      </p>

      <div style={{ background: "#16213e", borderRadius: 12, padding: 24 }}>
        <StadiumEqualizer
          wasmUrl="/stadium_eq.wasm"
          showSpectrum={true}
        />
      </div>

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
          <code>{`import { StadiumEqualizer } from "stadium-eq-react";

function App() {
  return (
    <StadiumEqualizer
      wasmUrl="/stadium_eq.wasm"
      showSpectrum={true}
    />
  );
}`}</code>
        </pre>
      </details>
    </>
  );
}
