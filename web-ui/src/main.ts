// Main thread entry point for the Stadium Audio Equalizer web UI.
//
// Loads the WASM binary, sets up AudioContext + AudioWorklet, wires mic input
// through the worklet to speakers, and connects UI controls.

import { initUI, setWorkletNode, setAnalyserNode, setStatusText } from "./ui.ts";

const WASM_URL = "stadium_eq.wasm";
const WORKLET_URL = "src/worklet-processor.ts";
const SAMPLE_RATE = 48000;

let audioCtx: AudioContext | null = null;
let workletNode: AudioWorkletNode | null = null;
let micStream: MediaStream | null = null;
let running = false;

/** Compile the WASM module on the main thread so we can transfer it. */
async function loadWasmModule(): Promise<WebAssembly.Module> {
  const response = await fetch(WASM_URL);
  if (!response.ok) {
    throw new Error(
      `Failed to fetch WASM from ${WASM_URL}: ${response.status} ${response.statusText}`
    );
  }
  const bytes = await response.arrayBuffer();
  return WebAssembly.compile(bytes);
}

/** Start the audio pipeline: mic -> worklet -> destination. */
async function start(): Promise<void> {
  if (running) return;

  setStatusText("loading", "Loading...");

  // 1. Load & compile WASM.
  const wasmModule = await loadWasmModule();

  // 2. Create AudioContext at desired sample rate.
  audioCtx = new AudioContext({ sampleRate: SAMPLE_RATE });

  // 3. Register the AudioWorklet module.
  await audioCtx.addModule(WORKLET_URL);

  // 4. Create the worklet node (mono in, mono out).
  workletNode = new AudioWorkletNode(audioCtx, "stadium-eq-processor", {
    numberOfInputs: 1,
    numberOfOutputs: 1,
    channelCount: 1,
    channelCountMode: "explicit",
  });

  // Listen for status updates from the worklet.
  workletNode.port.onmessage = (ev: MessageEvent) => {
    const msg = ev.data;
    if (msg.type === "ready") {
      setStatusText("processing", "Processing");
    } else if (msg.type === "status") {
      setStatusText(msg.status, capitalize(msg.status));
    } else if (msg.type === "error") {
      setStatusText("idle", `Error: ${msg.error}`);
    }
  };

  // 5. Send the compiled WASM module to the worklet.
  workletNode.port.postMessage({ type: "init-wasm", module: wasmModule });

  // 6. Get microphone input.
  micStream = await navigator.mediaDevices.getUserMedia({
    audio: {
      sampleRate: SAMPLE_RATE,
      channelCount: 1,
      echoCancellation: false,
      noiseSuppression: false,
      autoGainControl: false,
    },
  });

  const micSource = audioCtx.createMediaStreamSource(micStream);

  // 7. AnalyserNode for spectrum visualisation.
  const analyser = audioCtx.createAnalyser();
  analyser.fftSize = 256;
  analyser.smoothingTimeConstant = 0.7;

  // Connect: mic -> worklet -> analyser -> destination.
  micSource.connect(workletNode);
  workletNode.connect(analyser);
  analyser.connect(audioCtx.destination);

  // 8. Expose to UI layer.
  setWorkletNode(workletNode);
  setAnalyserNode(analyser);

  running = true;
}

/** Stop the audio pipeline. */
function stop(): void {
  if (!running) return;

  if (workletNode) {
    workletNode.disconnect();
    workletNode = null;
  }

  if (micStream) {
    micStream.getTracks().forEach((t) => t.stop());
    micStream = null;
  }

  if (audioCtx) {
    audioCtx.close();
    audioCtx = null;
  }

  setWorkletNode(null);
  setAnalyserNode(null);
  setStatusText("idle", "Idle");
  running = false;
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

document.addEventListener("DOMContentLoaded", () => {
  initUI();

  const btnStart = document.getElementById("btn-start") as HTMLButtonElement;
  const btnCalibrate = document.getElementById("btn-calibrate") as HTMLButtonElement;

  btnStart.addEventListener("click", async () => {
    if (!running) {
      btnStart.textContent = "Stop";
      btnStart.classList.add("active");
      btnCalibrate.disabled = false;
      try {
        await start();
      } catch (err) {
        console.error("Failed to start:", err);
        setStatusText("idle", `Error: ${err}`);
        btnStart.textContent = "Start";
        btnStart.classList.remove("active");
        btnCalibrate.disabled = true;
        running = false;
      }
    } else {
      stop();
      btnStart.textContent = "Start";
      btnStart.classList.remove("active");
      btnCalibrate.disabled = true;
    }
  });

  btnCalibrate.addEventListener("click", () => {
    if (workletNode) {
      workletNode.port.postMessage({ type: "calibrate" });
      setStatusText("calibrating", "Calibrating...");
    }
  });
});
