//! Requests of the window program `openpacketmon-app` (window and tray icon, see `app/`): the
//! texts of its menu and quitting. It signs in with the access token of the start link, which only
//! the user running openpacketmon can read, sent as `Authorization: Bearer …`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;

use crate::web::i18n;
use crate::web::{AppState, WebError};

/// Name of the window program, installed next to `openpacketmon`
const APP_BINARY: &str = "openpacketmon-app";

fn check(state: &AppState, headers: &HeaderMap) -> Result<(), WebError> {
    if state.auth.has_bearer_token(headers) {
        Ok(())
    } else {
        Err(WebError::Forbidden)
    }
}

/// `GET /app/texts`: the tray menu in the language of the UI
pub async fn texts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BTreeMap<&'static str, String>>, WebError> {
    check(&state, &headers)?;
    let t = i18n::current();
    Ok(Json(
        [
            ("open", "app-open"),
            ("browser", "app-browser"),
            ("quit", "app-quit"),
            ("saved", "app-saved"),
        ]
        .into_iter()
        // the window program puts the path of the download in place of `{path}`
        .map(|(key, id)| (key, t.get1(id, "path", "{path}")))
        .collect(),
    ))
}

/// `POST /app/quit`: stops openpacketmon, like the button in the settings
pub async fn quit(State(state): State<AppState>, headers: HeaderMap) -> Result<(), WebError> {
    check(&state, &headers)?;
    let shutdown = state.shutdown.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        shutdown.notify_one();
    });
    Ok(())
}

/// The window program next to this binary, if it is installed
pub fn app_binary() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?.with_file_name(APP_BINARY);
    path.exists().then_some(path)
}

/// `~/.config/autostart/openpacketmon-app.desktop`: starts the window program in the tray at sign-in
fn autostart_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(base.join("autostart").join(format!("{APP_BINARY}.desktop")))
}

/// Whether the window program starts at sign-in
pub fn autostart_enabled() -> bool {
    autostart_path().is_some_and(|p| p.exists())
}

/// Switches the start at sign-in on or off
pub fn set_autostart(enabled: bool, port: u16) -> std::io::Result<()> {
    let path = autostart_path().ok_or_else(|| std::io::Error::other("no home directory"))?;
    if !enabled {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let app = app_binary().ok_or_else(|| std::io::Error::other("openpacketmon-app is missing"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, autostart_entry(&app, port))
}

fn autostart_entry(app: &std::path::Path, port: u16) -> String {
    let exec = app
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=openpacketmon\n\
         Comment=Network traffic monitor in the tray\n\
         Icon=openpacketmon\n\
         Exec=\"{exec}\" --hidden --port {port}\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n"
    )
}

#[cfg(test)]
mod tests {
    use axum::http::{Request, StatusCode, header};

    use super::*;
    use crate::web::testutil::*;

    fn bearer(method: &str, path: &str, token: &str) -> Request<axum::body::Body> {
        Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, "127.0.0.1:7380")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn the_app_requests_need_the_access_token() {
        let h = harness().await;
        let reply = send(&h.app, bearer("GET", "/app/texts", "wrong")).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
        // a browser session is not enough
        let reply = send(&h.app, get("/app/texts", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
        let reply = send(&h.app, bearer("POST", "/app/quit", "wrong")).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);

        let reply = send(&h.app, bearer("GET", "/app/texts", "token")).await;
        assert_eq!(reply.status, StatusCode::OK);
        let texts: BTreeMap<String, String> = serde_json::from_str(&reply.body).unwrap();
        assert!(texts["saved"].contains("{path}"), "{}", texts["saved"]);
        assert_eq!(texts.len(), 4);

        let quit = h.state.shutdown.clone();
        let reply = send(&h.app, bearer("POST", "/app/quit", "token")).await;
        assert_eq!(reply.status, StatusCode::OK);
        tokio::time::timeout(std::time::Duration::from_secs(2), quit.notified())
            .await
            .unwrap();
    }

    #[test]
    fn the_autostart_entry_starts_hidden_on_the_port() {
        let entry = autostart_entry(std::path::Path::new("/usr/bin/openpacketmon-app"), 7381);
        assert!(entry.contains("Exec=\"/usr/bin/openpacketmon-app\" --hidden --port 7381\n"));
    }
}
