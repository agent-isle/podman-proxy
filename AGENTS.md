# podman-proxy agent reference

## Session Setup

Before working on this project, you must load the development environment.
This makes all required tools (`cargo`, `clippy`, `rustfmt`, `panache`, `lychee`) available in your PATH.

Run this before any other command:

```bash
source scripts/ai-dev-env.sh
```

This is the only way to access the development tools if you are inside a Nix shell.
If your environment does not persist variables between tool invocations, source the script and run the command in the same invocation, e.g.:

```bash
source scripts/ai-dev-env.sh && cargo build
```

If `ai-dev-env.sh` does not exist yet, generate it from the flake (run once):

```bash
nix print-dev-env > ./scripts/ai-dev-env.sh
```

### Constraints

- Do not run `nix` commands unless you have permission.
- Do not run `git` commands — the user operates git.

## Scope

- Unix socket proxy that intercepts Podman container `create` requests
- Mount policy enforcement — rejects mounts that leak secrets, escape the authorized surface, override read-only trees, or do not exist on the host
- HTTP request parsing over Unix sockets
- Podman API type deserialisation for container creation payloads, in both supported wire formats

## Architecture

### Project Structure

```
src/
  lib.rs                Crate root, deny lints, public API (start_proxy, SandboxMount)
  main.rs               Standalone executable CLI
  proxy.rs              Mount policy enforcement, request orchestration
  transport.rs          Raw socket HTTP forwarding and response writing
  parse.rs              HTTP-over-Unix-socket request parser
  http.rs               Podman API route and path utilities
  secret_detection.rs   Sandbox mount policy: secrets, allowlist, read-only flags
  types.rs              Serde types for the Podman container create API
  *.rs.md               Per-module documentation companions
tests/
  proxy_integration.rs  End-to-end proxy behavior tests
  proxy_eof_integration.rs  Keep-alive / EOF propagation tests
  support/              Mock Podman backend helpers
flake.nix               Nix build
```

### Public API

The library exposes two items at the crate root:

- `start_proxy(listen_path, real_path, secrets, allowed_mounts) -> Result<impl FnOnce()>`
- `SandboxMount { host: String, read_only: bool }`

The standalone executable (`src/main.rs`) wraps `start_proxy` behind a clap CLI.

### Podman mount policy

A bind source is rejected when it:

1. is or contains a known secret file
2. lies outside the authorized host mounts (it must be an allowed mount or a descendant of one)
3. mounts a read-only allowed mount read-write
4. does not exist on the host (podman would otherwise create it)

Sources are canonicalised (`realpath`) before matching so symlinks and `..` segments cannot bypass the checks.

## Development

### Commands

```bash
cargo build                            # development build
cargo test                             # test
cargo clippy --all-targets             # lint
cargo fmt                              # format
panache format .                       # format markdown
```

### Git hooks

Git hooks enforce formatting, linting, link checking, and tests automatically.

  | Hook         | When               | Checks                                                                      |
  | ------------ | ------------------ | --------------------------------------------------------------------------- |
  | `pre-commit` | Before each commit | `cargo fmt --check`, `cargo clippy`, `panache format --check .`, `lychee .` |
  | `pre-push`   | Before each push   | `cargo test`                                                                |

Run the setup script once:

```bash
scripts/setup-githooks.sh
```

## Conventions

### Updating documentation

Each `.rs` module has a companion `.rs.md` file describing its concern.
When you change a module, update its companion and run:

```bash
source scripts/ai-dev-env.sh
panache format .
lychee .
```
