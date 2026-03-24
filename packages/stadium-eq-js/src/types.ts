/** Configuration options for creating a StadiumEQ instance. */
export interface StadiumEQOptions {
  /** URL or path to the compiled `stadium_eq.wasm` binary. */
  wasmUrl: string;

  /**
   * Optional URL to a custom AudioWorklet processor script.
   * If omitted, the built-in processor is inlined via a Blob URL.
   */
  workletUrl?: string;

  /** Sample rate in Hz. Defaults to 48000. */
  sampleRate?: number;

  /** Frame size in samples. Defaults to 480 (10ms at 48kHz). */
  frameSize?: number;

  /** Audio input source. Defaults to requesting the microphone. */
  audioSource?: MediaStream | MediaStreamAudioSourceNode | AudioNode;
}

/** Mix levels for the source separation controls. */
export interface MixLevels {
  /** Crowd audio level, -1.0 to 1.0. */
  crowd: number;
  /** Speaker/voice audio level, -1.0 to 1.0. */
  speaker: number;
  /** Music/background audio level, -1.0 to 1.0. */
  music: number;
  /** Overall output gain in decibels. */
  gainDb: number;
}

/** Pipeline status values. */
export type PipelineStatus =
  | "idle"
  | "loading"
  | "calibrating"
  | "processing"
  | "bypassed"
  | "error";

/** Event map for StadiumEQ event listeners. */
export interface StadiumEQEventMap {
  statuschange: PipelineStatus;
  error: string;
  ready: void;
  destroyed: void;
}

/** Frequency data snapshot from the analyser. */
export interface SpectrumData {
  /** Raw byte frequency data (0-255 per bin). */
  frequencyData: Uint8Array;
  /** Number of frequency bins. */
  binCount: number;
}
