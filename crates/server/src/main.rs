//! Lattice survival game server.

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    engine_core::logging::init_logging(tracing::Level::INFO, None);
    tracing::info!("Lattice server starting...");
    
    // TODO: Initialize server systems
    
    Ok(())
}
