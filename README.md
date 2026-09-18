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
