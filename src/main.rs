//! tplot — Terminal Data Analysis with Lineage Tracking

mod app;
mod completion_source;
#[allow(dead_code)]
mod completions;
mod engine;
mod handler;
mod help;
#[allow(dead_code)]
mod jobs;
mod lineage_data;
pub mod mcp;
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
use crate::mcp::socket_path::socket_path;
use crate::status::build_status_bar;
use crate::workspace::build_workspace;

#[derive(Parser)]
#[command(name = "tplot", about = "Terminal data analysis with lineage tracking")]
struct Cli {
    /// Project directory
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Run as MCP bridge (stdin↔socket proxy) and exit
    #[arg(long = "mcp-server")]
    mcp_server: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.mcp_server {
        return mcp::bridge::run_mcp_bridge().map_err(|e| anyhow::anyhow!("{e}"));
    }

    let root_dir = fs::canonicalize(&cli.path)?;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(Box::new(
            std::fs::File::create(root_dir.join(".tplot.log"))
                .unwrap_or_else(|_| std::fs::File::create("/tmp/tplot.log").expect("log file")),
        )))
        .init();

    // Start MCP socket listener
    let sock_path = socket_path(&root_dir);
    let mcp_active = mcp::server::start_mcp_listener(&sock_path);
    if let Ok(ref p) = mcp_active {
        std::env::set_var("TPLOT_MCP_SOCKET", p.to_string_lossy().as_ref());
        log::info!("MCP server listening on {}", p.display());
    }

    let ws = build_workspace(&root_dir);
    let status = build_status_bar(&ws);
    let mut program = Program::new(Box::new(status), Box::new(ws));
    program.insert_named(
        "sidekick",
        Box::new(txv_widgets::sidekick_manager::SidekickManager::new()),
    );
    let mut app_state = AppState::new(root_dir);

    // Initial lineage tree refresh from loaded registry.
    {
        let desktop = program.desktop_mut();
        crate::handler::initial_refresh(desktop, &app_state.registry);
    }

    let mut backend = CrosstermBackend::new(ColorMode::TrueColor);
    program.run(&mut backend, |ctx| {
        handle_command(ctx, &mut app_state);
    });

    // Cleanup socket
    if mcp_active.is_ok() {
        let _ = fs::remove_file(&sock_path);
    }

    Ok(())
}
