//! The answers for the filter fields of the browser pages, see [`crate::view::suggest`].

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::view::suggest::{Catalog, ValueRow, catalog, value_rows};
use crate::web::AppState;
use crate::web::auth::Session;
use crate::web::i18n;

#[derive(Deserialize)]
pub struct NamesQuery {
    /// `connections` for the connection filter, else the packet filter
    #[serde(default)]
    kind: String,
}

/// `GET /api/filter/names`: names, kinds of value and operators of a filter
pub async fn names(_session: Session, Query(query): Query<NamesQuery>) -> Json<Catalog> {
    Json(catalog(i18n::current(), query.kind == "connections"))
}

#[derive(Deserialize)]
pub struct ValuesQuery {
    #[serde(default)]
    field: String,
    /// Part of the value, any case
    #[serde(default)]
    q: String,
}

/// `GET /api/filter/values?field=…&q=…`: values of a field of the connection filter, the most
/// frequent first
pub async fn values(
    State(state): State<AppState>,
    _session: Session,
    Query(query): Query<ValuesQuery>,
) -> Json<Vec<ValueRow>> {
    Json(value_rows(&state.engine.lock(), &query.field, &query.q))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::web::testutil::*;

    #[tokio::test]
    async fn suggestions_need_a_session() {
        let h = harness().await;
        let reply = send(&h.app, get("/api/filter/names", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.body.contains("tls.sni"), "{}", reply.body);
        let reply = send(
            &h.app,
            get("/api/filter/names?kind=connections", Some(&h.cookie)),
        )
        .await;
        assert!(reply.body.contains("country"), "{}", reply.body);
        assert!(!reply.body.contains("tls.sni"), "{}", reply.body);
        let reply = send(
            &h.app,
            get("/api/filter/values?field=country&q=d", Some(&h.cookie)),
        )
        .await;
        assert_eq!(reply.body, "[]");
        let reply = send(&h.app, get("/api/filter/values?field=x", Some(&h.cookie))).await;
        assert_eq!(reply.body, "[]");
        for path in ["/api/filter/names", "/api/filter/values?field=country"] {
            assert_ne!(send(&h.app, get(path, None)).await.status, StatusCode::OK);
        }
    }
}
