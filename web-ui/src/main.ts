// Main thread entry point for the Stadium Audio Equalizer web UI.
//
// Loads the WASM binary, sets up AudioContext + AudioWorklet, wires the
// selected audio source (microphone or sample file) through the worklet to
// speakers, and connects UI controls.

import { initUI, setWorkletNode, setAnalyserNode, setStatusText } from "./ui.ts";
import { AUDIO_SAMPLES } from "./samples.ts";

const WASM_URL = "stadium_eq.wasm";
const WORKLET_URL = "src/worklet-processor.ts";
const SAMPLE_RATE = 48000;

let audioCtx: AudioContext | null = null;
let workletNode: AudioWorkletNode | null = null;
let micStream: MediaStream | null = null;
let sampleAudioEl: HTMLAudioElement | null = null;
let sourceNode: AudioNode | null = null;
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

/** Get the currently selected audio source id from the dropdown. */
function getSelectedSource(): string {
  const select = document.getElementById("audio-source") as HTMLSelectElement;
  return select.value;
}

/** Create the audio source node based on the current dropdown selection. */
async function createAudioSource(ctx: AudioContext): Promise<AudioNode> {
  const sourceId = getSelectedSource();

  if (sourceId === "mic") {
    micStream = await navigator.mediaDevices.getUserMedia({
      audio: {
        sampleRate: SAMPLE_RATE,
        channelCount: 1,
        echoCancellation: false,
        noiseSuppression: false,
        autoGainControl: false,
      },
    });
    return ctx.createMediaStreamSource(micStream);
  }

  // Sample file source.
  const sample = AUDIO_SAMPLES.find((s) => s.id === sourceId);
  if (!sample) {
    throw new Error(`Unknown audio source: ${sourceId}`);
  }

  sampleAudioEl = new Audio(sample.file);
  sampleAudioEl.crossOrigin = "anonymous";
  sampleAudioEl.loop = true;

  // Wait for enough data to start playback.
  await new Promise<void>((resolve, reject) => {
    sampleAudioEl!.addEventListener("canplaythrough", () => resolve(), { once: true });
    sampleAudioEl!.addEventListener("error", () => {
      reject(new Error(`Failed to load sample: ${sample.file}`));
    }, { once: true });
    sampleAudioEl!.load();
  });

  const node = ctx.createMediaElementSource(sampleAudioEl);
  sampleAudioEl.play();
  return node;
}

/** Start the audio pipeline: source -> worklet -> destination. */
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

  // 6. Get the audio source (mic or sample file).
  sourceNode = await createAudioSource(audioCtx);

  // 7. AnalyserNode for spectrum visualisation.
  const analyser = audioCtx.createAnalyser();
  analyser.fftSize = 256;
  analyser.smoothingTimeConstant = 0.7;

  // Connect: source -> worklet -> analyser -> destination.
  sourceNode.connect(workletNode);
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

  if (sourceNode) {
    sourceNode.disconnect();
    sourceNode = null;
  }

  if (micStream) {
    micStream.getTracks().forEach((t) => t.stop());
    micStream = null;
  }

  if (sampleAudioEl) {
    sampleAudioEl.pause();
    sampleAudioEl.src = "";
    sampleAudioEl = null;
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

  // Populate sample options in the dropdown.
  const sampleGroup = document.getElementById("sample-options") as HTMLOptGroupElement;
  const descriptionEl = document.getElementById("source-description") as HTMLElement;
  const audioSourceSelect = document.getElementById("audio-source") as HTMLSelectElement;

  for (const sample of AUDIO_SAMPLES) {
    const option = document.createElement("option");
    option.value = sample.id;
    option.textContent = sample.label;
    sampleGroup.appendChild(option);
  }

  // Show description when a sample is selected.
  audioSourceSelect.addEventListener("change", () => {
    const sourceId = audioSourceSelect.value;
    const sample = AUDIO_SAMPLES.find((s) => s.id === sourceId);
    descriptionEl.textContent = sample ? sample.description : "";
  });

  const btnStart = document.getElementById("btn-start") as HTMLButtonElement;
  const btnCalibrate = document.getElementById("btn-calibrate") as HTMLButtonElement;

  btnStart.addEventListener("click", async () => {
    if (!running) {
      btnStart.textContent = "Stop";
      btnStart.classList.add("active");
      btnCalibrate.disabled = false;
      // Disable source selector while running.
      audioSourceSelect.disabled = true;
      try {
        await start();
      } catch (err) {
        console.error("Failed to start:", err);
        setStatusText("idle", `Error: ${err}`);
        btnStart.textContent = "Start";
        btnStart.classList.remove("active");
        btnCalibrate.disabled = true;
        audioSourceSelect.disabled = false;
        running = false;
      }
    } else {
      stop();
      btnStart.textContent = "Start";
      btnStart.classList.remove("active");
      btnCalibrate.disabled = true;
      audioSourceSelect.disabled = false;
    }
  });

  btnCalibrate.addEventListener("click", () => {
    if (workletNode) {
      workletNode.port.postMessage({ type: "calibrate" });
      setStatusText("calibrating", "Calibrating...");
    }
  });
});
