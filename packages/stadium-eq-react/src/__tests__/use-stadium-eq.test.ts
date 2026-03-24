import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useStadiumEQ } from "../use-stadium-eq.js";

// ---------------------------------------------------------------------------
// Mock the "stadium-eq" vanilla package
// ---------------------------------------------------------------------------

type Listener = (...args: any[]) => void;

const mockStart = vi.fn().mockResolvedValue(undefined);
const mockStop = vi.fn();
const mockCalibrate = vi.fn();
const mockSetMix = vi.fn();
const mockSetBypass = vi.fn();
const mockDestroy = vi.fn();
const mockOn = vi.fn();

// Store registered listeners so we can simulate events
let eventListeners: Record<string, Listener>;

vi.mock("stadium-eq", () => {
  class MockStadiumEQ {
    constructor(_options: any) {
      eventListeners = {};
      mockOn.mockImplementation((event: string, listener: Listener) => {
        eventListeners[event] = listener;
        return () => {
          delete eventListeners[event];
        };
      });
    }
    start = mockStart;
    stop = mockStop;
    calibrate = mockCalibrate;
    setMix = mockSetMix;
    setBypass = mockSetBypass;
    destroy = mockDestroy;
    on = mockOn;
    off = vi.fn();
    analyser = null;
  }
  return { StadiumEQ: MockStadiumEQ };
});

const defaultOptions = { wasmUrl: "/test.wasm" };

beforeEach(() => {
  vi.clearAllMocks();
  eventListeners = {};
});

describe("useStadiumEQ", () => {
  it("returns initial state with idle status, isRunning false, default mix, null error", () => {
    const { result } = renderHook(() => useStadiumEQ(defaultOptions));

    expect(result.current.status).toBe("idle");
    expect(result.current.isRunning).toBe(false);
    expect(result.current.mix).toEqual({
      crowd: 0,
      speaker: 0,
      music: 0,
      gainDb: 0,
    });
    expect(result.current.error).toBeNull();
    expect(result.current.instance).toBeNull();
  });

  it("start() calls StadiumEQ.start()", async () => {
    const { result } = renderHook(() => useStadiumEQ(defaultOptions));

    await act(async () => {
      await result.current.start();
    });

    expect(mockStart).toHaveBeenCalledTimes(1);
  });

  it("stop() calls StadiumEQ.stop()", async () => {
    const { result } = renderHook(() => useStadiumEQ(defaultOptions));

    // Must start first so the instance is created
    await act(async () => {
      await result.current.start();
    });

    act(() => {
      result.current.stop();
    });

    expect(mockStop).toHaveBeenCalledTimes(1);
  });

  it("calibrate() calls StadiumEQ.calibrate()", async () => {
    const { result } = renderHook(() => useStadiumEQ(defaultOptions));

    await act(async () => {
      await result.current.start();
    });

    act(() => {
      result.current.calibrate();
    });

    expect(mockCalibrate).toHaveBeenCalledTimes(1);
  });

  it("setMix() updates local mix state and calls StadiumEQ.setMix()", async () => {
    const { result } = renderHook(() => useStadiumEQ(defaultOptions));

    await act(async () => {
      await result.current.start();
    });

    act(() => {
      result.current.setMix({ crowd: 0.5, speaker: -0.3 });
    });

    expect(result.current.mix).toEqual({
      crowd: 0.5,
      speaker: -0.3,
      music: 0,
      gainDb: 0,
    });
    expect(mockSetMix).toHaveBeenCalledWith({
      crowd: 0.5,
      speaker: -0.3,
      music: 0,
      gainDb: 0,
    });
  });

  it("setBypass() calls StadiumEQ.setBypass()", async () => {
    const { result } = renderHook(() => useStadiumEQ(defaultOptions));

    await act(async () => {
      await result.current.start();
    });

    act(() => {
      result.current.setBypass(true);
    });

    expect(mockSetBypass).toHaveBeenCalledWith(true);
  });

  it("status updates when StadiumEQ emits 'statuschange'", async () => {
    const { result } = renderHook(() => useStadiumEQ(defaultOptions));

    await act(async () => {
      await result.current.start();
    });

    // The mock registers listeners via on() — simulate a statuschange event
    expect(eventListeners["statuschange"]).toBeDefined();

    act(() => {
      eventListeners["statuschange"]("processing");
    });

    expect(result.current.status).toBe("processing");
    expect(result.current.isRunning).toBe(true);
  });

  it("error updates when StadiumEQ emits 'error'", async () => {
    const { result } = renderHook(() => useStadiumEQ(defaultOptions));

    await act(async () => {
      await result.current.start();
    });

    expect(eventListeners["error"]).toBeDefined();

    act(() => {
      eventListeners["error"]("Something went wrong");
    });

    expect(result.current.error).toBe("Something went wrong");
  });

  it("cleanup on unmount calls destroy()", async () => {
    const { result, unmount } = renderHook(() => useStadiumEQ(defaultOptions));

    await act(async () => {
      await result.current.start();
    });

    unmount();

    expect(mockDestroy).toHaveBeenCalledTimes(1);
  });
});
