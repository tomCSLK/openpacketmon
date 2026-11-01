//! The browser UI: an HTTP server on the loopback interface that shows the [`Engine`]'s data.
//!
//! Pages are rendered on the server (askama templates in `templates/`, texts from `locales/`);
//! the scripts in `static/` only update live values (charts, tiles, tables) and never need a CDN.

mod api;
mod app;
mod assets;
pub mod auth;
mod capture;
mod connections;
mod entries;
mod format;
pub mod i18n;
mod jump;
mod notifications;
mod overview;
mod packets;
mod pager;
mod pages;
mod settings;
mod statistics;
mod stream;
mod suggest;

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::engine::Engine;
use crate::view::filter_index::FilterIndex;
use crate::web::auth::Auth;
use crate::web::i18n::Lang;

/// Port of the server unless `--port` says otherwise
pub const DEFAULT_PORT: u16 = 7380;
/// Program version shown in the sidebar
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Where the source code is published
pub const SOURCE_URL: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(Clone)]
pub struct AppState {
    pub engine: Engine,
    pub auth: Arc<Auth>,
    pub port: u16,
    /// Stops the server (button in the settings)
    pub shutdown: Arc<Notify>,
    /// The packets that pass the display filters in use
    pub packet_index: Arc<FilterIndex>,
}

impl AppState {
    pub fn new(engine: Engine, auth: Arc<Auth>, port: u16) -> Self {
        Self {
            engine,
            auth,
            port,
            shutdown: Arc::new(Notify::new()),
            packet_index: Arc::default(),
        }
    }

    /// The language of the pages: the one chosen in the settings, otherwise the browser's
    pub fn lang(&self, headers: &HeaderMap) -> Lang {
        if let Some(lang) = Lang::parse(&self.engine.lock().conf.language) {
            return lang;
        }
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .map_or_else(Lang::default, Lang::from_accept_language)
    }
}

/// Errors of the handlers, shown as an error page
#[derive(Debug)]
pub enum WebError {
    /// Missing or wrong CSRF token, foreign origin
    Forbidden,
    NotFound,
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let t = i18n::current();
        let (status, message) = match self {
            Self::Forbidden => (StatusCode::FORBIDDEN, t.get("error-forbidden")),
            Self::NotFound => (StatusCode::NOT_FOUND, t.get("error-not-found")),
        };
        (status, pages::error_page(t, status, message)).into_response()
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(capture::home))
        .route("/login", get(auth::login))
        .route("/logout", post(auth::logout))
        .route("/capture/start", post(capture::start))
        .route("/capture/stop", post(capture::stop))
        .route("/capture/pause", post(capture::pause))
        .route("/overview", get(overview::page))
        .route("/connections", get(connections::list))
        .route("/connection", get(connections::detail))
        .route("/connection/latency", post(connections::latency))
        .route("/packets", get(packets::list))
        .route("/packet", get(packets::detail))
        .route("/packets.pcap", get(packets::download))
        .route("/stream", get(stream::page))
        .route("/statistics", get(statistics::page))
        .route("/hosts", get(entries::hosts))
        .route("/services", get(entries::services))
        .route("/programs", get(entries::programs))
        .route("/favorites", post(entries::toggle_favorite))
        .route("/notifications", get(notifications::page))
        .route("/notifications/clear", post(notifications::clear))
        .route("/settings", get(settings::page).post(settings::save))
        .route("/settings/geo", post(settings::download_geo))
        .route("/display", post(settings::display))
        .route("/quit", post(settings::quit))
        .route("/app/texts", get(app::texts))
        .route("/app/quit", post(app::quit))
        .route("/api/live", get(api::live))
        .route("/api/previews", get(api::previews))
        .route("/api/packets", get(packets::rows_api))
        .route("/api/filter", get(packets::check))
        .route("/api/filter/names", get(suggest::names))
        .route("/api/filter/values", get(suggest::values))
        .route("/licenses", get(licenses))
        .route("/healthz", get(healthz))
        .route("/static/{*path}", get(assets::serve))
        .fallback(not_found)
        .layer(security_header(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(security_header(header::X_FRAME_OPTIONS, "DENY"))
        // not `no-referrer`: browsers then send `Origin: null` with forms, and the origin check fails
        .layer(security_header(header::REFERRER_POLICY, "same-origin"))
        .layer(security_header(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'",
        ))
        // pages show traffic data: never keep them in a cache (static files set their own header)
        .layer(security_header(header::CACHE_CONTROL, "no-store"))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            guard_and_language,
        ))
        .with_state(state)
}

fn security_header(name: HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(name, HeaderValue::from_static(value))
}

/// Refuses requests for another host name (DNS rebinding) and state-changing requests from
/// other origins, then runs the request in the language of the pages
async fn guard_and_language(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !auth::is_own_host(host, state.port) {
        return (StatusCode::MISDIRECTED_REQUEST, "unknown host").into_response();
    }
    if request.method() != Method::GET
        && request.method() != Method::HEAD
        && let Some(origin) = headers.get(header::ORIGIN)
        && !origin
            .to_str()
            .is_ok_and(|o| auth::is_own_origin(o, state.port))
    {
        return (StatusCode::FORBIDDEN, "foreign origin").into_response();
    }
    let lang = state.lang(headers);
    i18n::scope(lang, next.run(request)).await
}

/// Answers whether this is a running openpacketmon (used by a second start); exposes nothing else
async fn healthz() -> &'static str {
    crate::APP_NAME
}

/// The licenses of openpacketmon and of the software and data it uses (`THIRD-PARTY-LICENSES.txt`)
async fn licenses(_session: crate::web::auth::Session) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        crate::view::LICENSES,
    )
}

async fn not_found() -> WebError {
    WebError::NotFound
}

/// Longest wait for the header of a request: connections of a local program that never finishes
/// its requests would otherwise stay open (with their memory) for good
const HEADER_TIMEOUT: Duration = Duration::from_secs(10);

/// Starts the server on `127.0.0.1:port` and runs it until Ctrl+C, SIGTERM or the quit button
pub async fn serve(state: AppState, listener: tokio::net::TcpListener) -> std::io::Result<()> {
    let quit = state.shutdown.clone();
    let app = router(state);
    let mut http = hyper::server::conn::http1::Builder::new();
    http.timer(hyper_util::rt::TokioTimer::new())
        .header_read_timeout(HEADER_TIMEOUT);
    let stop = async move {
        tokio::select! {
            () = shutdown_signal() => {},
            () = quit.notified() => {},
        }
    };
    tokio::pin!(stop);
    loop {
        let accepted = tokio::select! {
            () = &mut stop => return Ok(()),
            accepted = listener.accept() => accepted,
        };
        let Ok((stream, _)) = accepted else {
            // e.g. no file descriptors left: try again later instead of spinning
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        let service = hyper_util::service::TowerToHyperService::new(app.clone());
        let connection = http.serve_connection(hyper_util::rt::TokioIo::new(stream), service);
        tokio::spawn(async move {
            let _ = connection.await;
        });
    }
}

/// Binds the server's socket; fails if the port is taken (e.g. by another running instance)
pub async fn bind(port: u16) -> std::io::Result<tokio::net::TcpListener> {
    tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

/// Whether an openpacketmon server answers on `port` (another instance of this user or another user)
pub async fn is_running(port: u16) -> bool {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let Ok(Ok(mut stream)) = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tokio::net::TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port))),
    )
    .await
    else {
        return false;
    };
    let request =
        format!("GET /healthz HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).await.is_err() {
        return false;
    }
    let mut response = Vec::new();
    let read = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream.read_to_end(&mut response),
    )
    .await;
    read.is_ok() && String::from_utf8_lossy(&response).ends_with(crate::APP_NAME)
}

#[cfg(test)]
pub(crate) mod testutil {
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;
    use crate::config::conf::Conf;

    pub const PORT: u16 = 7380;
    pub const HOST: &str = "127.0.0.1:7380";

    pub struct Harness {
        pub app: Router,
        pub state: AppState,
        /// Cookie of a signed-in session
        pub cookie: String,
        pub csrf: String,
    }

    pub struct Reply {
        pub status: StatusCode,
        pub headers: axum::http::HeaderMap,
        pub body: String,
    }

    /// A server with a fresh engine and a signed-in session
    pub async fn harness() -> Harness {
        let engine = Engine::new(Conf::default(), tokio::runtime::Handle::current());
        let state = AppState::new(engine, Arc::new(Auth::with_token("token".into())), PORT);
        let app = router(state.clone());
        let login = send(&app, get("/login?token=token", None)).await;
        assert_eq!(login.status, StatusCode::SEE_OTHER);
        let cookie = login.headers[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let csrf = state.auth.session(&{
            let mut h = HeaderMap::new();
            h.insert(header::COOKIE, cookie.parse().unwrap());
            h
        });
        Harness {
            app,
            state,
            cookie,
            csrf: csrf.unwrap().csrf_token,
        }
    }

    pub fn get(path: &str, cookie: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri(path).header(header::HOST, HOST);
        if let Some(cookie) = cookie {
            builder = builder.header(header::COOKIE, cookie);
        }
        builder.body(Body::empty()).unwrap()
    }

    pub fn post(path: &str, cookie: &str, form: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(path)
            .header(header::HOST, HOST)
            .header(header::COOKIE, cookie)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(form.to_owned()))
            .unwrap()
    }

    pub async fn send(app: &Router, request: Request<Body>) -> Reply {
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            headers,
            body: String::from_utf8_lossy(&body).into_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{Request, StatusCode, header};

    use super::testutil::*;

    #[tokio::test]
    async fn the_licenses_are_shown() {
        let h = harness().await;
        let reply = send(&h.app, get("/licenses", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.body.starts_with("openpacketmon\n"));
        for part in ["Giuliano Bellini", "uPlot 1.6.32", "IANA", "DB-IP", "axum "] {
            assert!(reply.body.contains(part), "{part}");
        }
    }

    #[tokio::test]
    async fn pages_need_a_session() {
        let h = harness().await;
        for path in [
            "/",
            "/overview",
            "/connections",
            "/hosts",
            "/settings",
            "/licenses",
        ] {
            let reply = send(&h.app, get(path, None)).await;
            assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{path}");
            assert!(reply.body.contains("<code>openpacketmon</code>"), "{path}");
            let reply = send(&h.app, get(path, Some(&h.cookie))).await;
            assert!(
                reply.status.is_success() || reply.status.is_redirection(),
                "{path}"
            );
        }
        let api = send(&h.app, get("/api/live", None)).await;
        assert_eq!(api.status, StatusCode::UNAUTHORIZED);
        assert!(api.body.is_empty());
    }

    #[tokio::test]
    async fn a_wrong_token_does_not_sign_in() {
        let h = harness().await;
        let reply = send(&h.app, get("/login?token=wrong", None)).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert!(reply.headers.get(header::SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn foreign_hosts_and_origins_are_refused() {
        let h = harness().await;
        let rebinding = Request::builder()
            .uri("/login?token=token")
            .header(header::HOST, "attacker.example:7380")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(
            send(&h.app, rebinding).await.status,
            StatusCode::MISDIRECTED_REQUEST
        );

        let mut cross_site = post(
            "/capture/stop",
            &h.cookie,
            &format!("csrf_token={}", h.csrf),
        );
        cross_site
            .headers_mut()
            .insert(header::ORIGIN, "http://attacker.example".parse().unwrap());
        assert_eq!(send(&h.app, cross_site).await.status, StatusCode::FORBIDDEN);

        let mut own_origin = post(
            "/capture/stop",
            &h.cookie,
            &format!("csrf_token={}", h.csrf),
        );
        own_origin
            .headers_mut()
            .insert(header::ORIGIN, "http://127.0.0.1:7380".parse().unwrap());
        assert_eq!(send(&h.app, own_origin).await.status, StatusCode::SEE_OTHER);

        let without_token = post("/capture/stop", &h.cookie, "csrf_token=wrong");
        assert_eq!(
            send(&h.app, without_token).await.status,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn responses_carry_security_headers() {
        let h = harness().await;
        let reply = send(&h.app, get("/healthz", None)).await;
        assert_eq!(reply.body, "openpacketmon");
        assert_eq!(reply.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(reply.headers[header::X_FRAME_OPTIONS], "DENY");
        assert_eq!(reply.headers[header::CACHE_CONTROL], "no-store");
        assert_eq!(reply.headers[header::REFERRER_POLICY], "same-origin");
        let csp = reply.headers[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap();
        assert!(csp.contains("default-src 'self'"), "{csp}");
        assert_eq!(
            send(&h.app, get("/nope", Some(&h.cookie))).await.status,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn the_server_recognizes_a_running_instance() {
        let listener = super::bind(0).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let h = harness().await;
        let mut state = h.state.clone();
        state.port = port;
        tokio::spawn(super::serve(state, listener));
        assert!(super::is_running(port).await);
        let free = super::bind(0).await.unwrap();
        let free_port = free.local_addr().unwrap().port();
        drop(free);
        assert!(!super::is_running(free_port).await);
    }
}
