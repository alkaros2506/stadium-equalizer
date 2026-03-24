// AudioWorkletProcessor for the Stadium EQ pipeline.
//
// Receives a compiled WASM module via MessagePort, instantiates it inside the
// worklet scope, and drives the C-ABI exported functions each render quantum.

/** Ring buffer for accumulating / draining f32 audio samples. */
class RingBuffer {
  private buf: Float32Array;
  private readIdx: number = 0;
  private writeIdx: number = 0;
  private count: number = 0;

  constructor(capacity: number) {
    this.buf = new Float32Array(capacity);
  }

  get available(): number {
    return this.count;
  }

  push(data: Float32Array): void {
    for (let i = 0; i < data.length; i++) {
      this.buf[this.writeIdx] = data[i];
      this.writeIdx = (this.writeIdx + 1) % this.buf.length;
    }
    this.count = Math.min(this.count + data.length, this.buf.length);
  }

  pull(out: Float32Array): void {
    const n = Math.min(out.length, this.count);
    for (let i = 0; i < n; i++) {
      out[i] = this.buf[this.readIdx];
      this.readIdx = (this.readIdx + 1) % this.buf.length;
    }
    this.count -= n;
    // Zero-fill remainder if we ran short.
    for (let i = n; i < out.length; i++) {
      out[i] = 0;
    }
  }
}

// Worklet-scope types (AudioWorkletProcessor is available globally in worklet).
declare class AudioWorkletProcessor {
  readonly port: MessagePort;
  constructor();
}
declare function registerProcessor(name: string, cls: any): void;
declare const sampleRate: number;

const FRAME_SIZE = 480;       // 10 ms at 48 kHz
const RING_CAPACITY = 960;    // Two frames of headroom
const QUANTUM = 128;          // Web Audio render quantum

class StadiumEqProcessor extends AudioWorkletProcessor {
  private wasm: WebAssembly.Instance | null = null;
  private ctx: number = 0; // pointer to Pipeline

  // WASM memory helpers (resolved after instantiation)
  private wasmMemory: WebAssembly.Memory | null = null;
  private wasmAlloc!: (size: number) => number;
  private wasmDealloc!: (ptr: number, size: number) => void;
  private wasmProcess!: (ctx: number, inp: number, out: number, len: number) => number;
  private wasmCalibrate!: (ctx: number) => void;
  private wasmSetMix!: (ctx: number, c: number, s: number, m: number, g: number) => void;
  private wasmSetBypass!: (ctx: number, b: number) => void;

  // WASM-side buffer pointers (f32 count, not bytes)
  private inPtr: number = 0;
  private outPtr: number = 0;

  private inputRing: RingBuffer = new RingBuffer(RING_CAPACITY);
  private outputRing: RingBuffer = new RingBuffer(RING_CAPACITY);

  private ready: boolean = false;

  constructor() {
    super();
    this.port.onmessage = (ev: MessageEvent) => this.onMessage(ev);
  }

  private async onMessage(ev: MessageEvent): Promise<void> {
    const msg = ev.data;

    switch (msg.type) {
      case "init-wasm": {
        try {
          const module: WebAssembly.Module = msg.module;
          this.wasm = await WebAssembly.instantiate(module, {});

          const exports = this.wasm.exports as any;
          this.wasmMemory = exports.memory as WebAssembly.Memory;

          this.wasmAlloc = exports.stadium_eq_alloc;
          this.wasmDealloc = exports.stadium_eq_dealloc;
          this.wasmProcess = exports.stadium_eq_process;
          this.wasmCalibrate = exports.stadium_eq_start_calibration;
          this.wasmSetMix = exports.stadium_eq_set_mix;
          this.wasmSetBypass = exports.stadium_eq_set_bypass;

          // Allocate WASM-side input/output buffers (FRAME_SIZE f32s each).
          this.inPtr = this.wasmAlloc(FRAME_SIZE);
          this.outPtr = this.wasmAlloc(FRAME_SIZE);

          // Initialise the pipeline (48 kHz, FRAME_SIZE).
          const rate = typeof sampleRate !== "undefined" ? sampleRate : 48000;
          this.ctx = (exports.stadium_eq_init as Function)(rate, FRAME_SIZE);

          this.ready = true;
          this.port.postMessage({ type: "ready" });
        } catch (err) {
          this.port.postMessage({ type: "error", error: String(err) });
        }
        break;
      }

      case "calibrate": {
        if (this.ready) {
          this.wasmCalibrate(this.ctx);
          this.port.postMessage({ type: "status", status: "calibrating" });
        }
        break;
      }

      case "set-mix": {
        if (this.ready) {
          const { crowd, speaker, music, gainDb } = msg;
          this.wasmSetMix(this.ctx, crowd, speaker, music, gainDb);
        }
        break;
      }

      case "set-bypass": {
        if (this.ready) {
          this.wasmSetBypass(this.ctx, msg.bypass ? 1 : 0);
          this.port.postMessage({
            type: "status",
            status: msg.bypass ? "bypassed" : "processing",
          });
        }
        break;
      }
    }
  }

  process(
    inputs: Float32Array[][],
    outputs: Float32Array[][],
    _params: Record<string, Float32Array>,
  ): boolean {
    const input = inputs[0]?.[0]; // mono channel 0
    const output = outputs[0]?.[0];

    if (!this.ready || !input || !output) {
      // Pass silence while not ready.
      if (output) output.fill(0);
      return true;
    }

    // Push incoming quantum into the input ring buffer.
    this.inputRing.push(input);

    // When we have at least one full frame, run the WASM pipeline.
    while (this.inputRing.available >= FRAME_SIZE) {
      // Pull a frame from the input ring into WASM memory.
      const mem = new Float32Array(
        (this.wasmMemory as WebAssembly.Memory).buffer,
        this.inPtr * 4, // byte offset (inPtr is in f32 units)
        FRAME_SIZE,
      );

      const tmp = new Float32Array(FRAME_SIZE);
      this.inputRing.pull(tmp);
      mem.set(tmp);

      // Process.
      const written = this.wasmProcess(this.ctx, this.inPtr, this.outPtr, FRAME_SIZE);

      // Read output frame from WASM memory into the output ring.
      const outMem = new Float32Array(
        (this.wasmMemory as WebAssembly.Memory).buffer,
        this.outPtr * 4,
        written,
      );
      this.outputRing.push(new Float32Array(outMem));
    }

    // Drain QUANTUM samples from the output ring into the real output.
    this.outputRing.pull(output);

    return true;
  }
}

registerProcessor("stadium-eq-processor", StadiumEqProcessor);
