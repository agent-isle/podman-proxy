# podman-proxy

## Scope

podman-proxy is a Unix socket proxy that intercepts Podman container `create` requests and enforces a sandbox mount policy.
It rejects bind mounts that leak secrets, escape the sandbox surface, mount a read-only tree read-write, or do not exist on the host.

Concerns managed in this crate:

- **Unix socket proxy** that intercepts Podman container `create` requests
- **Mount policy enforcement** — rejects mounts that leak secrets, escape the sandbox, override read-only trees, or do not exist on the host
- **HTTP request parsing** over Unix sockets
- **Podman API type deserialisation** for container creation payloads, in both supported wire formats (docker-compat `HostConfig` and libpod specgen `mounts`)

## Installation

### NixOS

All dependencies are handled automatically by Nix.

#### System-wide (configuration.nix)

Add podman-proxy as a flake input and configure it in your system:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    podman-proxy.url = "github:agent-isle/podman-proxy";
    podman-proxy.inputs.nixpkgs.follows = nixpkgs;
  };

  outputs = { self, nixpkgs, podman-proxy, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        {
          environment.systemPackages = [ podman-proxy.packages.x86_64-linux.default ];
        }
      ];
    };
  };
}
```

#### Per-user (Home Manager)

Add to your Home Manager configuration:

```nix
{ inputs, ... }:

{
  home.packages = [
    inputs.podman-proxy.packages.x86_64-linux.default
  ];
}
```

Or build and install manually:

```bash
nix build
cp ./result/bin/podman-proxy ~/.local/bin/
```

### Generic Linux

Requires Rust 1.80+.

Install with Cargo:

```bash
cargo install --path .
```

Or build and copy manually:

```bash
cargo build --release
cp target/release/podman-proxy ~/.local/bin/
```

### Post-install verification

```bash
podman-proxy --help
```

This should display the help message with available flags.

## Usage

### Library usage

podman-proxy provides both a **library** (`start_proxy`, `SandboxMount`) for embedding in host applications (e.g. agent-isle) and a standalone **executable** to launch the proxy independently.

```rust
use podman_proxy::{start_proxy, SandboxMount};

let stop = start_proxy(
    "/run/user/1000/agent/podman-proxy.sock", // proxy listen path
    "/run/user/1000/podman/podman.sock",      // real podman socket
    vec!["/home/user/.ssh/id_rsa".to_string()], // secret files to protect
    vec![SandboxMount { host: "/home/user/project".into(), read_only: false }],
)?;
// ... run ...
stop();
```

### Standalone executable

Launch the proxy independently from the command line:

```text
podman-proxy \
  --listen /run/user/1000/podman-proxy.sock \
  --real /run/user/1000/podman/podman.sock \
  --secret-file /home/user/.ssh/id_rsa \
  --allowed-mount /home/user/project \
  --allowed-mount /home/user/config:ro
```

  | Flag                                | Description                                                                                            |
  | ----------------------------------- | ------------------------------------------------------------------------------------------------------ |
  | `--listen <path>`                   | Path for the proxy’s listening socket (default `$XDG_RUNTIME_DIR/podman-proxy.sock`)                   |
  | `--real` / `--podman-socket <path>` | Path of the real Podman socket to forward to                                                           |
  | `--secret-file <path>`              | Host path of a secret file to protect; repeatable                                                      |
  | `--allowed-mount <host>[:ro]`       | Host path authorized for container bind mounts; append `:ro` to allow only read-only binds; repeatable |

A bound source is rejected when it:

1. is or contains a known secret file
2. lies outside the sandbox’s own host mounts (it must be a sandbox mount or a descendant of one)
3. mounts a read-only sandbox tree read-write
4. does not exist on the host (podman would otherwise create it)

Sources are canonicalised (`realpath`) before matching so symlinks and `..` segments cannot bypass the checks.

## Library contract

The crate-root re-exports are the frozen public surface, stable under SemVer:

```rust
pub fn start_proxy(
    listen_path: &str,
    real_path: &str,
    secrets: Vec<String>,
    allowed_mounts: Vec<SandboxMount>,
) -> Result<impl FnOnce()>
```

```rust
pub struct SandboxMount {
    pub host: String,
    pub read_only: bool,
}
```

`SandboxMount` derives `Debug`, `Clone`, `PartialEq`, and `Eq`.
The `proxy` and `types` modules stay reachable, but consumers should import the crate-root re-exports only.

### Semantics

- `listen_path` — Unix socket path the proxy binds; missing parent directories are created and the socket is chmod’d `0700`.
- `real_path` — the real Podman socket all traffic is forwarded to.
- `secrets` — host paths of secret files; a bind source equal to or containing one is rejected.
- `allowed_mounts` — the authorized host surface.
  A bind source must equal an allowed mount or be a descendant of one, must respect the read-only inheritance, and must exist on the host.
  - A source covered by a read-only allowed mount cannot be mounted read-write (the most restrictive covering mount wins).
  - Sources are canonicalized (`realpath`, with a textual fallback) before matching, so symlinks and `..` segments cannot bypass the checks.

### Behavior

Only container `create` requests are validated; all other traffic is forwarded verbatim.
A violating create gets an HTTP `403` with a body `container mounts violate sandbox policy: <reason>`.

### Lifecycle

`start_proxy` spawns background threads and returns a stop closure:

```rust
let stop = start_proxy(...)?;
stop(); // joins the listener and removes the listen socket
```

Consumers call the closure exactly once on teardown, and should wait for the listen socket to appear before connecting clients.

### Version pinning

Consumers pin podman-proxy by git tag, never by branch: tags are immutable, so the pinned revision cannot drift.

Cargo (what agent-isle uses):

```toml
podman-proxy = { git = "https://github.com/agent-isle/podman-proxy", tag = "v0.1.0", optional = true }
```

`Cargo.lock` records the exact commit for the tag; `cargo update -p podman-proxy` advances only within the pinned tag.

Nix:

Nix consumers pin through Cargo, not through a flake input: the git tag is resolved during the vendored cargo fetch (for example, `buildRustPackage`’s `cargoHash` step), exactly as Cargo does.
A flake input pointing at the same tag may be declared for documentation, but it is not consumed by the build and must be kept in sync with the Cargo `tag`.

### Release workflow

1. Bump the version in `Cargo.toml` and `flake.nix`.
2. Tag the release on GitHub: `vX.Y.Z`.
3. Consumers update the Cargo `tag` (and, if they declared a documentation flake input, its ref), then refresh their lockfiles.

## Architecture

### Files

  | File                                                       | Concern                                                   |
  | ---------------------------------------------------------- | --------------------------------------------------------- |
  | [`src/lib.rs.md`](src/lib.rs.md)                           | Crate root, public API                                    |
  | [`src/main.rs.md`](src/main.rs.md)                         | Standalone executable CLI                                 |
  | [`src/proxy.rs.md`](src/proxy.rs.md)                       | Mount policy enforcement, request orchestration           |
  | [`src/transport.rs.md`](src/transport.rs.md)               | Raw socket HTTP forwarding and response writing           |
  | [`src/parse.rs.md`](src/parse.rs.md)                       | HTTP-over-Unix-socket request parser                      |
  | [`src/secret_detection.rs.md`](src/secret_detection.rs.md) | Sandbox mount policy: secrets, allowlist, read-only flags |
  | [`src/http.rs.md`](src/http.rs.md)                         | Podman API route and path utilities                       |
  | [`src/types.rs.md`](src/types.rs.md)                       | Serde types for Podman container create API               |

### Development

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development environment setup, testing, linting, and conventions.
