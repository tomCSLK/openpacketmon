//! The page of "Follow stream".

use askama::Template;
use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;

use crate::view::Jump;
use crate::view::packets::conversation_filter;
use crate::view::stream::{ShownChunk, StreamSide, StreamView, stream};
use crate::web::auth::Session;
use crate::web::i18n::{self, Lang};
use crate::web::jump::Href;
use crate::web::pages::{Layout, render};
use crate::web::{AppState, WebError};

#[derive(Template)]
#[template(path = "stream.html")]
struct StreamPage {
    t: Lang,
    layout: Layout,
    number: u64,
    title: String,
    client: String,
    server: String,
    client_counts: String,
    server_counts: String,
    chunks: Vec<ShownChunk>,
    view: &'static str,
    side: &'static str,
    cut: bool,
    headers_only: bool,
    packets_href: String,
    base: String,
}

#[derive(Deserialize)]
pub struct StreamQuery {
    n: u64,
    #[serde(default)]
    view: String,
    #[serde(default)]
    side: String,
}

/// `GET /stream?n=…`: the connection of packet `n` as a stream
pub async fn page(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<StreamQuery>,
) -> Result<Response, WebError> {
    let t = i18n::current();
    let layout = Layout::new(&state.engine.lock(), &session);
    let (view, view_code) = if query.view == "hex" {
        (StreamView::Hex, "hex")
    } else {
        (StreamView::Text, "text")
    };
    let (side, side_code) = match query.side.as_str() {
        "client" => (StreamSide::Client, "client"),
        "server" => (StreamSide::Server, "server"),
        _ => (StreamSide::Both, "both"),
    };
    let s = stream(t, &state.engine, query.n, view, side).ok_or(WebError::NotFound)?;
    let page = StreamPage {
        t,
        layout,
        number: query.n,
        title: s.title,
        client: s.client,
        server: s.server,
        client_counts: s.client_counts,
        server_counts: s.server_counts,
        chunks: s.chunks,
        view: view_code,
        side: side_code,
        cut: s.cut,
        headers_only: s.headers_only,
        packets_href: Jump::Packets {
            filter: conversation_filter(&s.connection),
            connections: String::new(),
        }
        .href(),
        base: format!("/stream?n={}", query.n),
    };
    Ok(render(&page))
}
