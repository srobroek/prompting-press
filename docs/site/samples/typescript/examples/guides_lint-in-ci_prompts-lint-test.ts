// Wiring `prompt.check()` as a CI gate under `node --test`.
//
// A CI gate is a test that fails the build: load every `*.yaml` under a `prompts/`
// directory, construct each prompt, and assert `check()` returns no findings — naming
// the offender otherwise. Standalone: this program first materializes a `prompts/`
// directory of shipped fixtures in a temp dir and `chdir`s into it (a real repo keeps
// its own `prompts/` under version control), then registers the documented test cases.
//
// Run it: `node --test guides_lint-in-ci_prompts-lint-test.ts`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Prompt } from "prompting-press";

// ── Materialize the `prompts/` directory a real repo would keep under version control. ──
// A clean, shipped prompt: its untrusted-free variable needs no guard, so check() passes.
const dir = mkdtempSync(join(tmpdir(), "pp-lint-in-ci-"));
mkdirSync(join(dir, "prompts"));
writeFileSync(
  join(dir, "prompts", "assistant.yaml"),
  `
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company:
    type: string
    trusted: true
  max_words:
    type: integer
    trusted: true
`,
);
process.chdir(dir);

// ── The CI gate itself. ──
// test/prompts-lint.test.mjs  — runs under `node --test`
for (const file of readdirSync("prompts").filter((f) => f.endsWith(".yaml"))) {
  test(`shipped prompt ${file} passes check`, () => {
    // Construction throws on the hard invariants; let that fail the test directly.
    const prompt = Prompt.fromYaml(readFileSync(`prompts/${file}`, "utf-8"));
    const report = prompt.check();
    const findings = report.findings.map((f) => `${f.kind}: ${f.detail}`);
    assert.ok(report.passed(), `${file} lint findings:\n${findings.join("\n")}`);
  });
}
