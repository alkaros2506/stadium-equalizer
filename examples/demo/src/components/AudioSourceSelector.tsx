"use client";

import React from "react";
import { AUDIO_SAMPLES, type AudioSample } from "../audio-samples";

export interface AudioSourceSelectorProps {
  value: string;
  onChange: (sourceId: string) => void;
  disabled?: boolean;
}

export function AudioSourceSelector({
  value,
  onChange,
  disabled = false,
}: AudioSourceSelectorProps) {
  const selected: AudioSample | undefined = AUDIO_SAMPLES.find(
    (s) => s.id === value
  );

  return (
    <div style={{ marginBottom: 16 }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          background: "#1a1a2e",
          borderRadius: 8,
          padding: "10px 14px",
        }}
      >
        <span
          style={{ fontSize: 13, fontWeight: 600, width: 60, flexShrink: 0 }}
        >
          Source
        </span>
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          style={{
            flex: 1,
            padding: "6px 8px",
            background: "#0f0f23",
            color: "#e0e0e0",
            border: "1px solid #333",
            borderRadius: 4,
            fontSize: 13,
            cursor: disabled ? "not-allowed" : "pointer",
            opacity: disabled ? 0.5 : 1,
          }}
        >
          <option value="mic">Microphone</option>
          <optgroup label="Sample Audio">
            {AUDIO_SAMPLES.map((s) => (
              <option key={s.id} value={s.id}>
                {s.label}
              </option>
            ))}
          </optgroup>
        </select>
      </div>
      {selected && (
        <p
          style={{
            fontSize: 11,
            color: "#666",
            marginTop: 4,
            paddingLeft: 14,
          }}
        >
          {selected.description}
        </p>
      )}
    </div>
  );
}
