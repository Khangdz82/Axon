import fs from "node:fs";
import { WASI } from "node:wasi";

const files = process.argv.slice(2);
if (files.length === 0) {
  console.error("Usage: node tests/wasm/wasi_run.mjs <file1.wasm> [...]");
  process.exit(2);
}

let lastRc = 0;
for (const f of files) {
  const bytes = fs.readFileSync(f);
  console.log("=== Testing", f, bytes.length, "bytes ===");
  const ok = WebAssembly.validate(bytes);
  console.log("WebAssembly.validate:", ok);
  if (!ok) process.exit(1);
  let simdCnt = 0;
  for (let i = 0; i < bytes.length; i++) if (bytes[i] === 0xFD) simdCnt++;
  console.log("0xFD SIMD128 prefix occurrences:", simdCnt);
  const wasi = new WASI({ version: "preview1", returnOnExit: true });
  const mod = new WebAssembly.Module(bytes);
  const inst = new WebAssembly.Instance(mod, wasi.getImportObject());
  const rc = wasi.start(inst);
  console.log("Exit code:", rc);
  lastRc = rc;
}
process.exit(lastRc);
