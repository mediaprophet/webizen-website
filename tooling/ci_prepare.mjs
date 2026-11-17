// Workers Builds runs `npm clean-install` then `npx wrangler deploy`.
// It does not honour wrangler.jsonc build.command, so assemble _site here.
if (process.env.WORKERS_CI !== "1") process.exit(0);
await import("./build_pages.mjs");
