# podman-proxy contributing

## Development

### Development dependencies

With Nix, enter the dev shell to get all tools:

```bash
nix develop
```

Without Nix, install these tools manually:

  | Tool                                            | Purpose                     |
  | ----------------------------------------------- | --------------------------- |
  | [Rust](https://rustup.rs/) 1.80+                | Compiler                    |
  | [clippy](https://doc.rust-lang.org/clippy/)     | Linting                     |
  | [rustfmt](https://github.com/rust-lang/rustfmt) | Formatting                  |
  | [panache](https://panache.bz)                   | Markdown formatting/linting |
  | [lychee](https://github.com/lycheeverse/lychee) | Link checking               |

### Development build

```bash
cargo build
```

### Running tests

```bash
cargo test
```

### Linting

```bash
cargo clippy --all-targets
cargo fmt --check
panache format --check .
```

### Git hooks

Git hooks enforce formatting, linting, link checking, and tests automatically.

  | Hook         | When               | Checks                                                                      |
  | ------------ | ------------------ | --------------------------------------------------------------------------- |
  | `pre-commit` | Before each commit | `cargo fmt --check`, `cargo clippy`, `panache format --check .`, `lychee .` |
  | `pre-push`   | Before each push   | `cargo test`                                                                |

With Nix, hooks are configured automatically when entering the dev shell.

Without Nix, run the setup script once:

```bash
scripts/setup-githooks.sh
```

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

## Conventions

### Code conventions

- No globals — pass deps explicitly
- `anyhow::Result` for error handling
- `serde` for JSON deserialisation
- `tracing` for structured logging
- rustfmt formatting
- clippy linting
- Table-driven tests
- Per-module `*.rs.md` documentation companions kept in sync with the code
- Source files stay under the 500-line limit enforced by the pre-commit hook

### Updating documentation

Each `.rs` module has a companion `.rs.md` file describing its concern.
When you change a module, update its companion and run:

```bash
panache format .
lychee .
```
