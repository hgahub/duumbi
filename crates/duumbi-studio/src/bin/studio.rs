//! DUUMBI Studio standalone binary.
//!
//! Starts the Studio web platform as a standalone server.
//! Run via: `cargo run -p duumbi-studio --features ssr --bin studio -- --port 8421`

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::path::PathBuf;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let port: u16 = std::env::args()
        .position(|a| a == "--port")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|p| p.parse().ok())
        .unwrap_or(8421);

    let workspace = PathBuf::from(
        std::env::args()
            .position(|a| a == "--workspace")
            .and_then(|i| std::env::args().nth(i + 1))
            .unwrap_or_else(|| ".".to_string()),
    );

    if !workspace.join(".duumbi").exists() {
        eprintln!(
            "Error: No duumbi workspace found at '{}'. Run `duumbi init` first.",
            workspace.display()
        );
        std::process::exit(1);
    }

    if let Err(e) = duumbi_studio::start_server(port, workspace).await {
        eprintln!("Studio error: {e:#}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!(
        "This binary requires the 'ssr' feature. Run with: cargo run -p duumbi-studio --features ssr --bin studio"
    );
    std::process::exit(1);
}
