// Modified for openpacketmon (2026), see NOTICE. Based on `src/main.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

//! Module containing the entry point of application execution.
//!
//! openpacketmon runs a local web server (127.0.0.1) and opens the browser with a start link that
//! signs it in. Started a second time, it only opens the browser for the running instance.

use std::sync::Arc;

// used throughout the crate as `crate::…`
use networking::types::info_traffic::InfoTraffic;
use networking::types::service::Service;
use sniffnet_packet_parser::Protocol;
use utils::formatted_strings::print_cli_welcome_message;

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
mod mmdb;
mod networking;
mod notifications;
mod report;
mod utils;
mod web;

/// Name used for the binary, the configuration directory and the window class
pub const APP_NAME: &str = "openpacketmon";
/// Name shown to the user
pub const APP_TITLE: &str = "openpacketmon";

/// Entry point of application execution
#[allow(clippy::print_stdout, clippy::print_stderr)]
fn main() {
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

    let Ok(listener) = runtime.block_on(web::bind(args.port)) else {
        // the port is taken: by a running openpacketmon of this user, or by something else
        if runtime.block_on(web::is_running(args.port))
            && let Some(link) = web::auth::read_start_link(args.port)
        {
            println!("openpacketmon is already running: {link}");
            if args.adapter.is_some() || args.import.is_some() {
                eprintln!("The capture options are ignored; choose the source in the browser.");
            }
            if !args.no_browser {
                open_browser(&link);
            }
            return;
        }
        eprintln!(
            "Port {} on 127.0.0.1 is in use by another program or user; choose another one with --port.",
            args.port
        );
        std::process::exit(1);
    };

    print_cli_welcome_message();
    let engine = Engine::new(conf, runtime.handle().clone());
    engine.check_newer_release();
    if !start_requested_capture(&engine, args.adapter, args.import) {
        std::process::exit(1);
    }

    let auth = Arc::new(web::auth::Auth::new());
    let link = auth.start_link(args.port);
    match web::auth::write_start_link(args.port, &link) {
        Ok(_) => {}
        Err(e) => eprintln!("Could not save the start link: {e}"),
    }
    println!("Open in the browser: {link}");
    println!("Press Ctrl+C to quit.\n");
    if !args.no_browser {
        open_browser(&link);
    }

    let state = web::AppState::new(engine.clone(), auth, args.port);
    if let Err(e) = runtime.block_on(web::serve(state, listener)) {
        eprintln!("The web server stopped: {e}");
    }

    web::auth::remove_start_link(args.port);
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
        _ => eprintln!("Could not start the capture"),
    }
    false
}

/// Opens the start link in the user's browser
#[allow(clippy::print_stderr)]
fn open_browser(link: &str) {
    let opened = std::process::Command::new("xdg-open")
        .arg(link)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    if opened.is_err() {
        eprintln!("Could not open the browser; open the link above yourself.");
    }
}

#[allow(clippy::print_stdout)]
fn print_adapters(state: &EngineState) {
    println!("Network adapters:");
    for preview in &state.previews {
        let desc = preview.device.get_desc().cloned().unwrap_or_default();
        println!("  {:<20} {desc}", preview.device.get_name());
    }
}
