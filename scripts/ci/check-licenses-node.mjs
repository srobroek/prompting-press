// License-policy checker — Node (Apache-2.0 release compatibility).
//
// Walks the installed dependency tree of packages/typescript and asserts every
// package's license is permissive / Apache-2.0-compatible. Invoked by
// scripts/ci/check-licenses-node.sh (the moon/CI entry point).
//
// WHY a custom scanner instead of `pnpm licenses list`: pnpm's built-in command
// requires a fully-populated content-addressable store index and errors
// (ERR_PNPM_MISSING_PACKAGE_INDEX_FILE) on optional platform deps in offline /
// frozen-store CI. Reading each installed package.json is deterministic, offline,
// and dependency-free. Mirrors check-advisories-node.sh's "scan the FULL tree
// (runtime + dev toolchain — the build chain is supply chain too)" posture.
//
// The npm DISTRIBUTION surface is small (zod is a peer dep; there are no bundled
// prod `dependencies`; the .node addon's Rust licenses are covered by
// ci:check-licenses + THIRD-PARTY-LICENSES.md). Scanning the whole installed tree
// is a strict superset that also guards the dev toolchain's licenses.
//
// Exit 0 = all allowed; exit 1 = a disallowed license was found; exit 2 = setup
// error (node_modules missing — run `pnpm install` first).

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const TS_PKG = path.join(REPO_ROOT, "packages/typescript");
const PNPM_DIR = path.join(TS_PKG, "node_modules/.pnpm");

// Permissive, Apache-2.0-compatible SPDX ids (normalized upper-case for matching).
// Add an id only for a genuinely compatible license; never add copyleft to pass CI.
const ALLOWED = new Set(
  [
    "MIT",
    "MIT-0",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "0BSD",
    "ISC",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "CC0-1.0",
    "CC-BY-4.0",
    "BlueOak-1.0.0",
    "Python-2.0", // argparse (JS port) ships under the PSF/Python-2.0 license
    "Zlib",
    "WTFPL",
  ].map((s) => s.toUpperCase()),
);

function readLicense(pj) {
  if (!pj) return "UNKNOWN";
  if (typeof pj.license === "string") return pj.license;
  if (pj.license && typeof pj.license.type === "string") return pj.license.type; // legacy object form
  if (Array.isArray(pj.licenses)) return pj.licenses.map((l) => l.type || l).join(" OR "); // legacy array form
  return "UNKNOWN";
}

// A license expression is OK if:
//   - an OR group where ANY sub-term is allowed, or
//   - an AND group where ALL sub-terms are allowed, or
//   - a single allowed id.
// We strip SPDX "WITH <exception>" and parentheses; treat mixed AND/OR
// conservatively by requiring every OR-group to have an allowed member.
function isAllowed(expr) {
  if (!expr || expr === "UNKNOWN") return false;
  const norm = expr.replace(/[()]/g, " ").trim();
  // Split on OR first (least restrictive wins), then require each remaining
  // AND-group's terms to all be allowed.
  const orTerms = norm.split(/\s+OR\s+/i);
  return orTerms.some((orTerm) => {
    const andTerms = orTerm.split(/\s+AND\s+/i);
    return andTerms.every((t) => {
      const id = t.replace(/\s+WITH\s+.*/i, "").trim().toUpperCase();
      return ALLOWED.has(id);
    });
  });
}

function collectInstalledPackages() {
  const found = new Map(); // "name@version" -> license
  if (!fs.existsSync(PNPM_DIR)) return found;
  // node_modules/.pnpm/<name>@<version>/node_modules/<name>/package.json
  for (const entry of fs.readdirSync(PNPM_DIR, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const inner = path.join(PNPM_DIR, entry.name, "node_modules");
    if (!fs.existsSync(inner)) continue;
    const walk = (dir) => {
      for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
        if (!e.isDirectory()) continue;
        const p = path.join(dir, e.name);
        if (e.name.startsWith("@")) {
          walk(p); // scope dir — recurse one level
          continue;
        }
        const pjPath = path.join(p, "package.json");
        if (fs.existsSync(pjPath)) {
          try {
            const pj = JSON.parse(fs.readFileSync(pjPath, "utf8"));
            if (pj.name && pj.version) found.set(`${pj.name}@${pj.version}`, readLicense(pj));
          } catch {
            /* ignore unreadable package.json */
          }
        }
      }
    };
    walk(inner);
  }
  return found;
}

const pkgs = collectInstalledPackages();
if (pkgs.size === 0) {
  console.error(
    "ERROR: no installed packages found under packages/typescript/node_modules/.pnpm\n" +
      "Run `pnpm -C packages/typescript install` before the license gate.",
  );
  process.exit(2);
}

const byLicense = new Map();
const violations = [];
for (const [id, lic] of [...pkgs].sort()) {
  if (!byLicense.has(lic)) byLicense.set(lic, []);
  byLicense.get(lic).push(id);
  if (!isAllowed(lic)) violations.push({ id, lic });
}

console.log(`License gate (Node): scanned ${pkgs.size} installed packages.`);
console.log("  Licenses found:");
for (const lic of [...byLicense.keys()].sort()) {
  console.log(`    ${lic} (${byLicense.get(lic).length})`);
}
console.log("");

if (violations.length > 0) {
  console.error("ERROR: disallowed (non-Apache-2.0-compatible) licenses found:");
  for (const v of violations) console.error(`    ${v.id}  ->  ${v.lic}`);
  console.error(
    "\nEach package above carries a license not in the permissive allow-list.\n" +
      "Triage: replace the dependency, or (if genuinely Apache-2.0-compatible)\n" +
      "add its SPDX id to ALLOWED in scripts/ci/check-licenses-node.mjs with a rationale.",
  );
  process.exit(1);
}

console.log("License gate (Node) PASSED — all installed packages are Apache-2.0-compatible.");
