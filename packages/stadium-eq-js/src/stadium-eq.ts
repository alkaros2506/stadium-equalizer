import { WORKLET_SOURCE } from "./worklet-inline.js";
import type {
  StadiumEQOptions,
  MixLevels,
  PipelineStatus,
  StadiumEQEventMap,
  SpectrumData,
} from "./types.js";

type Listener<T> = (value: T) => void;

/**
 * StadiumEQ — main class for the Stadium Audio Equalizer.
 *
 * Handles WASM loading, AudioContext/AudioWorklet setup, and provides
 * a simple API for controlling the DSP pipeline.
 *
 * @example
 * ```js
 * import { StadiumEQ } from "stadium-eq";
 *
 * const eq = new StadiumEQ({ wasmUrl: "/stadium_eq.wasm" });
 * await eq.start();
 * eq.setMix({ crowd: 0.5, speaker: 1.0, music: -0.3, gainDb: 0 });
 * eq.calibrate();
 * ```
 */
export class StadiumEQ {
  private options: Required<
    Pick<StadiumEQOptions, "wasmUrl" | "sampleRate" | "frameSize">
  > &
    Pick<StadiumEQOptions, "workletUrl" | "audioSource">;

  private audioCtx: AudioContext | null = null;
  private workletNode: AudioWorkletNode | null = null;
  private analyserNode: AnalyserNode | null = null;
  private micStream: MediaStream | null = null;
  private sourceNode: AudioNode | null = null;
  private blobUrl: string | null = null;

  private _status: PipelineStatus = "idle";
  private _mix: MixLevels = { crowd: 0, speaker: 0, music: 0, gainDb: 0 };

  private listeners: {
    [K in keyof StadiumEQEventMap]?: Set<Listener<StadiumEQEventMap[K]>>;
  } = {};

  constructor(options: StadiumEQOptions) {
    this.options = {
      wasmUrl: options.wasmUrl,
      workletUrl: options.workletUrl,
      audioSource: options.audioSource,
      sampleRate: options.sampleRate ?? 48000,
      frameSize: options.frameSize ?? 480,
    };
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  /** Start the audio pipeline. Requests microphone access if no audioSource was provided. */
  async start(): Promise<void> {
    if (this._status !== "idle") return;
    this.setStatus("loading");

    try {
      // 1. Compile WASM
      const response = await fetch(this.options.wasmUrl);
      const bytes = await response.arrayBuffer();
      const wasmModule = await WebAssembly.compile(bytes);

      // 2. Create AudioContext
      this.audioCtx = new AudioContext({
        sampleRate: this.options.sampleRate,
      });

      // 3. Register the AudioWorklet
      const workletUrl = this.options.workletUrl ?? this.createBlobWorkletUrl();
      await this.audioCtx.audioWorklet.addModule(workletUrl);

      // 4. Create worklet node
      this.workletNode = new AudioWorkletNode(
        this.audioCtx,
        "stadium-eq-processor",
        {
          numberOfInputs: 1,
          numberOfOutputs: 1,
          channelCount: 1,
          channelCountMode: "explicit",
        }
      );

      // 5. Listen for messages from the worklet
      this.workletNode.port.onmessage = (ev: MessageEvent) => {
        const msg = ev.data;
        if (msg.type === "ready") {
          this.setStatus("processing");
          this.emit("ready", undefined as never);
        } else if (msg.type === "status") {
          this.setStatus(msg.status);
        } else if (msg.type === "error") {
          this.setStatus("error");
          this.emit("error", msg.error);
        }
      };

      // 6. Send WASM module to worklet
      this.workletNode.port.postMessage({
        type: "init-wasm",
        module: wasmModule,
      });

      // 7. Connect audio source
      this.sourceNode = await this.resolveAudioSource(this.audioCtx);

      // 8. Create analyser
      this.analyserNode = this.audioCtx.createAnalyser();
      this.analyserNode.fftSize = 256;
      this.analyserNode.smoothingTimeConstant = 0.7;

      // 9. Wire: source -> worklet -> analyser -> destination
      this.sourceNode.connect(this.workletNode);
      this.workletNode.connect(this.analyserNode);
      this.analyserNode.connect(this.audioCtx.destination);
    } catch (err) {
      this.setStatus("error");
      this.emit("error", String(err));
      throw err;
    }
  }

  /** Stop the audio pipeline and release all resources. */
  stop(): void {
    if (this._status === "idle") return;

    this.workletNode?.disconnect();
    this.workletNode = null;

    this.sourceNode?.disconnect();
    this.sourceNode = null;

    if (this.micStream) {
      this.micStream.getTracks().forEach((t) => t.stop());
      this.micStream = null;
    }

    this.analyserNode?.disconnect();
    this.analyserNode = null;

    if (this.audioCtx) {
      this.audioCtx.close();
      this.audioCtx = null;
    }

    if (this.blobUrl) {
      URL.revokeObjectURL(this.blobUrl);
      this.blobUrl = null;
    }

    this.setStatus("idle");
  }

  /** Stop and clean up. Fires the `destroyed` event. */
  destroy(): void {
    this.stop();
    this.emit("destroyed", undefined as never);
    this.listeners = {};
  }

  // ---------------------------------------------------------------------------
  // Controls
  // ---------------------------------------------------------------------------

  /** Start calibration (noise profiling). */
  calibrate(): void {
    this.workletNode?.port.postMessage({ type: "calibrate" });
  }

  /** Set the source separation mix levels. */
  setMix(mix: Partial<MixLevels>): void {
    this._mix = { ...this._mix, ...mix };
    this.workletNode?.port.postMessage({
      type: "set-mix",
      ...this._mix,
    });
  }

  /** Enable or disable bypass mode. */
  setBypass(bypass: boolean): void {
    this.workletNode?.port.postMessage({ type: "set-bypass", bypass });
  }

  // ---------------------------------------------------------------------------
  // State getters
  // ---------------------------------------------------------------------------

  /** Current pipeline status. */
  get status(): PipelineStatus {
    return this._status;
  }

  /** Current mix levels. */
  get mix(): Readonly<MixLevels> {
    return this._mix;
  }

  /** Whether the pipeline is actively running. */
  get isRunning(): boolean {
    return (
      this._status === "processing" ||
      this._status === "calibrating" ||
      this._status === "bypassed"
    );
  }

  /** The underlying AudioContext, or null if not started. */
  get context(): AudioContext | null {
    return this.audioCtx;
  }

  /** The AnalyserNode for spectrum visualization, or null if not started. */
  get analyser(): AnalyserNode | null {
    return this.analyserNode;
  }

  /**
   * Get the current spectrum data for visualization.
   * Returns null if the pipeline is not running.
   */
  getSpectrumData(): SpectrumData | null {
    if (!this.analyserNode) return null;
    const frequencyData = new Uint8Array(this.analyserNode.frequencyBinCount);
    this.analyserNode.getByteFrequencyData(frequencyData);
    return { frequencyData, binCount: this.analyserNode.frequencyBinCount };
  }

  // ---------------------------------------------------------------------------
  // Events
  // ---------------------------------------------------------------------------

  /** Subscribe to an event. Returns an unsubscribe function. */
  on<K extends keyof StadiumEQEventMap>(
    event: K,
    listener: Listener<StadiumEQEventMap[K]>
  ): () => void {
    if (!this.listeners[event]) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (this.listeners as any)[event] = new Set();
    }
    (this.listeners[event] as Set<Listener<StadiumEQEventMap[K]>>).add(
      listener
    );
    return () => this.off(event, listener);
  }

  /** Unsubscribe from an event. */
  off<K extends keyof StadiumEQEventMap>(
    event: K,
    listener: Listener<StadiumEQEventMap[K]>
  ): void {
    (this.listeners[event] as Set<Listener<StadiumEQEventMap[K]>> | undefined)?.delete(
      listener
    );
  }

  // ---------------------------------------------------------------------------
  // Internals
  // ---------------------------------------------------------------------------

  private emit<K extends keyof StadiumEQEventMap>(
    event: K,
    value: StadiumEQEventMap[K]
  ): void {
    const set = this.listeners[event] as
      | Set<Listener<StadiumEQEventMap[K]>>
      | undefined;
    set?.forEach((fn) => fn(value));
  }

  private setStatus(status: PipelineStatus): void {
    if (this._status === status) return;
    this._status = status;
    this.emit("statuschange", status);
  }

  private createBlobWorkletUrl(): string {
    const blob = new Blob([WORKLET_SOURCE], { type: "application/javascript" });
    this.blobUrl = URL.createObjectURL(blob);
    return this.blobUrl;
  }

  private async resolveAudioSource(
    ctx: AudioContext
  ): Promise<AudioNode> {
    const src = this.options.audioSource;

    if (src instanceof AudioNode) {
      return src;
    }

    // If a MediaStream was provided, wrap it.
    if (src instanceof MediaStream) {
      return ctx.createMediaStreamSource(src);
    }

    // Default: request microphone.
    this.micStream = await navigator.mediaDevices.getUserMedia({
      audio: {
        sampleRate: this.options.sampleRate,
        channelCount: 1,
        echoCancellation: false,
        noiseSuppression: false,
        autoGainControl: false,
      },
    });
    return ctx.createMediaStreamSource(this.micStream);
  }
}
