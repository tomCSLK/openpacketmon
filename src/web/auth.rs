//! Access to the browser UI.
//!
//! The server listens on the loopback interface only, but every local user and every web page the
//! browser opens can reach it. So a random access token is created at each start; it is part of
//! the start link, which only the user running openpacketmon can read (printed on the console and
//! written to a file only they can read, see [`write_start_link`]). Opening the link creates a session
//! (cookie `openpacketmon_session`, `HttpOnly`, `SameSite=Strict`), every form carries the session's
//! CSRF token, and requests with a foreign `Host` (DNS rebinding) or `Origin` are refused.

use std::collections::VecDeque;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use axum::extract::{FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use rand::RngCore;
use serde::Deserialize;
use subtle::ConstantTimeEq;

use crate::web::{AppState, WebError};

/// Name of the session cookie
pub const SESSION_COOKIE: &str = "openpacketmon_session";
/// Sessions kept at the same time; the oldest is dropped when another one is created
const MAX_SESSIONS: usize = 16;

/// A signed-in browser
#[derive(Clone, Debug)]
pub struct Session {
    id: String,
    /// Token of the forms and script requests of this session
    pub csrf_token: String,
}

pub struct Auth {
    access_token: String,
    sessions: Mutex<VecDeque<Session>>,
}

impl Auth {
    pub fn new() -> Self {
        Self::with_token(random_token())
    }

    pub fn with_token(access_token: String) -> Self {
        Self {
            access_token,
            sessions: Mutex::new(VecDeque::new()),
        }
    }

    /// The link that signs the browser in
    pub fn start_link(&self, port: u16) -> String {
        format!("http://127.0.0.1:{port}/login?token={}", self.access_token)
    }

    /// Whether the request carries the access token as `Authorization: Bearer …`: the window
    /// program `openpacketmon-app` reads the start link like the browser and uses it for its menu
    pub fn has_bearer_token(&self, headers: &HeaderMap) -> bool {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|t| bool::from(t.as_bytes().ct_eq(self.access_token.as_bytes())))
    }

    /// A new session if `token` is the access token
    fn login(&self, token: &str) -> Option<Session> {
        if !bool::from(token.as_bytes().ct_eq(self.access_token.as_bytes())) {
            return None;
        }
        let session = Session {
            id: random_token(),
            csrf_token: random_token(),
        };
        let mut sessions = self.lock();
        if sessions.len() >= MAX_SESSIONS {
            sessions.pop_front();
        }
        sessions.push_back(session.clone());
        Some(session)
    }

    /// The session of a request, by its cookie
    pub fn session(&self, headers: &HeaderMap) -> Option<Session> {
        let id = cookie(headers, SESSION_COOKIE)?;
        self.lock()
            .iter()
            .find(|s| bool::from(s.id.as_bytes().ct_eq(id.as_bytes())))
            .cloned()
    }

    fn logout(&self, headers: &HeaderMap) {
        if let Some(id) = cookie(headers, SESSION_COOKIE) {
            self.lock().retain(|s| s.id != id);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, VecDeque<Session>> {
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Session {
    /// Checks the CSRF token sent with a form or a script request
    pub fn check_csrf(&self, token: &str) -> Result<(), WebError> {
        if bool::from(token.as_bytes().ct_eq(self.csrf_token.as_bytes())) {
            Ok(())
        } else {
            Err(WebError::Forbidden)
        }
    }
}

/// 32 random bytes, hex-encoded
fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(64), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    })
}

/// The value of a cookie
fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(n, _)| *n == name)
        .map(|(_, v)| v)
}

/// The session of a signed-in browser; handlers that take it are only reachable after signing in.
/// Pages without a session show how to get the start link, script requests get a plain 401.
impl FromRequestParts<AppState> for Session {
    type Rejection = Response;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        std::future::ready(state.auth.session(&parts.headers).ok_or_else(|| {
            if parts.uri.path().starts_with("/api/") {
                StatusCode::UNAUTHORIZED.into_response()
            } else {
                let lang = state.lang(&parts.headers);
                crate::web::pages::signed_out(lang).into_response()
            }
        }))
    }
}

#[derive(Deserialize)]
pub struct LoginQuery {
    #[serde(default)]
    token: String,
}

/// `GET /login?token=…`: signs the browser in and continues without the token in the address
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<LoginQuery>,
) -> Response {
    let Some(session) = state.auth.login(&query.token) else {
        let lang = state.lang(&headers);
        return crate::web::pages::signed_out(lang).into_response();
    };
    let cookie = format!(
        "{SESSION_COOKIE}={}; Path=/; HttpOnly; SameSite=Strict",
        session.id
    );
    let mut response = Redirect::to("/").into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

#[derive(Deserialize)]
pub struct CsrfForm {
    #[serde(default)]
    pub csrf_token: String,
}

/// `POST /logout`
pub async fn logout(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    axum::Form(form): axum::Form<CsrfForm>,
) -> Result<Response, WebError> {
    session.check_csrf(&form.csrf_token)?;
    state.auth.logout(&headers);
    let mut response = crate::web::pages::signed_out(state.lang(&headers)).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "openpacketmon_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict",
        ),
    );
    Ok(response)
}

/// Whether a `Host` header names this server (loopback address or `localhost` with our port).
/// Anything else is a page that resolved its own name to 127.0.0.1 (DNS rebinding).
pub fn is_own_host(host: &str, port: u16) -> bool {
    let Some((name, host_port)) = host.rsplit_once(':') else {
        return false;
    };
    host_port == port.to_string() && matches!(name, "127.0.0.1" | "localhost" | "[::1]")
}

/// Whether an `Origin` header (sent by browsers with form posts and script requests) is this server
pub fn is_own_origin(origin: &str, port: u16) -> bool {
    origin
        .strip_prefix("http://")
        .is_some_and(|host| is_own_host(host, port))
}

/// Where the start link of the server on `port` is written for a second start of `openpacketmon`
/// (start menu entry): the user's runtime directory (a private tmpfs), or the configuration directory
/// without one. One file per port, so servers on other ports (tests, a second profile) do not mix.
pub fn start_link_path(port: u16) -> Option<PathBuf> {
    let dir = match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join(crate::APP_NAME),
        _ => confy::get_configuration_file_path(crate::APP_NAME, "conf")
            .ok()?
            .parent()?
            .to_path_buf(),
    };
    Some(dir.join(format!("start-link-{port}")))
}

/// Writes the start link to [`start_link_path`], readable by the current user only
pub fn write_start_link(port: u16, link: &str) -> std::io::Result<PathBuf> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

    let path = start_link_path(port).ok_or_else(|| std::io::Error::other("no directory"))?;
    if let Some(dir) = path.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
    }
    // a fresh file, so an old one with other permissions cannot be reused
    let _ = std::fs::remove_file(&path);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    writeln!(file, "{link}")?;
    Ok(path)
}

/// The start link written by the server running on `port`, if any
pub fn read_start_link(port: u16) -> Option<String> {
    let text = std::fs::read_to_string(start_link_path(port)?).ok()?;
    let link = text.trim();
    link.starts_with(&format!("http://127.0.0.1:{port}/"))
        .then(|| link.to_owned())
}

/// Removes the start link when the server on `port` stops
pub fn remove_start_link(port: u16) {
    if let Some(path) = start_link_path(port) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_cookie(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn only_the_access_token_creates_sessions() {
        let auth = Auth::with_token("secret".into());
        assert!(auth.login("").is_none());
        assert!(auth.login("secre").is_none());
        assert!(auth.login("secret2").is_none());
        let session = auth.login("secret").unwrap();
        assert_eq!(session.id.len(), 64);
        assert_ne!(session.id, session.csrf_token);

        let headers = headers_with_cookie(&format!("other=1; {SESSION_COOKIE}={}", session.id));
        let found = auth.session(&headers).unwrap();
        assert_eq!(found.csrf_token, session.csrf_token);
        assert!(found.check_csrf(&session.csrf_token).is_ok());
        assert!(found.check_csrf("").is_err());
        assert!(
            auth.session(&headers_with_cookie("openpacketmon_session=x"))
                .is_none()
        );

        auth.logout(&headers);
        assert!(auth.session(&headers).is_none());
    }

    #[test]
    fn old_sessions_are_dropped() {
        let auth = Auth::with_token("t".into());
        let first = auth.login("t").unwrap();
        for _ in 0..MAX_SESSIONS {
            auth.login("t").unwrap();
        }
        let headers = headers_with_cookie(&format!("{SESSION_COOKIE}={}", first.id));
        assert!(auth.session(&headers).is_none());
        assert_eq!(auth.lock().len(), MAX_SESSIONS);
    }

    #[test]
    fn foreign_hosts_and_origins_are_recognized() {
        assert!(is_own_host("127.0.0.1:7380", 7380));
        assert!(is_own_host("localhost:7380", 7380));
        assert!(is_own_host("[::1]:7380", 7380));
        assert!(!is_own_host("127.0.0.1:7381", 7380));
        assert!(!is_own_host("127.0.0.1", 7380));
        assert!(!is_own_host("evil.example:7380", 7380));
        assert!(is_own_origin("http://127.0.0.1:7380", 7380));
        assert!(!is_own_origin("https://127.0.0.1:7380", 7380));
        assert!(!is_own_origin("http://evil.example:7380", 7380));
        assert!(!is_own_origin("null", 7380));
    }

    #[test]
    fn start_links_carry_the_token() {
        let auth = Auth::with_token("abc".into());
        assert_eq!(
            auth.start_link(7380),
            "http://127.0.0.1:7380/login?token=abc"
        );
    }
}
