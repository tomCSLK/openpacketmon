//! Application layer: which protocol a TCP or UDP payload carries, by its port and its first bytes

use std::ops::Range;

use super::{Dissection, TransportInfo, dhcp, dns, misc, text, tls};

/// Ports where TLS runs from the first byte (HTTPS, DNS over TLS, IMAPS, POP3S, SMTPS, LDAPS …)
const TLS_PORTS: [u16; 9] = [443, 465, 636, 853, 993, 995, 5223, 8443, 8883];

/// Dissects the payload if its protocol is known; `false` leaves it as plain data
pub(super) fn dissect(
    data: &[u8],
    payload: Range<usize>,
    t: &TransportInfo,
    d: &mut Dissection,
) -> bool {
    if payload.is_empty() {
        return false;
    }
    if t.tcp {
        if tls::is_record(data, payload.start) {
            return tls::dissect(data, payload, t, d);
        }
        if text::is_http(data, payload.start) {
            return text::http(data, payload, d);
        }
        if t.has_port(53) {
            return dns::dissect_tcp(data, payload, d);
        }
        if TLS_PORTS.iter().any(|p| t.has_port(*p)) {
            tls::continuation(payload, d);
            return true;
        }
        return false;
    }
    if t.has_port(53) {
        dns::dissect(data, payload, dns::Kind::Dns, d)
    } else if t.has_port(5353) {
        dns::dissect(data, payload, dns::Kind::Mdns, d)
    } else if t.has_port(5355) {
        dns::dissect(data, payload, dns::Kind::Llmnr, d)
    } else if t.has_port(67) || t.has_port(68) {
        dhcp::dissect(data, payload, d)
    } else if t.has_port(546) || t.has_port(547) {
        dhcp::dissect_v6(data, payload, d)
    } else if t.has_port(1900) {
        text::ssdp(data, payload, d)
    } else if t.has_port(123) {
        misc::ntp(data, payload, d)
    } else if t.has_port(443) {
        misc::quic(data, payload, d)
    } else {
        false
    }
}
