import type { NextConfig } from "next";

const isEmbedded = process.env.BUILD_TARGET === "embedded";

const nextConfig: NextConfig = {
  output: isEmbedded ? "export" : undefined,
  images: {
    unoptimized: isEmbedded,
  },
  async rewrites() {
    if (isEmbedded) return { beforeFiles: [], afterFiles: [], fallback: [] };
    const rokoUrl = process.env.NEXT_PUBLIC_ROKO_SERVE_URL || "http://localhost:6677";
    // Use afterFiles so app-dir route handlers (e.g. /api/git-info) take
    // precedence over the proxy rewrite to roko-serve.
    return {
      beforeFiles: [],
      afterFiles: [
        { source: "/api/:path*", destination: `${rokoUrl}/api/:path*` },
        { source: "/ws/:path*", destination: `${rokoUrl}/ws/:path*` },
      ],
      fallback: [],
    };
  },
};

export default nextConfig;
