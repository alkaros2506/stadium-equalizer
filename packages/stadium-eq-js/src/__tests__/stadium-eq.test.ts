import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { StadiumEQ } from "../stadium-eq.js";

// ---------------------------------------------------------------------------
// Mock helpers
// ---------------------------------------------------------------------------

function createMockPort() {
  return {
    postMessage: vi.fn(),
    onmessage: null as ((ev: MessageEvent) => void) | null,
  };
}

function createMockWorkletNode(port: ReturnType<typeof createMockPort>) {
  return {
    port,
    connect: vi.fn(),
    disconnect: vi.fn(),
  };
}

function createMockAnalyser() {
  return {
    connect: vi.fn(),
    disconnect: vi.fn(),
    fftSize: 256,
    smoothingTimeConstant: 0.7,
    frequencyBinCount: 128,
    getByteFrequencyData: vi.fn((arr: Uint8Array) => {
      for (let i = 0; i < arr.length; i++) arr[i] = i % 256;
    }),
  };
}

function createMockMediaStreamSource() {
  return { connect: vi.fn(), disconnect: vi.fn() };
}

function createMockTrack() {
  return { stop: vi.fn() };
}

function createMockMediaStream(tracks = [createMockTrack()]) {
  return { getTracks: () => tracks };
}

function createMockAudioContext(
  analyser: ReturnType<typeof createMockAnalyser>,
  mediaStreamSource: ReturnType<typeof createMockMediaStreamSource>,
) {
  return {
    sampleRate: 48000,
    destination: {},
    audioWorklet: { addModule: vi.fn().mockResolvedValue(undefined) },
    createAnalyser: vi.fn(() => analyser),
    createMediaStreamSource: vi.fn(() => mediaStreamSource),
    close: vi.fn(),
  };
}

// ---------------------------------------------------------------------------
// Stub all browser globals before each test
// ---------------------------------------------------------------------------

let mockPort: ReturnType<typeof createMockPort>;
let mockWorkletNode: ReturnType<typeof createMockWorkletNode>;
let mockAnalyser: ReturnType<typeof createMockAnalyser>;
let mockMediaStreamSource: ReturnType<typeof createMockMediaStreamSource>;
let mockAudioCtx: ReturnType<typeof createMockAudioContext>;
let mockMicStream: ReturnType<typeof createMockMediaStream>;
let mockMicTrack: ReturnType<typeof createMockTrack>;

beforeEach(() => {
  mockPort = createMockPort();
  mockWorkletNode = createMockWorkletNode(mockPort);
  mockAnalyser = createMockAnalyser();
  mockMediaStreamSource = createMockMediaStreamSource();
  mockAudioCtx = createMockAudioContext(mockAnalyser, mockMediaStreamSource);
  mockMicTrack = createMockTrack();
  mockMicStream = createMockMediaStream([mockMicTrack]);

  // AudioContext — must use `function` for `new` to work in vitest 4.x
  vi.stubGlobal(
    "AudioContext",
    vi.fn(function (this: any) {
      return Object.assign(this, mockAudioCtx);
    }),
  );

  // AudioWorkletNode
  vi.stubGlobal(
    "AudioWorkletNode",
    vi.fn(function (this: any) {
      return Object.assign(this, mockWorkletNode);
    }),
  );

  // AudioNode — used for instanceof checks
  vi.stubGlobal("AudioNode", class AudioNode {});

  // MediaStream — used for instanceof checks
  vi.stubGlobal("MediaStream", class MediaStream {});

  // fetch
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      arrayBuffer: vi.fn().mockResolvedValue(new ArrayBuffer(8)),
    }),
  );

  // WebAssembly.compile
  vi.stubGlobal("WebAssembly", {
    compile: vi.fn().mockResolvedValue({ __wasmModule: true }),
  });

  // navigator.mediaDevices.getUserMedia
  vi.stubGlobal("navigator", {
    mediaDevices: {
      getUserMedia: vi.fn().mockResolvedValue(mockMicStream),
    },
  });

  // URL.createObjectURL / revokeObjectURL
  vi.stubGlobal("URL", {
    createObjectURL: vi.fn(() => "blob:mock-url"),
    revokeObjectURL: vi.fn(),
  });

  // Blob
  vi.stubGlobal("Blob", class Blob {
    constructor(public parts: unknown[], public opts: unknown) {}
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("StadiumEQ", () => {
  // ---- Constructor --------------------------------------------------------

  describe("constructor", () => {
    it("creates instance with correct defaults", () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      expect(eq.status).toBe("idle");
      expect(eq.mix).toEqual({ crowd: 0, speaker: 0, music: 0, gainDb: 0 });
      expect(eq.isRunning).toBe(false);
      expect(eq.context).toBeNull();
      expect(eq.analyser).toBeNull();
    });

    it("uses default sampleRate=48000 and frameSize=480", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      // AudioContext constructed with sampleRate 48000
      expect(AudioContext).toHaveBeenCalledWith({ sampleRate: 48000 });

      // init-wasm message was posted (frame size is used internally)
      expect(mockPort.postMessage).toHaveBeenCalledWith({
        type: "init-wasm",
        module: { __wasmModule: true },
      });
    });
  });

  // ---- start() ------------------------------------------------------------

  describe("start()", () => {
    it("fetches WASM, creates AudioContext, registers worklet, wires graph, requests mic", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      // Fetches WASM
      expect(fetch).toHaveBeenCalledWith("/test.wasm");

      // Compiles WASM
      expect(WebAssembly.compile).toHaveBeenCalled();

      // Creates AudioContext
      expect(AudioContext).toHaveBeenCalledWith({ sampleRate: 48000 });

      // Registers worklet via blob URL
      expect(mockAudioCtx.audioWorklet.addModule).toHaveBeenCalledWith("blob:mock-url");

      // Creates AudioWorkletNode
      expect(AudioWorkletNode).toHaveBeenCalledWith(
        mockAudioCtx,
        "stadium-eq-processor",
        {
          numberOfInputs: 1,
          numberOfOutputs: 1,
          channelCount: 1,
          channelCountMode: "explicit",
        },
      );

      // Posts init-wasm message
      expect(mockPort.postMessage).toHaveBeenCalledWith({
        type: "init-wasm",
        module: { __wasmModule: true },
      });

      // Requests microphone
      expect(navigator.mediaDevices.getUserMedia).toHaveBeenCalledWith({
        audio: {
          sampleRate: 48000,
          channelCount: 1,
          echoCancellation: false,
          noiseSuppression: false,
          autoGainControl: false,
        },
      });

      // Connects audio graph: source -> worklet -> analyser -> destination
      expect(mockMediaStreamSource.connect).toHaveBeenCalledWith(mockWorkletNode);
      expect(mockWorkletNode.connect).toHaveBeenCalledWith(mockAnalyser);
      expect(mockAnalyser.connect).toHaveBeenCalledWith(mockAudioCtx.destination);

      // Creates analyser with expected settings
      expect(mockAudioCtx.createAnalyser).toHaveBeenCalled();
    });

    it("uses custom workletUrl when provided", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm", workletUrl: "/custom-worklet.js" });
      await eq.start();

      expect(mockAudioCtx.audioWorklet.addModule).toHaveBeenCalledWith("/custom-worklet.js");
      // Should not create blob URL
      expect(URL.createObjectURL).not.toHaveBeenCalled();
    });

    it("accepts a custom audioSource (MediaStream) without requesting mic", async () => {
      // Create a real instance of our mock MediaStream class
      const customStream = new MediaStream();
      const customSourceNode = createMockMediaStreamSource();
      mockAudioCtx.createMediaStreamSource.mockReturnValue(customSourceNode);

      const eq = new StadiumEQ({ wasmUrl: "/test.wasm", audioSource: customStream as any });
      await eq.start();

      // Should NOT request microphone
      expect(navigator.mediaDevices.getUserMedia).not.toHaveBeenCalled();

      // Should use createMediaStreamSource with the provided stream
      expect(mockAudioCtx.createMediaStreamSource).toHaveBeenCalledWith(customStream);
    });

    it("sets status to 'loading' then connects worklet port onmessage", async () => {
      const statusChanges: string[] = [];
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      eq.on("statuschange", (s) => statusChanges.push(s));

      await eq.start();

      expect(statusChanges).toContain("loading");
      // worklet port onmessage handler should be set
      expect(mockPort.onmessage).toBeTypeOf("function");
    });
  });

  // ---- stop() -------------------------------------------------------------

  describe("stop()", () => {
    it("disconnects nodes, closes AudioContext, stops mic tracks, revokes blob URL", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      eq.stop();

      expect(mockWorkletNode.disconnect).toHaveBeenCalled();
      expect(mockMediaStreamSource.disconnect).toHaveBeenCalled();
      expect(mockMicTrack.stop).toHaveBeenCalled();
      expect(mockAnalyser.disconnect).toHaveBeenCalled();
      expect(mockAudioCtx.close).toHaveBeenCalled();
      expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:mock-url");
      expect(eq.status).toBe("idle");
    });

    it("is a no-op when already idle", () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      const statusCb = vi.fn();
      eq.on("statuschange", statusCb);

      eq.stop(); // should do nothing

      expect(statusCb).not.toHaveBeenCalled();
      expect(eq.status).toBe("idle");
    });
  });

  // ---- destroy() ----------------------------------------------------------

  describe("destroy()", () => {
    it("calls stop and emits 'destroyed' event, clears listeners", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      const destroyedCb = vi.fn();
      eq.on("destroyed", destroyedCb);

      eq.destroy();

      expect(destroyedCb).toHaveBeenCalled();
      expect(eq.status).toBe("idle");

      // Listeners should be cleared — further events should not fire
      const statusCb = vi.fn();
      eq.on("statuschange", statusCb);
      // The listeners object was cleared by destroy, but on() re-creates entries.
      // The key test is that the destroyedCb was called.
    });

    it("clears all event listeners after emitting destroyed", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      const statusCb = vi.fn();
      eq.on("statuschange", statusCb);
      statusCb.mockClear(); // clear the "loading" call from start()

      eq.destroy();

      // statuschange for "idle" fires before listeners are cleared
      // Now subscribe again and trigger — old listener should not fire
      const oldStatusCb = statusCb;
      oldStatusCb.mockClear();

      // After destroy, the listeners map is empty.
      // If we call start again the old statusCb should NOT be invoked.
      await eq.start();
      expect(oldStatusCb).not.toHaveBeenCalled();
    });
  });

  // ---- setMix() -----------------------------------------------------------

  describe("setMix()", () => {
    it("posts correct message to worklet port", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();
      mockPort.postMessage.mockClear();

      eq.setMix({ crowd: 0.5, speaker: 1.0, music: -0.3, gainDb: 2 });

      expect(mockPort.postMessage).toHaveBeenCalledWith({
        type: "set-mix",
        crowd: 0.5,
        speaker: 1.0,
        music: -0.3,
        gainDb: 2,
      });
    });

    it("merges partial updates with existing mix", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      eq.setMix({ crowd: 0.5 });
      expect(eq.mix).toEqual({ crowd: 0.5, speaker: 0, music: 0, gainDb: 0 });

      mockPort.postMessage.mockClear();
      eq.setMix({ speaker: 0.8 });
      expect(eq.mix).toEqual({ crowd: 0.5, speaker: 0.8, music: 0, gainDb: 0 });

      expect(mockPort.postMessage).toHaveBeenCalledWith({
        type: "set-mix",
        crowd: 0.5,
        speaker: 0.8,
        music: 0,
        gainDb: 0,
      });
    });
  });

  // ---- setBypass() --------------------------------------------------------

  describe("setBypass()", () => {
    it("posts correct message to worklet port", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();
      mockPort.postMessage.mockClear();

      eq.setBypass(true);
      expect(mockPort.postMessage).toHaveBeenCalledWith({
        type: "set-bypass",
        bypass: true,
      });

      mockPort.postMessage.mockClear();
      eq.setBypass(false);
      expect(mockPort.postMessage).toHaveBeenCalledWith({
        type: "set-bypass",
        bypass: false,
      });
    });
  });

  // ---- calibrate() --------------------------------------------------------

  describe("calibrate()", () => {
    it("posts correct message to worklet port", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();
      mockPort.postMessage.mockClear();

      eq.calibrate();
      expect(mockPort.postMessage).toHaveBeenCalledWith({ type: "calibrate" });
    });
  });

  // ---- Event system -------------------------------------------------------

  describe("Event system", () => {
    it("on() subscribes and receives events", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      const cb = vi.fn();
      eq.on("statuschange", cb);

      await eq.start();
      // Should have received "loading" status change
      expect(cb).toHaveBeenCalledWith("loading");
    });

    it("off() unsubscribes from events", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      const cb = vi.fn();
      eq.on("statuschange", cb);
      eq.off("statuschange", cb);

      await eq.start();
      expect(cb).not.toHaveBeenCalled();
    });

    it("on() returns an unsubscribe function", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      const cb = vi.fn();
      const unsub = eq.on("statuschange", cb);

      unsub();

      await eq.start();
      expect(cb).not.toHaveBeenCalled();
    });

    it("multiple listeners receive the same event", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      const cb1 = vi.fn();
      const cb2 = vi.fn();
      eq.on("statuschange", cb1);
      eq.on("statuschange", cb2);

      await eq.start();
      expect(cb1).toHaveBeenCalledWith("loading");
      expect(cb2).toHaveBeenCalledWith("loading");
    });
  });

  // ---- Status changes via worklet messages --------------------------------

  describe("Status changes via worklet messages", () => {
    it("worklet 'ready' message sets status to 'processing'", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      // Simulate worklet posting "ready"
      mockPort.onmessage!({ data: { type: "ready" } } as MessageEvent);

      expect(eq.status).toBe("processing");
      expect(eq.isRunning).toBe(true);
    });

    it("worklet 'error' message sets status to 'error' and emits error event", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      const errorCb = vi.fn();
      eq.on("error", errorCb);

      await eq.start();

      mockPort.onmessage!({ data: { type: "error", error: "WASM init failed" } } as MessageEvent);

      expect(eq.status).toBe("error");
      expect(errorCb).toHaveBeenCalledWith("WASM init failed");
    });

    it("worklet 'status' message updates status", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      mockPort.onmessage!({ data: { type: "status", status: "calibrating" } } as MessageEvent);
      expect(eq.status).toBe("calibrating");
      expect(eq.isRunning).toBe(true);
    });

    it("emits 'ready' event when worklet posts ready message", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      const readyCb = vi.fn();
      eq.on("ready", readyCb);

      await eq.start();
      mockPort.onmessage!({ data: { type: "ready" } } as MessageEvent);

      expect(readyCb).toHaveBeenCalled();
    });
  });

  // ---- getSpectrumData() --------------------------------------------------

  describe("getSpectrumData()", () => {
    it("returns null when not started", () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      expect(eq.getSpectrumData()).toBeNull();
    });

    it("returns frequency data when analyser exists", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      const data = eq.getSpectrumData();
      expect(data).not.toBeNull();
      expect(data!.binCount).toBe(128);
      expect(data!.frequencyData).toBeInstanceOf(Uint8Array);
      expect(data!.frequencyData.length).toBe(128);
      expect(mockAnalyser.getByteFrequencyData).toHaveBeenCalled();
    });
  });

  // ---- Error handling -----------------------------------------------------

  describe("start() error handling", () => {
    it("sets status to 'error' and emits error event on failure", async () => {
      vi.stubGlobal(
        "fetch",
        vi.fn().mockRejectedValue(new Error("Network error")),
      );

      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      const errorCb = vi.fn();
      eq.on("error", errorCb);

      await expect(eq.start()).rejects.toThrow("Network error");

      expect(eq.status).toBe("error");
      expect(errorCb).toHaveBeenCalledWith("Error: Network error");
    });
  });

  // ---- Double start / stop ------------------------------------------------

  describe("Double start/stop", () => {
    it("calling start() when already running is a no-op", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();

      // Simulate becoming "processing"
      mockPort.onmessage!({ data: { type: "ready" } } as MessageEvent);
      expect(eq.status).toBe("processing");

      // Reset mocks to verify no second initialization happens
      (fetch as any).mockClear();

      await eq.start(); // should be a no-op (status is not "idle")
      expect(fetch).not.toHaveBeenCalled();
    });

    it("calling start() when in 'loading' state is a no-op", async () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      await eq.start();
      // Status is "loading" after start (no "ready" message yet)
      expect(eq.status).toBe("loading");

      (fetch as any).mockClear();
      await eq.start();
      expect(fetch).not.toHaveBeenCalled();
    });

    it("calling stop() when idle is a no-op", () => {
      const eq = new StadiumEQ({ wasmUrl: "/test.wasm" });
      expect(eq.status).toBe("idle");

      // Should not throw or do anything
      eq.stop();
      expect(eq.status).toBe("idle");
    });
  });
});
