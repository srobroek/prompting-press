# Security Policy

## Supported versions

Security fixes are applied to the latest released version of each package
(`prompting-press`, `prompting-press-py`, `prompting-press-node`). Older
versions do not receive backported security patches.

## Scope

Prompting Press is a **pure computation library** — it parses and renders
prompt templates. It performs no I/O, makes no network calls, and does not
execute the prompts it produces. The primary security-relevant surface is:

- **Template rendering** (MiniJinja): malicious template content passed to `render()`.
- **Input validation** (Pydantic / Zod / garde): type-coercion or bypass issues in the Vars layer.
- **FFI boundary** (PyO3 / napi-rs): memory-safety issues in the binding layer.
- **Value scrubbing** (SEC-004 posture): fields tagged `sensitive` should be excluded from provenance hashes and logs — a bypass of this is a security issue.

Out of scope: vulnerabilities in the *output* prompt text or in whatever LLM
or service the caller sends it to — that is outside the library's boundary.

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Report privately via [GitHub's private vulnerability reporting](https://github.com/prompting-press/prompting-press/security/advisories/new)
(Settings → Security → Advisories → "Report a vulnerability").

Include:
- A description of the vulnerability and its impact.
- Steps to reproduce or a minimal proof-of-concept.
- Which package(s) and version(s) are affected.

You will receive an acknowledgement within **5 business days**. We aim to
release a fix within **30 days** for confirmed issues, sooner for critical ones.
We will coordinate disclosure timing with you.
