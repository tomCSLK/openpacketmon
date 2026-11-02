// Modified for openpacketmon (2026), see NOTICE. Based on `src/main.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

//! Module containing the entry point of application execution.
//!
//! openpacketmon is a program with a window: closing the window ends the program and the capture.

// used throughout the crate as `crate::…`
use networking::types::info_traffic::InfoTraffic;
use networking::types::service::Service;
use sniffnet_packet_parser::Protocol;

use crate::config::conf::CONF;
use crate::engine::Engine;
use crate::engine::state::EngineState;
use crate::networking::types::capture_context::{CaptureError, CaptureSourcePicklist};
use crate::utils::error_logger::{ErrorLogger, Location};

mod cli;
mod config;
mod countries;
mod dissect;
mod engine;
mod filter;
#[cfg(test)]
mod fuzz;
mod gui;
mod mcp;
mod mmdb;
mod networking;
mod notifications;
mod report;
mod utils;
mod view;

/// Name used for the binary, the configuration directory and the window class
pub const APP_NAME: &str = "openpacketmon";
/// Name shown to the user
pub const APP_TITLE: &str = "openpacketmon";

/// Entry point of application execution
#[allow(clippy::print_stdout, clippy::print_stderr)]
fn main() {
    // an assistant starts this next to the window: it must not read or restore the settings
    if std::env::args_os().nth(1).is_some_and(|arg| arg == "mcp") {
        std::process::exit(mcp::server::run());
    }

    let conf = CONF.clone();
    let args = cli::Args::handle();

    #[cfg(debug_assertions)]
    {
        // kill the main thread as soon as a secondary thread panics
        let orig_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            // invoke the default handler and exit the process
            orig_hook(panic_info);
            std::process::exit(1);
        }));
    }

    let Ok(runtime) = tokio::runtime::Runtime::new().log_err(location!()) else {
        eprintln!("Could not start the async runtime");
        std::process::exit(1);
    };

    if args.list_adapters {
        let engine = Engine::new(conf, runtime.handle().clone());
        print_adapters(&engine.lock());
        return;
    }

    let engine = Engine::new(conf, runtime.handle().clone());
    engine.check_newer_release();
    if !start_requested_capture(&engine, args.adapter, args.import) {
        std::process::exit(1);
    }
    let access = mcp::Access::new(engine.clone(), mcp::access_path());
    access.apply(engine.lock().conf.mcp.enabled);
    let window = gui::run(engine.clone(), access.clone());
    access.apply(false);
    if let Err(e) = window {
        eprintln!("Could not open the window: {e}");
        std::process::exit(1);
    }
    engine.stop();
    engine.store_conf();
    drop(runtime);
}

/// Starts the capture asked for on the command line; `false` if it failed
#[allow(clippy::print_stderr)]
fn start_requested_capture(
    engine: &Engine,
    adapter: Option<String>,
    import: Option<String>,
) -> bool {
    {
        let mut state = engine.lock();
        if let Some(path) = import {
            state.set_capture_source(CaptureSourcePicklist::File);
            state.set_pcap_import(path);
        } else if let Some(adapter) = adapter {
            state.set_capture_source(CaptureSourcePicklist::Device);
            if !state.select_device(&adapter) {
                eprintln!("Unknown network adapter: {adapter}\n");
                print_adapters(&state);
                return false;
            }
        } else {
            return true;
        }
    }
    if engine.start() {
        return true;
    }
    match &engine.lock().capture_error {
        Some(CaptureError::Fatal(e)) => eprintln!("Could not start the capture: {e}"),
        Some(CaptureError::NoIp(link)) => {
            eprintln!("Not started: the source carries no IP traffic (link type {link})");
        }
        _ => eprintln!("Could not start the capture"),
    }
    false
}

#[allow(clippy::print_stdout)]
fn print_adapters(state: &EngineState) {
    println!("Network adapters:");
    for preview in &state.previews {
        let desc = preview.device.get_desc().cloned().unwrap_or_default();
        println!("  {:<20} {desc}", preview.device.get_name());
    }
}
