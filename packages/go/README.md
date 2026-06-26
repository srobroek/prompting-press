# prompting-press (Go) — RESERVED

This directory is a **reserved placeholder**. There is no Go module here yet:
no `go.mod`, no toolchain, no source. It is deliberately **excluded** from the
Cargo workspace (`crates/*` only) and from the moon build/test graph (FR-005/006).

## Why reserved, not implemented

A Go binding is **deferred** (roadmap: Deferred → "Go binding"). When demand and a
solved binding path exist, Go will bind the **same** `prompting-press-core` engine
via cgo-over-C-ABI or WASM (wazero) — never an independent reimplementation
(constitution Principle I / C-01). Spec 006 reserves a conformance target for it.

Until then this README is the only file here, marking the name and intent.
