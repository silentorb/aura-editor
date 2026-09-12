//! Process logging for the editor binary.
//!
//! Defaults to appending under `./logs/` in the process working directory.
//! Override verbosity with `RUST_LOG` (default: `info`).

use std::fs::{self, OpenOptions};
use std::path::Path;
use std::sync::Mutex;

use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

const LOG_DIR: &str = "logs";
const LOG_FILE: &str = "aura-editor.log";

/// Creates `./logs` if needed and installs a tracing subscriber (file + stderr).
///
/// Safe to call once at process start. Subsequent calls are ignored.
pub fn init_logging() {
    let log_dir = Path::new(LOG_DIR);
    if let Err(error) = fs::create_dir_all(log_dir) {
        eprintln!("aura-editor: failed to create {LOG_DIR}/: {error}");
    }

    let log_path = log_dir.join(LOG_FILE);
    let file_layer = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(file) => Some(fmt::layer().with_ansi(false).with_writer(Mutex::new(file))),
        Err(error) => {
            eprintln!(
                "aura-editor: failed to open {}: {error}",
                log_path.display()
            );
            None
        }
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(LevelFilter::INFO.to_string()));

    let stderr_layer = fmt::layer().with_writer(std::io::stderr);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer);

    let result = if let Some(file_layer) = file_layer {
        registry.with(file_layer).try_init()
    } else {
        registry.try_init()
    };

    if let Err(error) = result {
        eprintln!("aura-editor: logging already initialized: {error}");
        return;
    }

    install_panic_hook();
    tracing::info!(
        log_path = %log_path.display(),
        "logging initialized"
    );
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "application panic");
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_paths_are_under_working_directory_logs() {
        assert_eq!(LOG_DIR, "logs");
        assert_eq!(Path::new(LOG_DIR).join(LOG_FILE), Path::new("logs/aura-editor.log"));
    }
}
