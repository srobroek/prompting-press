/**
 * version.mjs — semver helpers + manifest read/write for docs versioning.
 *
 * Stable Node.js APIs only (fs/path) — runs under node >=22.12.0 (CI deploy
 * toolchain) and node 25.x (local mise).
 *
 * Exports:
 *   parse(verStr)           → { major, minor, patch } | throws on bad input
 *   bucket(verStr)          → "X.Y"
 *   bucketAction(new, prev) → 'new-bucket' | 'overwrite' | throws
 *   readManifest(path?)     → parsed manifest object
 *   writeManifest(obj, path?) → void (canonical sorted-key JSON + trailing newline)
 */

import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

/** Default manifest path relative to this file (scripts/lib → src/data). */
const DEFAULT_MANIFEST = resolve(
  __dirname,
  "../../src/data/versions.json",
);

// ---------------------------------------------------------------------------
// Semver helpers
// ---------------------------------------------------------------------------

const SEMVER_RE = /^(\d+)\.(\d+)\.(\d+)$/;

/**
 * Parse a semver string "X.Y.Z" into its numeric parts.
 * Throws a descriptive Error on any other input.
 *
 * @param {string} verStr
 * @returns {{ major: number, minor: number, patch: number }}
 */
export function parse(verStr) {
  if (typeof verStr !== "string") {
    throw new Error(`version.mjs parse: expected string, got ${typeof verStr} (${verStr})`);
  }
  const m = SEMVER_RE.exec(verStr.trim());
  if (!m) {
    throw new Error(
      `version.mjs parse: "${verStr}" is not a valid semver string (expected X.Y.Z)`,
    );
  }
  return {
    major: Number(m[1]),
    minor: Number(m[2]),
    patch: Number(m[3]),
  };
}

/**
 * Derive the minor bucket string "X.Y" from a full semver.
 *
 * @param {string} verStr
 * @returns {string}
 */
export function bucket(verStr) {
  const { major, minor } = parse(verStr);
  return `${major}.${minor}`;
}

/**
 * Determine whether a new version creates a new minor bucket or overwrites the
 * existing one (patch rollup).
 *
 * Rules:
 *   - major.minor changed (compared to prevVer) → 'new-bucket'
 *   - only patch changed                         → 'overwrite'
 *   - unparsable newVer or prevVer              → throws
 *
 * @param {string} newVer  e.g. "1.1.0"
 * @param {string} prevVer e.g. "1.0.3"
 * @returns {'new-bucket' | 'overwrite'}
 */
export function bucketAction(newVer, prevVer) {
  // Both must parse — throws on garbage input satisfying FR-013.
  const n = parse(newVer);
  const p = parse(prevVer);

  if (n.major !== p.major || n.minor !== p.minor) {
    return "new-bucket";
  }
  return "overwrite";
}

// ---------------------------------------------------------------------------
// Manifest read/write (canonical sorted-key JSON)
// ---------------------------------------------------------------------------

/**
 * Deep-sort all object keys alphabetically (recursive).
 * Arrays preserve their order; only object keys are sorted.
 *
 * @param {unknown} value
 * @returns {unknown}
 */
function sortKeys(value) {
  if (Array.isArray(value)) {
    return value.map(sortKeys);
  }
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((k) => [k, sortKeys(value[k])]),
    );
  }
  return value;
}

/**
 * Read and parse the versions manifest.
 *
 * @param {string} [manifestPath] Absolute path; defaults to src/data/versions.json.
 * @returns {object}
 */
export function readManifest(manifestPath = DEFAULT_MANIFEST) {
  const raw = readFileSync(manifestPath, "utf-8");
  return JSON.parse(raw);
}

/**
 * Write the versions manifest as canonical sorted-key JSON with a trailing newline.
 * This ensures `docs:snapshot` is idempotent (twice-run zero-diff).
 *
 * @param {object} manifest
 * @param {string} [manifestPath] Absolute path; defaults to src/data/versions.json.
 */
export function writeManifest(manifest, manifestPath = DEFAULT_MANIFEST) {
  const sorted = sortKeys(manifest);
  writeFileSync(manifestPath, JSON.stringify(sorted, null, 2) + "\n", "utf-8");
}

// ---------------------------------------------------------------------------
// Self-checks (run via: node scripts/lib/version.mjs)
// ---------------------------------------------------------------------------

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  let failed = 0;

  function check(label, fn) {
    try {
      fn();
      console.log(`  PASS  ${label}`);
    } catch (e) {
      console.error(`  FAIL  ${label}: ${e.message}`);
      failed++;
    }
  }

  function assert(cond, msg) {
    if (!cond) throw new Error(msg);
  }

  console.log("version.mjs self-checks:");

  check("parse('1.2.3') → {major:1,minor:2,patch:3}", () => {
    const r = parse("1.2.3");
    assert(r.major === 1 && r.minor === 2 && r.patch === 3, JSON.stringify(r));
  });

  check("bucket('2.5.0') → '2.5'", () => {
    assert(bucket("2.5.0") === "2.5", bucket("2.5.0"));
  });

  check("bucketAction('1.1.0','1.0.3') === 'new-bucket'", () => {
    assert(bucketAction("1.1.0", "1.0.3") === "new-bucket",
      `got ${bucketAction("1.1.0", "1.0.3")}`);
  });

  check("bucketAction('1.1.1','1.1.0') === 'overwrite'", () => {
    assert(bucketAction("1.1.1", "1.1.0") === "overwrite",
      `got ${bucketAction("1.1.1", "1.1.0")}`);
  });

  check("bucketAction('garbage','1.1.0') throws", () => {
    let threw = false;
    try { bucketAction("garbage", "1.1.0"); } catch { threw = true; }
    assert(threw, "expected throw but did not throw");
  });

  check("bucketAction('1.1.0','garbage') throws", () => {
    let threw = false;
    try { bucketAction("1.1.0", "garbage"); } catch { threw = true; }
    assert(threw, "expected throw but did not throw");
  });

  check("sortKeys produces alphabetical output", () => {
    const r = sortKeys({ z: 1, a: 2, m: 3 });
    assert(JSON.stringify(Object.keys(r)) === '["a","m","z"]', JSON.stringify(Object.keys(r)));
  });

  if (failed > 0) {
    console.error(`\n${failed} check(s) FAILED`);
    process.exit(1);
  }
  console.log(`\nAll checks passed.`);
}
