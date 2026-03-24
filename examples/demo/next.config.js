/** @type {import('next').NextConfig} */
const nextConfig = {
  // Serve .wasm files correctly
  async headers() {
    return [
      {
        source: "/:path*.wasm",
        headers: [
          { key: "Content-Type", value: "application/wasm" },
        ],
      },
    ];
  },
};

module.exports = nextConfig;
