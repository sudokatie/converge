//! Lattice survival game server.

mod server;

use anyhow::Result;
use server::LatticeServer;

const DEFAULT_PORT: u16 = 27015;
const DEFAULT_SEED: u64 = 12345;

#[tokio::main]
async fn main() -> Result<()> {
    engine_core::logging::init_logging(tracing::Level::INFO, None);
    
    // TODO: Parse command-line arguments for port and seed
    let port = DEFAULT_PORT;
    let seed = DEFAULT_SEED;
    
    tracing::info!("Lattice server starting on port {port} with seed {seed}");
    
    let mut server = LatticeServer::new(port, seed)?;
    server.run()?;
    
    Ok(())
}
