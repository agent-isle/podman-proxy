use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{bail, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use podman_proxy::{start_proxy, SandboxMount};

/// Launch a Podman socket proxy that enforces a sandbox mount policy.
///
/// The proxy listens on a Unix socket and forwards container create requests
/// to the real Podman socket, rejecting any bind mount that leaks secrets,
/// escapes the sandbox surface, overrides a read-only tree, or does not exist
/// on the host.
#[derive(Parser)]
#[command(name = "podman-proxy", about, version)]
struct Cli {
    /// Path for the proxy's own listening socket.
    #[arg(long, default_value_t = default_listen_path())]
    listen: String,

    /// Path of the real Podman socket to forward to.
    #[arg(long = "real", alias = "podman-socket")]
    real: String,

    /// Host path of a secret file to protect. May be repeated.
    #[arg(long = "secret-file")]
    secret_files: Vec<String>,

    /// Host path authorized for container bind mounts. Append `:ro` to allow
    /// only read-only binds. May be repeated.
    #[arg(long = "allowed-mount")]
    allowed_mounts: Vec<String>,
}

fn default_listen_path() -> String {
    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/0".to_string())
        + "/podman-proxy.sock"
}

/// Parse an `--allowed-mount` argument (`/path` or `/path:ro`) into a
/// [`SandboxMount`].
fn parse_allowed_mount(spec: &str) -> Result<SandboxMount> {
    let (host, read_only) = match spec.split_once(':') {
        Some((host, "ro")) => (host, true),
        Some((host, "")) => (host, false),
        Some((_host, other)) => bail!("invalid allowed-mount option: {other:?} (expected `ro`)"),
        None => (spec, false),
    };
    if !host.starts_with('/') {
        bail!("allowed-mount must be an absolute host path: {spec:?}");
    }
    Ok(SandboxMount {
        host: host.to_string(),
        read_only,
    })
}

/// The proxy shutdown hook, set once before signal handlers are installed.
static SHUTDOWN: std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>> = std::sync::Mutex::new(None);
/// Set to true once a shutdown signal has been received.
static RECEIVED_SIGNAL: AtomicBool = AtomicBool::new(false);

fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let allowed = cli
        .allowed_mounts
        .iter()
        .map(|m| parse_allowed_mount(m))
        .collect::<Result<Vec<_>>>()?;

    if !Path::new(&cli.real).exists() {
        bail!("real socket does not exist: {}", cli.real);
    }

    let stop = start_proxy(&cli.listen, &cli.real, cli.secret_files, allowed)?;
    safe_lock().replace(Box::new(stop));

    tracing::info!(listen = %cli.listen, real = %cli.real, "podman proxy started");

    install_signal_handlers()?;

    // Park until a signal fires the handler, which runs SHUTDOWN and exits.
    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

/// Handle SIGINT/SIGTERM: run the proxy shutdown hook (removing the socket)
/// once, then exit.
unsafe extern "C" fn handle_signal(_: libc::c_int) {
    if RECEIVED_SIGNAL.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(shutdown) = safe_lock().take() {
        shutdown();
    }
    std::process::exit(0);
}

/// Lock SHUTDOWN, recovering from a poisoned mutex.
fn safe_lock() -> std::sync::MutexGuard<'static, Option<Box<dyn FnOnce() + Send>>> {
    match SHUTDOWN.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Install SIGINT/SIGTERM handlers using `sigaction`.
fn install_signal_handlers() -> Result<()> {
    unsafe {
        for &signum in &[libc::SIGINT, libc::SIGTERM] {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = handle_signal as *const () as usize;
            libc::sigemptyset(&mut action.sa_mask);
            if libc::sigaction(signum, &action, std::ptr::null_mut()) != 0 {
                bail!("failed to install signal handler for {signum}");
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("podman-proxy: {e:#}");
        std::process::exit(1);
    }
}
