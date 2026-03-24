import type { Metadata } from "next";
import React from "react";

export const metadata: Metadata = {
  title: "Stadium EQ — Demo",
  description: "Interactive demos of the Stadium Audio Equalizer wrappers",
};

const NAV_ITEMS = [
  { href: "/", label: "Drop-in Component" },
  { href: "/custom", label: "Custom Hook" },
  { href: "/vanilla", label: "Vanilla JS" },
];

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body
        style={{
          margin: 0,
          background: "#0f0f23",
          color: "#e0e0e0",
          fontFamily:
            '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
          minHeight: "100vh",
        }}
      >
        {/* Top nav */}
        <nav
          style={{
            display: "flex",
            alignItems: "center",
            gap: 24,
            padding: "16px 24px",
            borderBottom: "1px solid #1e1e3a",
            background: "#16213e",
          }}
        >
          <strong style={{ fontSize: 18, color: "#0dff72", marginRight: 16 }}>
            Stadium EQ
          </strong>
          {NAV_ITEMS.map((item) => (
            <a
              key={item.href}
              href={item.href}
              style={{
                color: "#aaa",
                textDecoration: "none",
                fontSize: 14,
                fontWeight: 500,
                padding: "4px 8px",
                borderRadius: 4,
              }}
            >
              {item.label}
            </a>
          ))}
        </nav>

        {/* Page content */}
        <main style={{ maxWidth: 640, margin: "40px auto", padding: "0 24px" }}>
          {children}
        </main>
      </body>
    </html>
  );
}
