//! openpacketmon as a desktop application: the browser UI of the openpacketmon server in a window of
//! its own, with an icon in the tray.
//!
//! The window is a separate program on purpose: `/usr/bin/openpacketmon` has the capture
//! capabilities (`cap_net_raw`, `cap_net_admin`), and the web engine should not run with them. This
//! program has no special rights. It starts the server if none is running for the port (or uses the
//! one of the systemd user service), signs in with the start link the server writes for its user, and
//! talks to the server only over HTTP on 127.0.0.1.
//!
//! - Closing the window hides it; the capture goes on and the tray icon stays.
//! - The tray menu opens the window or the browser, or quits openpacketmon (server included).
//! - When the server stops (quit in the settings, service stopped), this program ends as well.
//! - A second start shows the window of the running instance (Unix socket in the runtime directory).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::platform::unix::WindowExtUnix;
use tao::window::{Icon, Window, WindowBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};
use wry::{NewWindowResponse, WebContext, WebView, WebViewBuilder, WebViewBuilderExtUnix};

/// Name of the server binary, the configuration and the runtime directory
const APP_NAME: &str = "openpacketmon";
const DEFAULT_PORT: u16 = 7380;
/// How long a newly started server may take until it answers
const START_TIMEOUT: Duration = Duration::from_secs(20);
/// Icon of the window and the tray (RGBA)
const ICON_PNG: &[u8] = include_bytes!("../../static/favicon-64.png");

/// What the event loop is asked to do, from the tray, the web view or other threads
#[derive(Debug)]
enum UserEvent {
    Show,
    Browser,
    Quit,
    /// The server does not answer any more
    ServerGone,
    /// A file was downloaded (PCAP) to this path
    Downloaded(PathBuf),
    /// A link to another site: opened in the user's browser
    External(String),
}

struct Args {
    port: u16,
    /// Start in the tray only (autostart at sign-in)
    hidden: bool,
}

#[allow(clippy::print_stdout)]
fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        port: DEFAULT_PORT,
        hidden: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--hidden" => args.hidden = true,
            "--port" => {
                args.port = it
                    .next()
                    .and_then(|p| p.parse().ok())
                    .ok_or("--port needs a port number")?;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: openpacketmon-app [--port PORT] [--hidden]\n\n\
                     Shows openpacketmon in a window with a tray icon; starts the server if needed.\n\
                     --hidden  start with the tray icon only\n\
                     Without a desktop, start the server with `openpacketmon` and use the browser."
                );
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("openpacketmon-app {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }
    Ok(args)
}

/// The user's runtime directory for openpacketmon (as the server uses it for the start link)
fn runtime_dir() -> Option<PathBuf> {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir).join(APP_NAME)),
        _ => config_dir(),
    }
}

/// `~/.config/openpacketmon`
fn config_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(base.join(APP_NAME))
}

/// `~/.local/share/openpacketmon/webview`: cookies and cache of the web view
fn webview_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(std::env::var_os("HOME")?)
            .join(".local")
            .join("share"),
    };
    Some(base.join(APP_NAME).join("webview"))
}

/// The start link of the server on `port`, written by the server for its user only
fn read_start_link(port: u16) -> Option<String> {
    let path = runtime_dir()?.join(format!("start-link-{port}"));
    let text = std::fs::read_to_string(path).ok()?;
    let link = text.trim();
    link.starts_with(&format!("http://127.0.0.1:{port}/login?token="))
        .then(|| link.to_owned())
}

/// A small HTTP/1.1 request to the server; status and body
fn http(port: u16, method: &str, path: &str, token: Option<&str>) -> Option<(u16, String)> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .ok()?;
    let auth = token.map_or_else(String::new, |t| format!("Authorization: Bearer {t}\r\n"));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: 0\r\n\
         Accept-Language: {}\r\n{auth}\r\n",
        desktop_language()
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    let status = response.split(' ').nth(1)?.parse().ok()?;
    let body = response
        .split_once("\r\n\r\n")
        .map_or_else(String::new, |(_, b)| b.to_owned());
    Some((status, body))
}

/// The language of the desktop (`de_DE.UTF-8` → `de`), for the texts of the server when the UI
/// follows the browser language
fn desktop_language() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|var| std::env::var(var).ok())
        .find_map(|v| language_code(&v))
        .unwrap_or_else(|| "en".to_owned())
}

/// The language of a locale value: `de_DE.UTF-8` → `de`; nothing for `C`, `POSIX` or an empty value
fn language_code(locale: &str) -> Option<String> {
    if locale == "C" || locale == "POSIX" {
        return None;
    }
    let code: String = locale
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    (!code.is_empty()).then_some(code)
}

/// Whether an openpacketmon server answers on `port`
fn server_running(port: u16) -> bool {
    http(port, "GET", "/healthz", None)
        .is_some_and(|(status, body)| status == 200 && body == APP_NAME)
}

/// The server binary next to this program (`/usr/bin/openpacketmon`, or `target/…/openpacketmon`)
fn server_binary() -> PathBuf {
    std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name(APP_NAME))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(APP_NAME))
}

/// Starts the server if none answers on the port and waits for its start link
fn ensure_server(port: u16) -> Result<String, String> {
    if !server_running(port) {
        Command::new(server_binary())
            .args(["--no-browser", "--port", &port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // its own process group: keeps running when this program or its terminal ends
            .process_group(0)
            .spawn()
            .map_err(|e| format!("could not start {}: {e}", server_binary().display()))?;
    }
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        if server_running(port)
            && let Some(link) = read_start_link(port)
        {
            return Ok(link);
        }
        if Instant::now() > deadline {
            return Err(format!(
                "openpacketmon does not answer on 127.0.0.1:{port} (port used by another program?)"
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Only one instance per port: a second start asks the first one to show its window
fn single_instance(port: u16) -> Option<UnixListener> {
    let dir = runtime_dir()?;
    let path = dir.join(format!("app-{port}.sock"));
    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ = stream.write_all(b"show\n");
        std::process::exit(0);
    }
    let _ = std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir);
    let _ = std::fs::remove_file(&path);
    UnixListener::bind(&path).ok()
}

fn listen_for_show(listener: UnixListener, proxy: EventLoopProxy<UserEvent>) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            drop(stream);
            if proxy.send_event(UserEvent::Show).is_err() {
                break;
            }
        }
    });
}

/// Ends this program when the server has stopped answering for a few seconds
fn watch_server(port: u16, proxy: EventLoopProxy<UserEvent>) {
    std::thread::spawn(move || {
        let mut failures = 0;
        loop {
            std::thread::sleep(Duration::from_secs(2));
            failures = if server_running(port) {
                0
            } else {
                failures + 1
            };
            if failures >= 3 {
                let _ = proxy.send_event(UserEvent::ServerGone);
                break;
            }
        }
    });
}

/// Texts of the tray menu in the language of the UI (from the server), English without it
fn menu_texts(port: u16, token: &str) -> HashMap<String, String> {
    let mut texts: HashMap<String, String> = [
        ("open", "Open openpacketmon"),
        ("browser", "Open in the browser"),
        ("quit", "Quit openpacketmon"),
        ("saved", "Saved to {path}"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    if let Some((200, body)) = http(port, "GET", "/app/texts", Some(token))
        && let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&body)
    {
        for (key, value) in map {
            if let serde_json::Value::String(text) = value {
                texts.insert(key, text);
            }
        }
    }
    texts
}

fn icon_rgba() -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(ICON_PNG));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    Some((buf, info.width, info.height))
}

/// Opens a link in the user's browser; only web addresses, `xdg-open` would start the program
/// of any other scheme (`file:`, `smb:` …)
fn open_external(url: &str) {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return;
    }
    let _ = Command::new("xdg-open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// A window with a web view showing `url`; links to other sites open in the browser
fn open_window(
    target: &EventLoopWindowTarget<UserEvent>,
    context: &mut WebContext,
    builder: WindowBuilder,
    url: &str,
    base: &str,
    proxy: &EventLoopProxy<UserEvent>,
) -> Option<(Window, WebView)> {
    let mut builder = builder;
    if let Some((rgba, w, h)) = icon_rgba() {
        builder = builder.with_window_icon(Icon::from_rgba(rgba, w, h).ok());
    }
    let window = builder.build(target).ok()?;
    let navigation = {
        let (base, proxy) = (base.to_owned(), proxy.clone());
        move |url: String| {
            if url.starts_with(&base) || url.starts_with("about:") {
                true
            } else {
                let _ = proxy.send_event(UserEvent::External(url));
                false
            }
        }
    };
    let new_window = {
        let (base, proxy) = (base.to_owned(), proxy.clone());
        move |url: String, _features| {
            let event = if url.starts_with(&base) {
                UserEvent::Show
            } else {
                UserEvent::External(url)
            };
            let _ = proxy.send_event(event);
            NewWindowResponse::Deny
        }
    };
    let downloaded = {
        let proxy = proxy.clone();
        move |_url: String, path: Option<PathBuf>, ok: bool| {
            if ok && let Some(path) = path {
                let _ = proxy.send_event(UserEvent::Downloaded(path));
            }
        }
    };
    let webview = WebViewBuilder::new_with_web_context(context)
        .with_url(url)
        .with_navigation_handler(navigation)
        .with_new_window_req_handler(new_window)
        .with_download_started_handler(|_url, _path| true)
        .with_download_completed_handler(downloaded)
        .with_clipboard(true)
        .build_gtk(window.default_vbox()?)
        .ok()?;
    Some((window, webview))
}

/// Shows a short note at the bottom of the page (download finished)
fn toast(webview: &WebView, text: &str) {
    let Ok(text) = serde_json::to_string(text) else {
        return;
    };
    let _ = webview.evaluate_script(&format!(
        "(function(){{var t=document.createElement('div');t.className='app-toast';t.textContent={text};\
         document.body.appendChild(t);setTimeout(function(){{t.remove();}},6000);}})();"
    ));
}

fn home_relative(path: &Path) -> String {
    match std::env::var_os("HOME").map(PathBuf::from) {
        Some(home) if path.starts_with(&home) => path.strip_prefix(&home).map_or_else(
            |_| path.display().to_string(),
            |p| format!("~/{}", p.display()),
        ),
        _ => path.display().to_string(),
    }
}

#[allow(clippy::print_stderr, clippy::too_many_lines)]
fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("openpacketmon-app: {e}");
            std::process::exit(2);
        }
    };
    let port = args.port;
    let listener = single_instance(port);
    let link = match ensure_server(port) {
        Ok(link) => link,
        Err(e) => {
            eprintln!("openpacketmon-app: {e}");
            std::process::exit(1);
        }
    };
    let token = link
        .split_once("token=")
        .map(|(_, t)| t.to_owned())
        .unwrap_or_default();
    let base = format!("http://127.0.0.1:{port}/");
    let texts = menu_texts(port, &token);
    let text = move |key: &str| texts.get(key).cloned().unwrap_or_default();

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    if let Some(listener) = listener {
        listen_for_show(listener, proxy.clone());
    }
    watch_server(port, proxy.clone());

    let mut context = WebContext::new(webview_dir());
    let main_builder = WindowBuilder::new()
        .with_title("openpacketmon")
        .with_inner_size(LogicalSize::new(1440.0, 900.0))
        .with_min_inner_size(LogicalSize::new(820.0, 560.0))
        .with_visible(!args.hidden);
    let Some((main_window, main_view)) = open_window(
        &event_loop,
        &mut context,
        main_builder,
        &link,
        &base,
        &proxy,
    ) else {
        eprintln!("openpacketmon-app: could not open the window");
        std::process::exit(1);
    };

    let menu = Menu::new();
    let open_item = MenuItem::new(text("open"), true, None);
    let browser_item = MenuItem::new(text("browser"), true, None);
    let quit_item = MenuItem::new(text("quit"), true, None);
    let _ = menu.append_items(&[
        &open_item,
        &browser_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ]);
    {
        let ids = [
            open_item.id().clone(),
            browser_item.id().clone(),
            quit_item.id().clone(),
        ];
        // the handler must be `Sync`, the proxy is not
        let proxy = std::sync::Mutex::new(proxy.clone());
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let event = match ids.iter().position(|id| *id == event.id) {
                Some(0) => UserEvent::Show,
                Some(1) => UserEvent::Browser,
                Some(2) => UserEvent::Quit,
                _ => return,
            };
            if let Ok(proxy) = proxy.lock() {
                let _ = proxy.send_event(event);
            }
        }));
    }
    let mut tray: Option<TrayIcon> = None;
    let mut menu = Some(menu);

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                // the tray icon needs the running GTK loop
                let mut builder = TrayIconBuilder::new().with_tooltip("openpacketmon");
                if let Some(menu) = menu.take() {
                    builder = builder.with_menu(Box::new(menu));
                }
                if let Some((rgba, w, h)) = icon_rgba()
                    && let Ok(icon) = tray_icon::Icon::from_rgba(rgba, w, h)
                {
                    builder = builder.with_icon(icon);
                }
                match builder.build() {
                    Ok(icon) => tray = Some(icon),
                    Err(e) => eprintln!("openpacketmon-app: no tray icon: {e}"),
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if tray.is_some() {
                    // the capture goes on; the tray icon brings the window back
                    main_window.set_visible(false);
                } else {
                    // without a tray icon the window could not be opened again
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(UserEvent::Show) => {
                main_window.set_visible(true);
                main_window.set_minimized(false);
                main_window.set_focus();
            }
            Event::UserEvent(UserEvent::Browser) => open_external(&link),
            Event::UserEvent(UserEvent::External(url)) => open_external(&url),
            Event::UserEvent(UserEvent::Downloaded(path)) => {
                let note = text("saved").replace("{path}", &home_relative(&path));
                toast(&main_view, &note);
            }
            Event::UserEvent(UserEvent::Quit) => {
                let _ = http(port, "POST", "/app/quit", Some(&token));
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::ServerGone) => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_language_comes_from_the_locale() {
        assert_eq!(language_code("de_DE.UTF-8").as_deref(), Some("de"));
        assert_eq!(language_code("en").as_deref(), Some("en"));
        assert_eq!(language_code("C"), None);
        assert_eq!(language_code(""), None);
    }

    #[test]
    fn paths_in_the_home_directory_are_shortened() {
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return;
        };
        assert_eq!(
            home_relative(&home.join("Downloads/a.pcap")),
            "~/Downloads/a.pcap"
        );
        assert_eq!(home_relative(Path::new("/tmp/a.pcap")), "/tmp/a.pcap");
    }

    #[test]
    fn the_icon_decodes_to_rgba() {
        let (rgba, w, h) = icon_rgba().unwrap();
        assert_eq!((w, h), (64, 64));
        assert_eq!(rgba.len(), 64 * 64 * 4);
    }
}
