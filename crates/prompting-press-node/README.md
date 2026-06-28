# prompting-press-node

The **napi-rs (Node-API) binding crate** (cdylib) — the second of exactly two
crates permitted to depend on an FFI toolkit (`napi`/`napi-derive`); the kernel
and Rust consumer stay FFI-free (Principle II / C-02). It exposes the Rust consumer
surface to Node.js as a native addon; `packages/typescript/` wraps it into an npm
package.

It contains the `#[napi]` marshaling layer over the kernel/consumer (registry,
render, getSource, check, composition) — marshaling + delegation only, **zero engine
logic** (render/agreement/variant/hash live once in the shared core, Principle I).
The Zod facade + the public API live in `packages/typescript/`. See that package's
README for the consumer-facing docs.
