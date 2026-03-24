"use client";

import dynamic from "next/dynamic";
import { useState } from "react";
import { AudioSourceSelector } from "../components/AudioSourceSelector";
import { AUDIO_SAMPLES } from "../audio-samples";

// StadiumEqualizer uses browser APIs (AudioContext, WASM) — disable SSR
const StadiumEqualizer = dynamic(
  () => import("stadium-eq-react").then((m) => ({ default: m.StadiumEqualizer })),
  { ssr: false }
);

export default function DropInDemo() {
  const [sourceId, setSourceId] = useState("mic");
  const [audioSource, setAudioSource] = useState<HTMLAudioElement | undefined>(
    undefined
  );
  const [key, setKey] = useState(0);

  const handleSourceChange = (id: string) => {
    setSourceId(id);
    // Force remount of StadiumEqualizer when source changes
    setKey((k) => k + 1);

    if (id === "mic") {
      setAudioSource(undefined);
    } else {
      const sample = AUDIO_SAMPLES.find((s) => s.id === id);
      if (sample) {
        const el = new Audio(sample.file);
        el.crossOrigin = "anonymous";
        el.loop = true;
        setAudioSource(el);
      }
    }
  };

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

      <div style={{ marginBottom: 16 }}>
        <AudioSourceSelector value={sourceId} onChange={handleSourceChange} />
      </div>

      <div style={{ background: "#16213e", borderRadius: 12, padding: 24 }}>
        <StadiumEqualizer
          key={key}
          wasmUrl="/stadium_eq.wasm"
          showSpectrum={true}
          audioSource={audioSource}
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
  // Pass an <audio> element to use a sample instead of the mic
  const audio = new Audio("/samples/stadium-announcer.ogg");
  audio.loop = true;

  return (
    <StadiumEqualizer
      wasmUrl="/stadium_eq.wasm"
      showSpectrum={true}
      audioSource={audio}
    />
  );
}`}</code>
        </pre>
      </details>
    </>
  );
}
