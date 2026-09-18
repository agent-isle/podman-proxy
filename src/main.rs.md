# `src/main.rs` — standalone executable

Command-line entry point that launches the proxy independently from agent-isle.
Parses `--listen`, `--real`/`--podman-socket`, repeatable `--secret-file`, and repeatable `--allowed-mount <host>[:ro]` via clap, then calls `start_proxy` and parks until SIGINT/SIGTERM, at which point it runs the shutdown hook (removing the socket) and exits.
