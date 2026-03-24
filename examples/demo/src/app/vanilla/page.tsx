"use client";

import dynamic from "next/dynamic";

const VanillaDemo = dynamic(() => import("@/components/VanillaDemo"), {
  ssr: false,
});

export default function VanillaPage() {
  return (
    <>
      <h1 style={{ fontSize: 24, fontWeight: 700, marginBottom: 8 }}>
        Vanilla JS
      </h1>
      <p style={{ color: "#888", fontSize: 14, marginBottom: 24, lineHeight: 1.6 }}>
        No React required. The <code style={{ color: "#0dff72" }}>StadiumEQ</code>{" "}
        class works with any framework — or none at all. Event-driven API with full
        TypeScript types.
      </p>
      <VanillaDemo />

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
          <code>{`import { StadiumEQ } from "stadium-eq";

const eq = new StadiumEQ({ wasmUrl: "/stadium_eq.wasm" });

eq.on("statuschange", (status) => {
  document.getElementById("status")!.textContent = status;
});

eq.on("error", (msg) => console.error("EQ error:", msg));

document.getElementById("start")!.onclick = () => eq.start();
document.getElementById("stop")!.onclick = () => eq.stop();
document.getElementById("calibrate")!.onclick = () => eq.calibrate();

document.getElementById("crowd")!.oninput = (e) => {
  eq.setMix({ crowd: +e.target.value });
};`}</code>
        </pre>
      </details>
    </>
  );
}
