//! Lattice survival game client.

use anyhow::Result;

fn main() -> Result<()> {
    engine_core::logging::init_logging(tracing::Level::INFO, None);
    tracing::info!("Lattice starting...");
    
    // TODO: Initialize game systems
    
    Ok(())
}
