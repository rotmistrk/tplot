//! tplot — Terminal Data Analysis with Lineage Tracking

use clap::Parser;

#[derive(Parser)]
#[command(name = "tplot", about = "Terminal data analysis with lineage tracking")]
struct Cli {
    /// Project directory
    #[arg(default_value = ".")]
    path: std::path::PathBuf,
}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    println!("tplot — not yet implemented");
    Ok(())
}
