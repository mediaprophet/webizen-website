import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const script = path.join("tooling", "build_pages.py");
const candidates =
  process.platform === "win32" ? ["python", "py", "python3"] : ["python3", "python"];

for (const cmd of candidates) {
  const args = cmd === "py" ? ["-3", script] : [script];
  const result = spawnSync(cmd, args, {
    cwd: root,
    stdio: "inherit",
    shell: false,
  });
  if (result.error?.code === "ENOENT") continue;
  process.exit(result.status ?? 1);
}

console.error("Python 3 is required to assemble _site.");
process.exit(1);
