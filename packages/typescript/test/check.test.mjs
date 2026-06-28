/**
 * US3 agreement + provenance lint tests for the TypeScript facade (`prompting-press`) — spec 005, T018.
 *
 * US3 surfaces the shared core's pure analysis pass to TS as `check(reg)`. The lint is performed
 * once, in Rust (Principle I/IV); the binding re-derives nothing and only surfaces the consumer's
 * `CheckReport`, preserving its deterministic finding order. These tests prove every documented
 * finding `kind` is reachable from TS, that the report's protocol behaves (`passed`/`isEmpty`/
 * `findings`), and — critically — that `check` is PURE: it never mutates the registry and never
 * renders (FR-019).
 *
 * Finding-kind / variant facts (from the Rust consumer; mirrored from the spec-004 Python suite):
 *   - `undeclared_variable`     → variant "default" (per-variant; the implicit root arm).
 *   - `untrusted_without_guard` → variant undefined  (a PROMPT-level finding, not per-variant).
 *   - `reserved_variant_name`   → variant "default" (the offending variant key).
 *   - `analysis_error`          → variant "default"; reachable via an excluded feature (`{% include %}`).
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import { z } from "zod";

import { Registry, check, render, PromptingPressError } from "prompting-press";

const KIND_UNDECLARED = "undeclared_variable";
const KIND_UNTRUSTED = "untrusted_without_guard";
const KIND_ANALYSIS = "analysis_error";
const KIND_RESERVED = "reserved_variant_name";

const kinds = (report) => report.findings.map((f) => f.kind);
const signature = (report) => report.findings.map((f) => [f.kind, f.prompt, f.variant ?? null]);

// --------------------------------------------------------------------------------------
// 1. Clean registry passes — the no-findings contract (FR-016 / FR-019 baseline)
// --------------------------------------------------------------------------------------

test("a clean registry passes with an empty report", () => {
  const reg = new Registry();
  reg.insert({
    name: "greet",
    role: "user",
    body: "Hi {{ name }}, you have {{ count }} messages",
    variables: {
      name: { type: "string", provenance: "trusted" },
      count: { type: "integer", provenance: "trusted" },
    },
  });

  const report = check(reg);

  assert.equal(report.passed(), true);
  assert.equal(report.isEmpty(), true);
  assert.deepEqual([...report.findings], []);
});

test("an empty registry yields an empty, passing report", () => {
  const report = check(new Registry());
  assert.equal(report.passed(), true);
  assert.equal(report.findings.length, 0);
});

// --------------------------------------------------------------------------------------
// 2. Undeclared variable — SC-004 / FR-016 (the headline agreement check)
// --------------------------------------------------------------------------------------

test("an undeclared variable is flagged, naming the prompt/variant/variable (SC-004)", () => {
  const reg = new Registry();
  reg.insert({
    name: "ghosty",
    role: "user",
    body: "Hi {{ name }} and {{ ghost }}",
    variables: { name: { type: "string", provenance: "trusted" } },
  });

  const report = check(reg);

  assert.equal(report.passed(), false);
  assert.deepEqual(kinds(report), [KIND_UNDECLARED]);

  const finding = report.findings[0];
  assert.equal(finding.kind, KIND_UNDECLARED);
  assert.equal(finding.prompt, "ghosty");
  assert.equal(finding.variant, "default"); // the implicit root arm is named "default"
  assert.ok(finding.detail.includes("ghost"), finding.detail); // detail names the undeclared var
});

// --------------------------------------------------------------------------------------
// 3. Untrusted without guard — SC-005 / FR-017 (the provenance lint)
// --------------------------------------------------------------------------------------

test("an untrusted variable with no guard is flagged at the prompt level (SC-005)", () => {
  const reg = new Registry();
  reg.insert({
    name: "search",
    role: "user",
    body: "Query: {{ q }}",
    variables: { q: { type: "string", provenance: "untrusted" } },
  });

  const report = check(reg);

  assert.deepEqual(kinds(report), [KIND_UNTRUSTED]);
  const finding = report.findings[0];
  assert.equal(finding.kind, KIND_UNTRUSTED);
  assert.equal(finding.prompt, "search");
  // Prompt-level, not per-variant ⇒ napi surfaces an absent variant as `undefined`.
  assert.equal(finding.variant, undefined);
  assert.ok(finding.detail.includes("q"), finding.detail);
});

for (const guardKey of ["meta", "metadata"]) {
  test(`a \`guard\` key under \`${guardKey}\` clears the untrusted_without_guard finding`, () => {
    const reg = new Registry();
    reg.insert({
      name: "search",
      role: "user",
      body: "Query: {{ q }}",
      variables: { q: { type: "string", provenance: "untrusted" } },
      [guardKey]: { guard: "sanitized upstream" },
    });

    const report = check(reg);

    assert.equal(report.passed(), true, `a guard under \`${guardKey}\` should satisfy the lint`);
    assert.ok(!kinds(report).includes(KIND_UNTRUSTED));
  });
}

// --------------------------------------------------------------------------------------
// 4. Reserved variant name — FR-018 (a variants map keyed literally `default`)
// --------------------------------------------------------------------------------------

test("a variant literally named `default` is flagged as reserved_variant_name", () => {
  const reg = new Registry();
  reg.insert({
    name: "rv",
    role: "user",
    body: "Base {{ x }}",
    variables: { x: { type: "string", provenance: "trusted" } },
    variants: { default: { body: "Variant {{ x }}" } },
  });

  const report = check(reg);

  assert.ok(kinds(report).includes(KIND_RESERVED));
  const reserved = report.findings.find((f) => f.kind === KIND_RESERVED);
  assert.equal(reserved.prompt, "rv");
  assert.equal(reserved.variant, "default");
});

// --------------------------------------------------------------------------------------
// 5. Analysis error — an excluded template feature surfaces a finding, never a crash
// --------------------------------------------------------------------------------------

test("an excluded feature (`{% include %}`) surfaces analysis_error, not a crash", () => {
  const reg = new Registry();
  reg.insert({ name: "ae", role: "user", body: '{% include "x" %}' });

  const report = check(reg); // must not throw

  assert.ok(kinds(report).includes(KIND_ANALYSIS));
  const analysis = report.findings.find((f) => f.kind === KIND_ANALYSIS);
  assert.equal(analysis.prompt, "ae");
  assert.equal(analysis.variant, "default");
});

// --------------------------------------------------------------------------------------
// 6. Purity — FR-019: check mutates nothing and renders nothing
// --------------------------------------------------------------------------------------

test("check is pure: a render is byte-identical before/after, and repeated checks are equal (FR-019)", () => {
  const reg = new Registry();
  reg.insert({
    name: "greet",
    role: "user",
    body: "Hi {{ name }}",
    variables: { name: { type: "string", provenance: "trusted" } },
  });

  const Vars = z.object({ name: z.string() });
  const before = render(reg, "greet", Vars, { name: "Ada" });

  const reportA = check(reg);
  const reportB = check(reg);

  const after = render(reg, "greet", Vars, { name: "Ada" });

  // Render identical after check ⇒ check rendered nothing and mutated nothing.
  assert.equal(after.text, before.text);
  assert.equal(after.text, "Hi Ada");
  assert.equal(after.templateHash, before.templateHash);
  assert.equal(after.renderHash, before.renderHash);

  // Repeated analysis is itself stable (no accumulating state).
  assert.deepEqual(signature(reportA), signature(reportB));
  assert.equal(reportA.passed(), true);
});

// --------------------------------------------------------------------------------------
// 7. Multiple findings / determinism — the consumer's finding order is preserved
// --------------------------------------------------------------------------------------

test("multiple flagged prompts yield deterministic findings (one per prompt)", () => {
  const reg = new Registry();
  // Inserted in a deliberately non-sorted order to prove the report order is the core's.
  reg.insert({ name: "zeta", role: "user", body: "X {{ ghost }}", variables: {} });
  reg.insert({
    name: "alpha",
    role: "user",
    body: "Q {{ q }}",
    variables: { q: { type: "string", provenance: "untrusted" } },
  });
  reg.insert({
    name: "mid",
    role: "user",
    body: "M {{ x }}",
    variables: { x: { type: "string", provenance: "trusted" } },
    variants: { default: { body: "V {{ x }}" } },
  });

  const sig1 = signature(check(reg));
  const sig2 = signature(check(reg));
  const sig3 = signature(check(reg));

  assert.equal(sig1.length, 3, "one finding per flagged prompt");
  // Stable order across calls — the determinism guarantee.
  assert.deepEqual(sig1, sig2);
  assert.deepEqual(sig2, sig3);

  // All three expected violations are present (as a set, so the exact ordering is not over-fit).
  const asSet = new Set(sig1.map((s) => JSON.stringify(s)));
  assert.ok(asSet.has(JSON.stringify([KIND_UNTRUSTED, "alpha", null])));
  assert.ok(asSet.has(JSON.stringify([KIND_RESERVED, "mid", "default"])));
  assert.ok(asSet.has(JSON.stringify([KIND_UNDECLARED, "zeta", "default"])));

  const report = check(reg);
  assert.equal(report.passed(), false);
  assert.equal(report.findings.length, 3);
});

// --------------------------------------------------------------------------------------
// 8. Surface smoke — the US3 check API is exposed where the binding promises it
// --------------------------------------------------------------------------------------

test("the US3 check surface is exposed", () => {
  assert.equal(typeof check, "function");
  const report = check(new Registry());
  for (const attr of ["passed", "isEmpty"]) {
    assert.equal(typeof report[attr], "function");
  }
  assert.ok(Array.isArray(report.findings));
  // A PromptingPressError is the base of the hierarchy (smoke).
  assert.equal(typeof PromptingPressError, "function");
});
