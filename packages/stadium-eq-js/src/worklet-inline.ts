/**
 * Inline source for the AudioWorklet processor.
 * This is stringified and loaded via a Blob URL so users don't need
 * to host a separate worklet file.
 */
export const WORKLET_SOURCE = /* js */ `
class RingBuffer {
  constructor(capacity) {
    this.buf = new Float32Array(capacity);
    this.readIdx = 0;
    this.writeIdx = 0;
    this.count = 0;
  }

  get available() {
    return this.count;
  }

  push(data) {
    for (let i = 0; i < data.length; i++) {
      this.buf[this.writeIdx] = data[i];
      this.writeIdx = (this.writeIdx + 1) % this.buf.length;
    }
    this.count = Math.min(this.count + data.length, this.buf.length);
  }

  pull(out) {
    const n = Math.min(out.length, this.count);
    for (let i = 0; i < n; i++) {
      out[i] = this.buf[this.readIdx];
      this.readIdx = (this.readIdx + 1) % this.buf.length;
    }
    this.count -= n;
    for (let i = n; i < out.length; i++) {
      out[i] = 0;
    }
  }
}

const FRAME_SIZE = 480;
const RING_CAPACITY = 960;

class StadiumEqProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.wasm = null;
    this.ctx = 0;
    this.wasmMemory = null;
    this.inPtr = 0;
    this.outPtr = 0;
    this.inputRing = new RingBuffer(RING_CAPACITY);
    this.outputRing = new RingBuffer(RING_CAPACITY);
    this.ready = false;

    this.port.onmessage = (ev) => this.onMessage(ev);
  }

  async onMessage(ev) {
    const msg = ev.data;

    switch (msg.type) {
      case "init-wasm": {
        try {
          const module = msg.module;
          this.wasm = await WebAssembly.instantiate(module, {});

          const exports = this.wasm.exports;
          this.wasmMemory = exports.memory;

          this.wasmAlloc = exports.stadium_eq_alloc;
          this.wasmDealloc = exports.stadium_eq_dealloc;
          this.wasmProcess = exports.stadium_eq_process;
          this.wasmCalibrate = exports.stadium_eq_start_calibration;
          this.wasmSetMix = exports.stadium_eq_set_mix;
          this.wasmSetBypass = exports.stadium_eq_set_bypass;

          this.inPtr = this.wasmAlloc(FRAME_SIZE);
          this.outPtr = this.wasmAlloc(FRAME_SIZE);

          const rate = typeof sampleRate !== "undefined" ? sampleRate : 48000;
          this.ctx = exports.stadium_eq_init(rate, FRAME_SIZE);

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

  process(inputs, outputs) {
    const input = inputs[0] && inputs[0][0];
    const output = outputs[0] && outputs[0][0];

    if (!this.ready || !input || !output) {
      if (output) output.fill(0);
      return true;
    }

    this.inputRing.push(input);

    while (this.inputRing.available >= FRAME_SIZE) {
      const mem = new Float32Array(
        this.wasmMemory.buffer,
        this.inPtr * 4,
        FRAME_SIZE
      );
      const tmp = new Float32Array(FRAME_SIZE);
      this.inputRing.pull(tmp);
      mem.set(tmp);

      const written = this.wasmProcess(this.ctx, this.inPtr, this.outPtr, FRAME_SIZE);

      const outMem = new Float32Array(
        this.wasmMemory.buffer,
        this.outPtr * 4,
        written
      );
      this.outputRing.push(new Float32Array(outMem));
    }

    this.outputRing.pull(output);
    return true;
  }
}

registerProcessor("stadium-eq-processor", StadiumEqProcessor);
`;
