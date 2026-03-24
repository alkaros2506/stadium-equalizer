// UI controller for the Stadium Audio Equalizer.
//
// Binds slider and toggle events, updates the status display, draws a spectrum
// visualisation on a canvas, and forwards mix/calibrate/bypass messages to
// the AudioWorkletNode.

// ---------------------------------------------------------------------------
// Shared references set from main.ts after audio graph is built.
// ---------------------------------------------------------------------------

let workletNode: AudioWorkletNode | null = null;
let analyserNode: AnalyserNode | null = null;

let animFrameId: number = 0;

// ---------------------------------------------------------------------------
// Public setters called from main.ts
// ---------------------------------------------------------------------------

export function setWorkletNode(node: AudioWorkletNode | null): void {
  workletNode = node;
}

export function setAnalyserNode(node: AnalyserNode | null): void {
  analyserNode = node;

  if (analyserNode) {
    startSpectrumLoop();
  } else {
    stopSpectrumLoop();
  }
}

// ---------------------------------------------------------------------------
// Status display
// ---------------------------------------------------------------------------

export function setStatusText(
  state: string,
  label: string,
): void {
  const dot = document.getElementById("status-dot");
  const text = document.getElementById("status-text");

  if (dot) {
    // Remove all state classes, then add the current one.
    dot.classList.remove("idle", "calibrating", "processing", "bypassed", "loading");
    dot.classList.add(state);
  }

  if (text) {
    text.textContent = label;
  }
}

// ---------------------------------------------------------------------------
// Slider helpers
// ---------------------------------------------------------------------------

function readSliderValues(): {
  crowd: number;
  speaker: number;
  music: number;
  gainDb: number;
} {
  const crowd = parseFloat(
    (document.getElementById("slider-crowd") as HTMLInputElement).value,
  );
  const speaker = parseFloat(
    (document.getElementById("slider-speaker") as HTMLInputElement).value,
  );
  const music = parseFloat(
    (document.getElementById("slider-music") as HTMLInputElement).value,
  );
  const gainDb = parseFloat(
    (document.getElementById("slider-gain") as HTMLInputElement).value,
  );
  return { crowd, speaker, music, gainDb };
}

function sendMix(): void {
  if (!workletNode) return;
  const mix = readSliderValues();
  workletNode.port.postMessage({ type: "set-mix", ...mix });
}

function formatLevel(v: number): string {
  return v.toFixed(2);
}

function formatGain(v: number): string {
  return `${v.toFixed(1)} dB`;
}

// ---------------------------------------------------------------------------
// Spectrum visualisation
// ---------------------------------------------------------------------------

let spectrumCanvas: HTMLCanvasElement | null = null;
let spectrumCtx: CanvasRenderingContext2D | null = null;
let freqData: Uint8Array | null = null;

function startSpectrumLoop(): void {
  if (animFrameId) return;
  drawSpectrum();
}

function stopSpectrumLoop(): void {
  if (animFrameId) {
    cancelAnimationFrame(animFrameId);
    animFrameId = 0;
  }

  // Clear the canvas to black when stopped.
  if (spectrumCtx && spectrumCanvas) {
    spectrumCtx.fillStyle = "#0f0f23";
    spectrumCtx.fillRect(0, 0, spectrumCanvas.width, spectrumCanvas.height);
  }
}

function drawSpectrum(): void {
  animFrameId = requestAnimationFrame(drawSpectrum);

  if (!analyserNode || !spectrumCanvas || !spectrumCtx) return;

  // Lazily allocate the frequency data buffer.
  if (!freqData || freqData.length !== analyserNode.frequencyBinCount) {
    freqData = new Uint8Array(analyserNode.frequencyBinCount);
  }

  analyserNode.getByteFrequencyData(freqData);

  const w = spectrumCanvas.width;
  const h = spectrumCanvas.height;
  const binCount = freqData.length;

  // Background.
  spectrumCtx.fillStyle = "#0f0f23";
  spectrumCtx.fillRect(0, 0, w, h);

  // Bar width so all bins fit.
  const barWidth = w / binCount;

  for (let i = 0; i < binCount; i++) {
    const value = freqData[i]; // 0-255
    const pct = value / 255;
    const barHeight = pct * h;

    // Colour gradient: green -> yellow -> red.
    const r = Math.min(255, Math.floor(pct * 2 * 255));
    const g = Math.min(255, Math.floor((1 - pct) * 2 * 255));
    spectrumCtx.fillStyle = `rgb(${r}, ${g}, 40)`;

    const x = i * barWidth;
    spectrumCtx.fillRect(x, h - barHeight, barWidth > 1 ? barWidth - 0.5 : barWidth, barHeight);
  }
}

// ---------------------------------------------------------------------------
// Initialisation — called once from main.ts on DOMContentLoaded.
// ---------------------------------------------------------------------------

export function initUI(): void {
  // Grab canvas.
  spectrumCanvas = document.getElementById("spectrum-canvas") as HTMLCanvasElement;
  if (spectrumCanvas) {
    spectrumCtx = spectrumCanvas.getContext("2d");
  }

  // Bind mix sliders.
  const sliderCrowd = document.getElementById("slider-crowd") as HTMLInputElement;
  const sliderSpeaker = document.getElementById("slider-speaker") as HTMLInputElement;
  const sliderMusic = document.getElementById("slider-music") as HTMLInputElement;
  const sliderGain = document.getElementById("slider-gain") as HTMLInputElement;

  const valCrowd = document.getElementById("val-crowd") as HTMLElement;
  const valSpeaker = document.getElementById("val-speaker") as HTMLElement;
  const valMusic = document.getElementById("val-music") as HTMLElement;
  const valGain = document.getElementById("val-gain") as HTMLElement;

  if (sliderCrowd) {
    sliderCrowd.addEventListener("input", () => {
      valCrowd.textContent = formatLevel(parseFloat(sliderCrowd.value));
      sendMix();
    });
  }

  if (sliderSpeaker) {
    sliderSpeaker.addEventListener("input", () => {
      valSpeaker.textContent = formatLevel(parseFloat(sliderSpeaker.value));
      sendMix();
    });
  }

  if (sliderMusic) {
    sliderMusic.addEventListener("input", () => {
      valMusic.textContent = formatLevel(parseFloat(sliderMusic.value));
      sendMix();
    });
  }

  if (sliderGain) {
    sliderGain.addEventListener("input", () => {
      valGain.textContent = formatGain(parseFloat(sliderGain.value));
      sendMix();
    });
  }

  // Bind bypass toggle.
  const toggleBypass = document.getElementById("toggle-bypass") as HTMLInputElement;
  if (toggleBypass) {
    toggleBypass.addEventListener("change", () => {
      if (!workletNode) return;
      workletNode.port.postMessage({
        type: "set-bypass",
        bypass: toggleBypass.checked,
      });
    });
  }
}
