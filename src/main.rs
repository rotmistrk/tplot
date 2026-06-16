//! tplot — Terminal Data Analysis with Lineage Tracking

mod app;
mod engine;
mod handler;
mod help;
#[allow(dead_code)]
mod jobs;
mod lineage_data;
mod node_behavior;
#[allow(dead_code)]
mod node_state;
mod plot;
mod registry;
#[allow(dead_code)]
mod scripting;
mod slots;
mod sql_analysis;
mod status;
mod views;
mod workspace;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use txv_core::program::Program;
use txv_render::backend::CrosstermBackend;
use txv_render::ColorMode;

use crate::app::AppState;
use crate::handler::handle_command;
use crate::status::build_status_bar;
use crate::workspace::build_workspace;

#[derive(Parser)]
#[command(name = "tplot", about = "Terminal data analysis with lineage tracking")]
struct Cli {
    /// Project directory
    #[arg(default_value = ".")]
    path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root_dir = fs::canonicalize(&cli.path)?;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let ws = build_workspace(&root_dir);
    let status = build_status_bar(&ws);
    let mut program = Program::new(Box::new(status), Box::new(ws));
    let mut app_state = AppState::new(root_dir);

    let mut backend = CrosstermBackend::new(ColorMode::TrueColor);
    program.run(&mut backend, |ctx| {
        handle_command(ctx, &mut app_state);
    });

    Ok(())
}
