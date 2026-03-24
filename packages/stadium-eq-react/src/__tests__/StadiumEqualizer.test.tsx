import { describe, it, expect, vi, beforeEach } from "vitest";
import React from "react";
import { render, screen, fireEvent, act } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { StadiumEqualizer } from "../StadiumEqualizer.js";

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

beforeEach(() => {
  vi.clearAllMocks();
  eventListeners = {};
});

describe("StadiumEqualizer component", () => {
  it("renders with all controls: Start/Stop button, Calibrate button, sliders, bypass toggle, spectrum canvas", () => {
    render(<StadiumEqualizer wasmUrl="/test.wasm" />);

    // Start/Stop button
    expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument();

    // Calibrate button
    expect(screen.getByRole("button", { name: "Calibrate" })).toBeInTheDocument();

    // Sliders (4 range inputs: Crowd, Speaker, Music, Gain)
    const sliders = screen.getAllByRole("slider");
    expect(sliders).toHaveLength(4);

    // Bypass checkbox
    expect(screen.getByRole("checkbox")).toBeInTheDocument();

    // Spectrum canvas
    const canvas = document.querySelector("canvas");
    expect(canvas).toBeInTheDocument();
  });

  it("Start button triggers start, changes text to 'Stop' when running", async () => {
    render(<StadiumEqualizer wasmUrl="/test.wasm" />);

    const startButton = screen.getByRole("button", { name: "Start" });

    // Click Start — handleToggle is async, so we need act to flush promises
    await act(async () => {
      fireEvent.click(startButton);
    });

    expect(mockStart).toHaveBeenCalledTimes(1);

    // Simulate status change to "processing" to make isRunning true
    expect(eventListeners["statuschange"]).toBeDefined();

    act(() => {
      eventListeners["statuschange"]("processing");
    });

    expect(screen.getByRole("button", { name: "Stop" })).toBeInTheDocument();
  });

  it("Calibrate button is disabled when not running", () => {
    render(<StadiumEqualizer wasmUrl="/test.wasm" />);

    const calibrateButton = screen.getByRole("button", { name: "Calibrate" });
    expect(calibrateButton).toBeDisabled();
  });

  it("Sliders update mix values", () => {
    render(<StadiumEqualizer wasmUrl="/test.wasm" />);

    const sliders = screen.getAllByRole("slider");
    // The first slider is "Crowd"
    const crowdSlider = sliders[0];

    fireEvent.change(crowdSlider, { target: { value: "0.75" } });

    expect(crowdSlider).toHaveValue("0.75");
  });

  it("Bypass toggle sends setBypass", async () => {
    render(<StadiumEqualizer wasmUrl="/test.wasm" />);

    // First need to start the EQ so the instance exists
    const startButton = screen.getByRole("button", { name: "Start" });
    fireEvent.click(startButton);

    await vi.waitFor(() => {
      expect(eventListeners["statuschange"]).toBeDefined();
    });

    React.act(() => {
      eventListeners["statuschange"]("processing");
    });

    const checkbox = screen.getByRole("checkbox");
    fireEvent.click(checkbox);

    expect(mockSetBypass).toHaveBeenCalledWith(true);
  });
});
