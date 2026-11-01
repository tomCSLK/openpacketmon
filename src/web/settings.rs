//! The settings page and the unit selection of the page heads.

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::{Redirect, Response};
use serde::Deserialize;

use crate::mmdb::geo::{self, DownloadError};
use crate::networking::types::data_representation::DataUnit;
use crate::view::settings::{
    Choice, GeoRow, Settings, SettingsChange, apply_settings, parse_addresses, parse_sound,
    settings,
};
use crate::web::auth::{CsrfForm, Session};
use crate::web::capture::back_path;
use crate::web::i18n::{self, Lang};
use crate::web::pages::{Layout, flash_message, render};
use crate::web::{AppState, WebError};

fn parse_unit(code: &str) -> Option<DataUnit> {
    match code {
        "bytes" => Some(DataUnit::Bytes),
        "bits" => Some(DataUnit::Bits),
        "packets" => Some(DataUnit::Packets),
        _ => None,
    }
}

#[derive(Template)]
#[template(path = "settings.html")]
#[allow(clippy::struct_excessive_bools)] // switches of a page or a command line
struct SettingsPage {
    t: Lang,
    layout: Layout,
    flash: Option<String>,
    languages: Vec<Choice>,
    page_sizes: Vec<(usize, bool)>,
    favorite_active: bool,
    favorite_sounds: Vec<Choice>,
    favorite_addresses: String,
    /// Typed words that were no address (after saving)
    invalid_addresses: Option<String>,
    volume: u8,
    mmdb_country: String,
    mmdb_asn: String,
    geo_rows: Vec<GeoRow>,
    geo_dir: String,
    /// The download failed or is still running
    geo_error: Option<String>,
    update_checks: bool,
    packets_enabled: bool,
    packets_max: Vec<(String, usize, bool)>,
    packets_headers: bool,
    /// The window program is installed: offer its start at sign-in
    app_installed: bool,
    autostart: bool,
}

#[derive(Deserialize)]
pub struct PageQuery {
    msg: Option<String>,
    /// Words of the favorite addresses that were not saved
    invalid: Option<String>,
    /// Outcome of a failed download of the geolocation databases
    geo: Option<String>,
}

/// `GET /settings`
pub async fn page(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PageQuery>,
) -> Response {
    let t = i18n::current();
    let engine = state.engine.lock();
    let Settings {
        languages,
        page_sizes,
        favorite_active,
        favorite_sounds,
        favorite_addresses,
        volume,
        mmdb_country,
        mmdb_asn,
        geo_rows,
        geo_dir,
        update_checks,
        packets_enabled,
        packets_max,
        packets_headers,
    } = settings(t, &engine);
    render(&SettingsPage {
        t,
        layout: Layout::new(&engine, &session),
        flash: flash_message(t, query.msg.as_deref()),
        languages,
        page_sizes,
        favorite_active,
        favorite_sounds,
        favorite_addresses,
        invalid_addresses: query
            .invalid
            .filter(|i| !i.is_empty())
            .map(|i| t.get1("settings-addresses-invalid", "words", i)),
        volume,
        mmdb_country,
        mmdb_asn,
        geo_rows,
        geo_dir,
        geo_error: match query.geo.as_deref() {
            Some("busy") => Some(t.get("settings-geo-busy")),
            Some(_) => Some(t.get("settings-geo-failed")),
            None => None,
        },
        update_checks,
        packets_enabled,
        packets_max,
        packets_headers,
        app_installed: crate::web::app::app_binary().is_some(),
        autostart: crate::web::app::autostart_enabled(),
    })
}

#[derive(Deserialize)]
pub struct SettingsForm {
    #[serde(default)]
    csrf_token: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    page_size: usize,
    favorite_active: Option<String>,
    #[serde(default)]
    favorite_sound: String,
    #[serde(default)]
    favorite_addresses: String,
    #[serde(default)]
    volume: u8,
    #[serde(default)]
    mmdb_country: String,
    #[serde(default)]
    mmdb_asn: String,
    update_checks: Option<String>,
    packets_enabled: Option<String>,
    #[serde(default)]
    packets_max: usize,
    packets_headers: Option<String>,
    /// Only sent when the window program is installed
    autostart_shown: Option<String>,
    autostart: Option<String>,
}

/// `POST /settings`
pub async fn save(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<SettingsForm>,
) -> Result<Redirect, WebError> {
    session.check_csrf(&form.csrf_token)?;
    let (addresses, invalid) = parse_addresses(&form.favorite_addresses);
    apply_settings(
        &mut state.engine.lock(),
        &SettingsChange {
            language: form.language,
            page_size: form.page_size,
            favorite_active: form.favorite_active.is_some(),
            favorite_sound: parse_sound(&form.favorite_sound),
            addresses,
            volume: form.volume,
            mmdb_country: form.mmdb_country,
            mmdb_asn: form.mmdb_asn,
            update_checks: form.update_checks.is_some(),
            packets_enabled: form.packets_enabled.is_some(),
            packets_max: form.packets_max,
            packets_memory: None,
            packets_headers: form.packets_headers.is_some(),
        },
    );
    if form.autostart_shown.is_some()
        && form.autostart.is_some() != crate::web::app::autostart_enabled()
        && let Err(e) = crate::web::app::set_autostart(form.autostart.is_some(), state.port)
    {
        #[allow(clippy::print_stderr)]
        {
            eprintln!("Could not change the start at sign-in: {e}");
        }
    }
    state.engine.store_conf();
    if invalid.is_empty() {
        Ok(Redirect::to("/settings?msg=settings_saved"))
    } else {
        Ok(Redirect::to(&format!(
            "/settings?msg=settings_saved&invalid={}#notifications",
            crate::web::entries::encode(&invalid.join(" "))
        )))
    }
}

/// `POST /settings/geo`: downloads the DB-IP Lite databases and uses them
pub async fn download_geo(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<CsrfForm>,
) -> Result<Redirect, WebError> {
    session.check_csrf(&form.csrf_token)?;
    match geo::download_dbip().await {
        Ok(()) => {
            state.engine.lock().reload_geo();
            Ok(Redirect::to("/settings?msg=geo_downloaded#geo"))
        }
        Err(DownloadError::Busy) => Ok(Redirect::to("/settings?geo=busy#geo")),
        Err(DownloadError::Failed(e)) => {
            #[allow(clippy::print_stderr)]
            {
                eprintln!("Download of the geolocation databases failed: {e}");
            }
            Ok(Redirect::to("/settings?geo=failed#geo"))
        }
    }
}

#[derive(Deserialize)]
pub struct DisplayForm {
    #[serde(default)]
    csrf_token: String,
    #[serde(default)]
    unit: String,
    #[serde(default)]
    back: String,
}

#[derive(Template)]
#[template(path = "quit.html")]
struct QuitPage {
    t: Lang,
}

/// `POST /quit`: stops openpacketmon (the capture ends, the settings are saved)
pub async fn quit(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<CsrfForm>,
) -> Result<Response, WebError> {
    session.check_csrf(&form.csrf_token)?;
    // a moment later, so the browser can still load the logo and styles of the page
    let shutdown = state.shutdown.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        shutdown.notify_one();
    });
    Ok(render(&QuitPage { t: i18n::current() }))
}

/// `POST /display`: the unit of the volumes (bytes, bits or packets)
pub async fn display(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<DisplayForm>,
) -> Result<Redirect, WebError> {
    session.check_csrf(&form.csrf_token)?;
    if let Some(unit) = parse_unit(&form.unit) {
        state.engine.lock().conf.data_repr.data_unit = unit;
        state.engine.store_conf();
    }
    Ok(Redirect::to(&back_path(&form.back, "/overview")))
}

#[cfg(test)]
mod tests {
    use axum::http::{StatusCode, header};

    use super::*;
    use crate::notifications::types::sound::Sound;
    use crate::web::testutil::*;

    #[tokio::test]
    async fn settings_are_saved() {
        let h = harness().await;
        let reply = send(&h.app, get("/settings", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        let form = format!(
            "csrf_token={}&language=de&page_size=100&favorite_active=on&favorite_sound=gulp&favorite_addresses=10.0.0.2%0D%0A192.168.1.20+nonsense&volume=150&packets_max=250000&packets_headers=on",
            h.csrf
        );
        let reply = send(&h.app, post("/settings", &h.cookie, &form)).await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER);
        assert_eq!(
            reply.headers[header::LOCATION],
            "/settings?msg=settings_saved&invalid=nonsense#notifications"
        );
        {
            let engine = h.state.engine.lock();
            let conf = &engine.conf;
            assert_eq!(conf.language, "de");
            assert_eq!(conf.page_size, 100);
            let n = &conf.settings.notifications;
            assert!(n.favorite_notification.is_active);
            assert_eq!(n.favorite_notification.sound, Sound::Gulp);
            assert_eq!(n.volume, 100);
            assert_eq!(conf.favorites.addresses().len(), 2);
            assert!(conf.favorites.contains_address(&[192, 168, 1, 20].into()));
            assert!(conf.updates.disable_checks());
            assert!(!conf.packet_store.enabled);
            assert_eq!(conf.packet_store.max_packets, 250_000);
            assert!(conf.packet_store.headers_only);
        }
        // the pages follow the chosen language
        let reply = send(&h.app, get("/settings", Some(&h.cookie))).await;
        assert!(reply.body.contains("<html lang=\"de\">"));

        let form = format!("csrf_token={}&unit=packets&back=%2Fhosts", h.csrf);
        let reply = send(&h.app, post("/display", &h.cookie, &form)).await;
        assert_eq!(reply.headers[header::LOCATION], "/hosts");
        assert_eq!(
            h.state.engine.lock().conf.data_repr.data_unit,
            DataUnit::Packets
        );
    }

    #[tokio::test]
    async fn the_badge_shows_at_most_two_digits() {
        let h = harness().await;
        h.state.engine.lock().unread_notifications = 7;
        let reply = send(&h.app, get("/settings", Some(&h.cookie))).await;
        assert!(reply.body.contains("data-live-unread title=\"7\">7</span>"));
        h.state.engine.lock().unread_notifications = 150;
        let reply = send(&h.app, get("/settings", Some(&h.cookie))).await;
        assert!(
            reply
                .body
                .contains("data-live-unread title=\"150\">99+</span>")
        );
    }

    #[tokio::test]
    async fn quit_needs_the_csrf_token_and_stops_the_server() {
        let h = harness().await;
        let reply = send(&h.app, post("/quit", &h.cookie, "csrf_token=wrong")).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
        let form = format!("csrf_token={}", h.csrf);
        let reply = send(&h.app, post("/quit", &h.cookie, &form)).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.body.contains("<code>openpacketmon</code>"));
        let stopped = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            h.state.shutdown.notified(),
        )
        .await;
        assert!(stopped.is_ok());
    }
}
