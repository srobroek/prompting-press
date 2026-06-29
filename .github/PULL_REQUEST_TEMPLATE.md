## Summary

<!-- One or two sentences. What does this change and why? -->

## Binding(s) changed

<!-- Check all that apply -->
- [ ] Core engine (`prompting-press-core`)
- [ ] Rust consumer (`prompting-press`)
- [ ] Python (`prompting-press-py`)
- [ ] TypeScript (`prompting-press-node`)
- [ ] Docs site
- [ ] Build / CI / schemas

## Checklist

- [ ] PR title follows conventional commits (`feat:`, `fix:`, `chore:`, etc.) — it becomes the changelog entry on squash-merge
- [ ] `cargo test` passes (Rust / core)
- [ ] `pytest` passes (Python)
- [ ] `node test` / `pnpm test` passes (TypeScript)
- [ ] Schema codegen gates pass (if schema changed)
- [ ] Docs updated if public API surface changed
- [ ] No publishing step triggered (publishing is gated to maintainers via release-please)
