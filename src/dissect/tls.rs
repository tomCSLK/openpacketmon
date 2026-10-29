//! TLS records (RFC 5246, 8446): handshake messages with the server name (SNI), ALPN and the
//! negotiated version and cipher suite; the encrypted rest is only named

use std::fmt::Write;
use std::ops::Range;

use super::{Dissection, Field, Followup, TransportInfo, hex_bytes, hex16, text, u8_at, u16_at};

/// Longest record: 2^14 bytes plus the expansion allowed for encryption (RFC 5246 6.2.3)
const MAX_RECORD: usize = 16_384 + 2_048;

/// The bytes at `at` look like the start of a TLS record
pub(super) fn is_record(data: &[u8], at: usize) -> bool {
    let (Some(kind), Some(major), Some(minor), Some(len)) = (
        u8_at(data, at),
        u8_at(data, at + 1),
        u8_at(data, at + 2),
        u16_at(data, at + 3),
    ) else {
        return false;
    };
    (20..=23).contains(&kind)
        && major == 3
        && minor <= 4
        && len > 0
        && usize::from(len) <= MAX_RECORD
}

fn version_name(v: u16) -> &'static str {
    match v {
        0x0300 => "SSL 3.0",
        0x0301 => "TLS 1.0",
        0x0302 => "TLS 1.1",
        0x0303 => "TLS 1.2",
        0x0304 => "TLS 1.3",
        _ if is_grease(v) => "GREASE",
        _ => "unknown",
    }
}

fn protocol_name(v: u16) -> &'static str {
    match v {
        0x0300 => "SSLv3",
        0x0301 => "TLSv1",
        0x0302 => "TLSv1.1",
        0x0304 => "TLSv1.3",
        _ => "TLSv1.2",
    }
}

/// Reserved values that clients send to keep servers tolerant (RFC 8701)
fn is_grease(v: u16) -> bool {
    v & 0x0f0f == 0x0a0a && v >> 8 == v & 0xff
}

/// Names of the cipher suites in common use (IANA registry)
fn cipher_suite_name(c: u16) -> String {
    match c {
        0x1301 => "TLS_AES_128_GCM_SHA256",
        0x1302 => "TLS_AES_256_GCM_SHA384",
        0x1303 => "TLS_CHACHA20_POLY1305_SHA256",
        0xc02b => "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
        0xc02c => "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
        0xc02f => "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256",
        0xc030 => "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384",
        0xcca8 => "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256",
        0xcca9 => "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256",
        0xc009 => "TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA",
        0xc00a => "TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA",
        0xc013 => "TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA",
        0xc014 => "TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA",
        0x009c => "TLS_RSA_WITH_AES_128_GCM_SHA256",
        0x009d => "TLS_RSA_WITH_AES_256_GCM_SHA384",
        0x002f => "TLS_RSA_WITH_AES_128_CBC_SHA",
        0x0035 => "TLS_RSA_WITH_AES_256_CBC_SHA",
        0x000a => "TLS_RSA_WITH_3DES_EDE_CBC_SHA",
        0x00ff => "TLS_EMPTY_RENEGOTIATION_INFO_SCSV",
        _ if is_grease(c) => "GREASE",
        _ => return format!("Unknown ({})", hex16(c)),
    }
    .to_owned()
}

fn group_name(g: u16) -> String {
    match g {
        0x0017 => "secp256r1",
        0x0018 => "secp384r1",
        0x0019 => "secp521r1",
        0x001d => "x25519",
        0x001e => "x448",
        0x0100 => "ffdhe2048",
        0x0101 => "ffdhe3072",
        0x11ec => "X25519MLKEM768",
        0x6399 => "X25519Kyber768Draft00",
        _ if is_grease(g) => "GREASE",
        _ => return hex16(g),
    }
    .to_owned()
}

fn extension_name(e: u16) -> String {
    match e {
        0 => "server_name",
        5 => "status_request",
        10 => "supported_groups",
        11 => "ec_point_formats",
        13 => "signature_algorithms",
        16 => "application_layer_protocol_negotiation",
        18 => "signed_certificate_timestamp",
        21 => "padding",
        23 => "extended_master_secret",
        27 => "compress_certificate",
        35 => "session_ticket",
        41 => "pre_shared_key",
        42 => "early_data",
        43 => "supported_versions",
        45 => "psk_key_exchange_modes",
        51 => "key_share",
        17_513 | 17_613 => "application_settings",
        0xfe0d => "encrypted_client_hello",
        0xff01 => "renegotiation_info",
        _ if is_grease(e) => "GREASE",
        _ => return format!("Unknown ({e})"),
    }
    .to_owned()
}

fn handshake_name(t: u8) -> Option<&'static str> {
    Some(match t {
        0 => "Hello Request",
        1 => "Client Hello",
        2 => "Server Hello",
        4 => "New Session Ticket",
        8 => "Encrypted Extensions",
        11 => "Certificate",
        12 => "Server Key Exchange",
        13 => "Certificate Request",
        14 => "Server Hello Done",
        15 => "Certificate Verify",
        16 => "Client Key Exchange",
        20 => "Finished",
        _ => return None,
    })
}

fn alert_name(a: u8) -> String {
    match a {
        0 => "Close Notify",
        10 => "Unexpected Message",
        20 => "Bad Record MAC",
        40 => "Handshake Failure",
        42 => "Bad Certificate",
        45 => "Certificate Expired",
        46 => "Certificate Unknown",
        47 => "Illegal Parameter",
        48 => "Unknown CA",
        50 => "Decode Error",
        51 => "Decrypt Error",
        70 => "Protocol Version",
        80 => "Internal Error",
        90 => "User Canceled",
        112 => "Unrecognized Name",
        116 => "Certificate Required",
        120 => "No Application Protocol",
        _ => return format!("Alert {a}"),
    }
    .to_owned()
}

/// What the records of a segment show, for the packet list
#[derive(Default)]
struct Summary {
    parts: Vec<String>,
    /// The version from `supported_versions` or the hello
    version: Option<u16>,
    /// For the display filter
    sni: Vec<String>,
    alpn: Vec<String>,
    handshakes: Vec<u8>,
    alerts: Vec<u8>,
}

/// One or more TLS records from the start of the payload
#[allow(clippy::too_many_lines)] // the four record types
pub(super) fn dissect(
    data: &[u8],
    payload: Range<usize>,
    t: &TransportInfo,
    d: &mut Dissection,
) -> bool {
    let mut pos = payload.start;
    let mut summary = Summary::default();
    let mut record_version = 0x0303;
    while pos + 5 <= payload.end && is_record(data, pos) {
        let kind = data[pos];
        let version = u16_at(data, pos + 1).unwrap_or_default();
        record_version = version;
        let len = usize::from(u16_at(data, pos + 3).unwrap_or_default());
        let body = pos + 5..pos + 5 + len;
        let available = body.start..body.end.min(payload.end);
        let kind_name = match kind {
            20 => "Change Cipher Spec",
            21 => "Alert",
            22 => "Handshake",
            _ => "Application Data",
        };
        let mut fields = vec![
            Field::new(
                "Content type",
                format!("{kind_name} ({kind})"),
                pos..pos + 1,
            ),
            Field::new(
                "Version",
                format!("{} ({})", version_name(version), hex16(version)),
                pos + 1..pos + 3,
            ),
            Field::new("Length", len.to_string(), pos + 3..pos + 5),
        ];
        match kind {
            20 => summary.parts.push("Change Cipher Spec".into()),
            21 => {
                if len == 2 && available.len() == 2 {
                    let (level, desc) = (data[body.start], data[body.start + 1]);
                    let name = alert_name(desc);
                    fields.push(Field::new(
                        "Level",
                        if level == 2 {
                            "Fatal (2)"
                        } else {
                            "Warning (1)"
                        },
                        body.start..body.start + 1,
                    ));
                    fields.push(Field::new(
                        "Description",
                        format!("{name} ({desc})"),
                        body.start + 1..body.end,
                    ));
                    summary.parts.push(format!("Alert ({name})"));
                    summary.alerts.push(desc);
                } else {
                    summary.parts.push("Encrypted Alert".into());
                }
            }
            22 => handshake(data, available.clone(), &mut fields, &mut summary),
            _ => {
                fields.push(Field::new(
                    "Encrypted application data",
                    format!("{len} bytes"),
                    available.clone(),
                ));
                summary.parts.push("Application Data".into());
            }
        }
        if body.end > payload.end {
            fields.push(Field::new(
                "[Continues]",
                format!("{} more bytes in the next segments", body.end - payload.end),
                payload.end..payload.end,
            ));
            if kind == 22 && d.followup.is_none() {
                d.followup = Some(Followup {
                    next_seq: t
                        .seq
                        .wrapping_add(u32::try_from(payload.len()).unwrap_or(0)),
                    missing: body.end - payload.end,
                });
            }
        }
        d.add(
            format!("Transport Layer Security ({kind_name})"),
            pos..body.end.min(payload.end),
            fields,
        );
        pos = body.end;
    }
    if summary.parts.is_empty() {
        return false;
    }
    if pos < payload.end {
        d.add(
            "Transport Layer Security (continuation data)".into(),
            pos..payload.end,
            vec![Field::new(
                "Length",
                (payload.end - pos).to_string(),
                pos..payload.end,
            )],
        );
    }
    d.protocol = protocol_name(summary.version.unwrap_or(record_version));
    d.has("tls");
    d.num("tls.record.version", record_version);
    if let Some(v) = summary.version {
        d.num("tls.version", v);
    }
    for name in summary.sni.drain(..) {
        d.text("tls.sni", name);
    }
    for name in summary.alpn.drain(..) {
        d.text("tls.alpn", name);
    }
    for kind in summary.handshakes.drain(..) {
        d.num("tls.handshake.type", kind);
    }
    for alert in summary.alerts.drain(..) {
        d.num("tls.alert", alert);
    }
    let mut parts: Vec<String> = Vec::new();
    for part in summary.parts {
        if parts.last() != Some(&part) {
            parts.push(part);
        }
    }
    d.info = parts.join(", ");
    true
}

/// Data of a TLS connection that does not start a record (the rest of a long record)
pub(super) fn continuation(payload: Range<usize>, d: &mut Dissection) {
    let len = payload.len();
    d.protocol = "TLS";
    d.has("tls");
    d.info = "Continuation Data".into();
    d.add(
        "Transport Layer Security (continuation data)".into(),
        payload.clone(),
        vec![Field::new("Length", len.to_string(), payload)],
    );
}

/// The handshake messages of a record (they may continue beyond the captured bytes)
fn handshake(data: &[u8], body: Range<usize>, fields: &mut Vec<Field>, summary: &mut Summary) {
    let mut pos = body.start;
    while pos + 4 <= body.end {
        let kind = data[pos];
        let len = (usize::from(data[pos + 1]) << 16)
            | (usize::from(data[pos + 2]) << 8)
            | usize::from(data[pos + 3]);
        let Some(name) = handshake_name(kind).filter(|_| len <= 1 << 20) else {
            // after Change Cipher Spec the handshake is encrypted
            fields.push(Field::new(
                "Encrypted handshake message",
                format!("{} bytes", body.end - pos),
                pos..body.end,
            ));
            summary.parts.push("Encrypted Handshake Message".into());
            return;
        };
        let msg = pos + 4..(pos + 4 + len).min(body.end);
        summary.handshakes.push(kind);
        let mut children = vec![
            Field::new("Handshake type", format!("{name} ({kind})"), pos..pos + 1),
            Field::new("Length", len.to_string(), pos + 1..pos + 4),
        ];
        let mut part = name.to_owned();
        match kind {
            1 | 2 => hello(
                data,
                msg.clone(),
                kind == 1,
                &mut children,
                &mut part,
                summary,
            ),
            _ => {}
        }
        if pos + 4 + len > body.end {
            children.push(Field::new(
                "[Incomplete]",
                format!("{} of {len} bytes captured", msg.len()),
                body.end..body.end,
            ));
        }
        fields.push(Field::new("Handshake", name, pos..msg.end).with_children(children));
        summary.parts.push(part);
        pos += 4 + len;
    }
}

/// Client Hello or Server Hello
#[allow(clippy::too_many_lines)] // the fixed fields, then the extensions
fn hello(
    data: &[u8],
    msg: Range<usize>,
    client: bool,
    fields: &mut Vec<Field>,
    part: &mut String,
    summary: &mut Summary,
) {
    let end = msg.end;
    let mut p = msg.start;
    let Some(version) = u16_at(data, p).filter(|_| p + 2 <= end) else {
        return;
    };
    fields.push(Field::new(
        "Version",
        format!("{} ({})", version_name(version), hex16(version)),
        p..p + 2,
    ));
    summary.version = Some(version);
    p += 2;
    if p + 32 > end {
        return;
    }
    fields.push(Field::new("Random", hex_bytes(&data[p..p + 32]), p..p + 32));
    p += 32;
    let Some(sid_len) = u8_at(data, p).map(usize::from).filter(|l| p + 1 + l <= end) else {
        return;
    };
    fields.push(Field::new(
        "Session ID length",
        sid_len.to_string(),
        p..p + 1,
    ));
    p += 1 + sid_len;
    if client {
        let Some(cs_len) = u16_at(data, p)
            .map(usize::from)
            .filter(|l| p + 2 + l <= end)
        else {
            return;
        };
        let suites: Vec<Field> = (0..cs_len / 2)
            .filter_map(|i| {
                let at = p + 2 + i * 2;
                u16_at(data, at).map(|c| {
                    Field::new(
                        "Cipher suite",
                        format!("{} ({})", cipher_suite_name(c), hex16(c)),
                        at..at + 2,
                    )
                })
            })
            .collect();
        fields.push(
            Field::new(
                "Cipher suites",
                format!("{} suites", suites.len()),
                p..p + 2 + cs_len,
            )
            .with_children(suites),
        );
        p += 2 + cs_len;
        let Some(comp_len) = u8_at(data, p).map(usize::from).filter(|l| p + 1 + l <= end) else {
            return;
        };
        p += 1 + comp_len;
    } else {
        let Some(c) = u16_at(data, p).filter(|_| p + 3 <= end) else {
            return;
        };
        fields.push(Field::new(
            "Cipher suite",
            format!("{} ({})", cipher_suite_name(c), hex16(c)),
            p..p + 2,
        ));
        p += 3;
    }
    let Some(ext_len) = u16_at(data, p).map(usize::from).filter(|_| p + 2 <= end) else {
        return;
    };
    let ext_end = (p + 2 + ext_len).min(end);
    p += 2;
    let mut sni = None;
    while p + 4 <= ext_end {
        let (Some(kind), Some(len)) = (u16_at(data, p), u16_at(data, p + 2).map(usize::from))
        else {
            break;
        };
        let value = p + 4..p + 4 + len;
        if value.end > ext_end {
            fields.push(Field::new(
                "Extension",
                format!("{} (incomplete)", extension_name(kind)),
                p..ext_end,
            ));
            break;
        }
        let v = &data[value.clone()];
        let shown = extension_value(kind, v, client);
        match kind {
            0 if client => sni = server_name(v),
            16 => {
                let mut i = 2;
                while let Some(&l) = v.get(i) {
                    if let Some(proto) = v.get(i + 1..i + 1 + usize::from(l)) {
                        summary.alpn.push(text(proto));
                    }
                    i += 1 + usize::from(l);
                }
            }
            43 if !client && v.len() == 2 => summary.version = u16_at(v, 0),
            43 if client
                && (1..v.len())
                    .step_by(2)
                    .any(|i| u16_at(v, i) == Some(0x0304)) =>
            {
                summary.version = Some(0x0304);
            }
            _ => {}
        }
        let mut text_value = extension_name(kind);
        if let Some(s) = shown {
            let _ = write!(text_value, ": {s}");
        }
        fields.push(Field::new("Extension", text_value, p..value.end));
        p = value.end;
    }
    if let Some(name) = sni {
        let _ = write!(part, " (SNI={name})");
        summary.sni.push(name);
    }
}

fn server_name(v: &[u8]) -> Option<String> {
    // list length, name type 0 (host name), name length, name
    let len = usize::from(u16_at(v, 3)?);
    (v.get(2) == Some(&0)).then(|| v.get(5..5 + len).map(text))?
}

/// A short text for the extensions worth reading
fn extension_value(kind: u16, v: &[u8], client: bool) -> Option<String> {
    match kind {
        0 if client => server_name(v),
        16 => {
            let mut out = Vec::new();
            let mut i = 2;
            while let Some(&l) = v.get(i) {
                out.push(text(v.get(i + 1..i + 1 + usize::from(l))?));
                i += 1 + usize::from(l);
            }
            Some(out.join(", "))
        }
        43 if client => Some(
            (1..v.len())
                .step_by(2)
                .filter_map(|i| u16_at(v, i))
                .map(version_name)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        43 => u16_at(v, 0).map(|x| version_name(x).to_owned()),
        10 => Some(
            (2..v.len())
                .step_by(2)
                .filter_map(|i| u16_at(v, i))
                .map(group_name)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        51 if client => {
            // client shares: list length, then group, key length, key
            let mut out = Vec::new();
            let mut i = 2;
            while let (Some(g), Some(l)) = (u16_at(v, i), u16_at(v, i + 2)) {
                out.push(group_name(g));
                i += 4 + usize::from(l);
            }
            Some(out.join(", "))
        }
        51 => u16_at(v, 0).map(group_name),
        0xfe0d => Some(format!("{} bytes", v.len())),
        _ => None,
    }
}
