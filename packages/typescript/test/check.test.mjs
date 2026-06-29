/**
 * US3 advisory lint tests for the TypeScript facade (`prompting-press`) — spec 005, T018.
 * Updated for spec 008 object surface: uses `Prompt.check()` instead of `check(reg)`.
 *
 * US3 surfaces the shared core's pure analysis pass to TS as `prompt.check()`. The lint is
 * performed once, in Rust (Principle I/IV); the binding re-derives nothing. These tests prove
 * the advisory finding class is reachable from TS, that the report's protocol behaves
 * (`passed`/`isEmpty`/`findings`), and that `check()` is PURE: never mutates, never renders.
 *
 * Post-reshape (spec 008 / R7 / Q4): construction enforces the **hard** invariants (agreement,
 * parse, reserved-name). The only LIVE finding `check()` can surface is:
 *   - `untrusted_without_guard` — a prompt with untrusted/external vars and no guard configured.
 *
 * All fixtures use `origin` (spec 008 rename from `provenance`).
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import { Prompt, PromptingPressError } from "prompting-press";

const KIND_UNTRUSTED = "untrusted_without_guard";

const kinds = (report) => report.findings.map((f) => f.kind);

// ─── 1. Clean prompt passes ───────────────────────────────────────────────────────────────

test("a trusted-only prompt passes with an empty report", () => {
  const p = new Prompt({
    name: "greet",
    role: "user",
    body: "Hi {{ name }}",
    variables: { name: { type: "string", origin: "trusted" } },
  });
  const report = p.check();
  assert.ok(report.passed(), "trusted-only prompt must pass");
  assert.ok(report.isEmpty(), "no findings ⇒ isEmpty()");
  assert.deepEqual(report.findings, []);
});

// ─── 2. Untrusted-without-guard advisory ─────────────────────────────────────────────────

test("an untrusted variable without a guard produces untrusted_without_guard finding", () => {
  const p = new Prompt({
    name: "ask",
    role: "user",
    body: "{{ topic }}",
    variables: { topic: { type: "string", origin: "untrusted" } },
  });
  const report = p.check();
  assert.ok(!report.passed(), "unguarded untrusted var must fail the lint");
  assert.ok(
    kinds(report).includes(KIND_UNTRUSTED),
    `expected ${KIND_UNTRUSTED}, got ${kinds(report)}`,
  );
  const f = report.findings[0];
  assert.equal(f.prompt, "ask");
  assert.ok(f.detail.includes("topic"), `detail must mention the field, got: ${f.detail}`);
});

test("check() passes when a guard is configured in meta", () => {
  const p = new Prompt({
    name: "guarded",
    role: "user",
    body: "{{ payload }}",
    variables: { payload: { type: "string", origin: "untrusted" } },
    meta: { guard: { enabled: true } },
  });
  assert.ok(p.check().passed(), "guard configured → check must pass");
});

test("check() passes when a guard is configured in metadata", () => {
  const p = new Prompt({
    name: "guarded_meta",
    role: "user",
    body: "{{ payload }}",
    variables: { payload: { type: "string", origin: "untrusted" } },
    metadata: { guard: { enabled: true } },
  });
  assert.ok(p.check().passed(), "guard in metadata → check must pass");
});

// ─── 3. Purity (FR-019) ──────────────────────────────────────────────────────────────────

test("check() is pure: calling it twice returns the same result (no mutation)", () => {
  const p = new Prompt({
    name: "ask",
    role: "user",
    body: "{{ topic }}",
    variables: { topic: { type: "string", origin: "untrusted" } },
  });
  const r1 = p.check();
  const r2 = p.check();
  assert.equal(r1.passed(), r2.passed());
  assert.equal(r1.findings.length, r2.findings.length);
  assert.equal(r1.findings[0]?.kind, r2.findings[0]?.kind);
});

// ─── 4. Report protocol ──────────────────────────────────────────────────────────────────

test("report.passed() and report.isEmpty() are aliases", () => {
  const p = new Prompt({
    name: "greet",
    role: "user",
    body: "Hi",
    variables: {},
  });
  const report = p.check();
  assert.equal(report.passed(), report.isEmpty());
});

test("report.findings is an array of Finding objects with stable fields", () => {
  const p = new Prompt({
    name: "ask",
    role: "user",
    body: "{{ topic }}",
    variables: { topic: { type: "string", origin: "untrusted" } },
  });
  const report = p.check();
  assert.ok(Array.isArray(report.findings));
  const f = report.findings[0];
  assert.equal(typeof f.prompt, "string");
  assert.equal(typeof f.kind, "string");
  assert.equal(typeof f.detail, "string");
});

// ─── 5. Surface smoke ────────────────────────────────────────────────────────────────────

test("check() is a method on a Prompt instance", () => {
  const p = new Prompt({ name: "greet", role: "user", body: "hi", variables: {} });
  assert.equal(typeof p.check, "function");
});

test("PromptingPressError is exported", () => {
  assert.ok(PromptingPressError.prototype instanceof Error);
});
