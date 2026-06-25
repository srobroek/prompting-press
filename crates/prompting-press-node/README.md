# prompting-press-node

The **napi-rs (Node-API) binding crate** (cdylib) — the second of exactly two
crates permitted to depend on an FFI toolkit (`napi`/`napi-derive`); the kernel
and Rust consumer stay FFI-free (Principle II / C-02). It exposes the Rust consumer
surface to Node.js as a native addon; `packages/typescript/` wraps it into an npm
package.

Spec 001 ships this as a **stub** (a trivial `#[napi]` function). Marshaling + the
Zod facade land in spec 005.
