//! Link layer headers: Ethernet II with VLAN tags, Linux cooked capture (SLL, SLL2), BSD loopback

use super::{Dissection, Field, Next, Value, hex16, mac, u8_at, u16_at, u32_at};

/// Name of an `EtherType`
pub(super) fn ether_type_name(t: u16) -> &'static str {
    match t {
        0x0800 => "IPv4",
        0x0806 => "ARP",
        0x0842 => "Wake-on-LAN",
        0x86dd => "IPv6",
        0x8100 => "802.1Q VLAN",
        0x88a8 => "802.1ad VLAN",
        0x8863 => "PPPoE Discovery",
        0x8864 => "PPPoE Session",
        0x888e => "802.1X (EAPOL)",
        0x88cc => "LLDP",
        0x8899 => "Realtek",
        0x88e1 => "HomePlug AV",
        0x893a => "IEEE 1905.1",
        _ => "unknown",
    }
}

fn type_field(name: &'static str, t: u16, at: usize) -> Field {
    Field::new(
        name,
        format!("{} ({})", ether_type_name(t), hex16(t)),
        at..at + 2,
    )
}

/// Ethernet II; returns what follows and where
pub(super) fn ethernet(data: &[u8], d: &mut Dissection) -> Option<(Next, usize)> {
    if data.len() < 14 {
        d.add("Ethernet II".into(), 0..data.len(), Vec::new());
        d.cut(0);
        d.protocol = "Ethernet";
        return None;
    }
    let (dst, src) = (mac(&data[0..6]), mac(&data[6..12]));
    let mut ether_type = u16_at(data, 12)?;
    d.has("eth");
    for (key, at) in [("eth.dst", 0), ("eth.src", 6)] {
        if let Ok(bytes) = data[at..at + 6].try_into() {
            d.set(key, Value::Mac(bytes));
        }
    }
    d.src.clone_from(&src);
    d.dst.clone_from(&dst);
    d.protocol = "Ethernet";
    if ether_type <= 1500 {
        // IEEE 802.3 with a length instead of a type; LLC follows (spanning tree and the like)
        d.add(
            format!("IEEE 802.3 Ethernet, Src: {src}, Dst: {dst}"),
            0..14,
            vec![
                Field::new("Destination", dst, 0..6),
                Field::new("Source", src, 6..12),
                Field::new("Length", ether_type.to_string(), 12..14),
            ],
        );
        d.protocol = "LLC";
        d.info = "IEEE 802.3 frame with LLC".into();
        return None;
    }
    d.add(
        format!("Ethernet II, Src: {src}, Dst: {dst}"),
        0..14,
        vec![
            Field::new("Destination", dst, 0..6),
            Field::new("Source", src, 6..12),
            type_field("Type", ether_type, 12),
        ],
    );
    let mut offset = 14;
    // VLAN tags, also stacked (QinQ)
    while matches!(ether_type, 0x8100 | 0x88a8) {
        let Some(tci) = u16_at(data, offset) else {
            d.cut(offset);
            return None;
        };
        let Some(inner) = u16_at(data, offset + 2) else {
            d.cut(offset);
            return None;
        };
        let id = tci & 0x0fff;
        d.has("vlan");
        d.num("vlan.id", id);
        d.add(
            format!("802.1Q Virtual LAN, PRI: {}, ID: {id}", tci >> 13),
            offset..offset + 4,
            vec![
                Field::new("Priority", (tci >> 13).to_string(), offset..offset + 1),
                Field::new(
                    "Drop eligible",
                    if tci & 0x1000 == 0 { "no" } else { "yes" },
                    offset..offset + 1,
                ),
                Field::new("ID", id.to_string(), offset..offset + 2),
                type_field("Type", inner, offset + 2),
            ],
        );
        ether_type = inner;
        offset += 4;
    }
    d.num("eth.type", ether_type);
    Some((Next::EtherType(ether_type), offset))
}

fn sll_packet_type(t: u16) -> &'static str {
    match t {
        0 => "Unicast to us",
        1 => "Broadcast",
        2 => "Multicast",
        3 => "Unicast to another host",
        4 => "Sent by us",
        _ => "unknown",
    }
}

/// Linux cooked capture (the `any` adapter), version 1 or 2
pub(super) fn linux_sll(data: &[u8], d: &mut Dissection, v2: bool) -> Option<(Next, usize)> {
    let len = if v2 { 20 } else { 16 };
    let title = if v2 {
        "Linux cooked capture v2"
    } else {
        "Linux cooked capture v1"
    };
    d.protocol = "SLL";
    d.has("sll");
    if data.len() < len {
        d.add(title.into(), 0..data.len(), Vec::new());
        d.cut(0);
        return None;
    }
    let mut fields = Vec::new();
    let (protocol, packet_type, addr_len, addr_at) = if v2 {
        let protocol = u16_at(data, 0)?;
        fields.push(type_field("Protocol", protocol, 0));
        fields.push(Field::new(
            "Interface index",
            u32_at(data, 4)?.to_string(),
            4..8,
        ));
        fields.push(Field::new(
            "Link-layer address type",
            u16_at(data, 8)?.to_string(),
            8..10,
        ));
        (
            protocol,
            u16::from(u8_at(data, 10)?),
            usize::from(u8_at(data, 11)?),
            12,
        )
    } else {
        let protocol = u16_at(data, 14)?;
        fields.push(Field::new(
            "Link-layer address type",
            u16_at(data, 2)?.to_string(),
            2..4,
        ));
        (protocol, u16_at(data, 0)?, usize::from(u16_at(data, 4)?), 6)
    };
    let type_at = if v2 { 10..11 } else { 0..2 };
    fields.insert(
        0,
        Field::new("Packet type", sll_packet_type(packet_type), type_at),
    );
    let addr_len = addr_len.min(8);
    let addr = mac(&data[addr_at..addr_at + addr_len]);
    fields.push(Field::new(
        "Source",
        addr.clone(),
        addr_at..addr_at + addr_len,
    ));
    if !v2 {
        fields.push(type_field("Protocol", protocol, 14));
    }
    d.src.clone_from(&addr);
    d.add(format!("{title}, Src: {addr}"), 0..len, fields);
    Some((Next::EtherType(protocol), len))
}

/// BSD loopback (`NULL`/`LOOP`): an address family in four bytes
pub(super) fn null(data: &[u8], d: &mut Dissection) -> Option<(Next, usize)> {
    d.protocol = "Loopback";
    d.has("loopback");
    let Some(raw) = data.get(0..4) else {
        d.add("Null/Loopback".into(), 0..data.len(), Vec::new());
        d.cut(0);
        return None;
    };
    let bytes = [raw[0], raw[1], raw[2], raw[3]];
    let family = match (u32::from_le_bytes(bytes), u32::from_be_bytes(bytes)) {
        (2, _) | (_, 2) => "IPv4",
        (24 | 28 | 30, _) | (_, 24 | 28 | 30) => "IPv6",
        _ => "unknown",
    };
    d.add(
        "Null/Loopback".into(),
        0..4,
        vec![Field::new("Family", family, 0..4)],
    );
    (family != "unknown").then_some((Next::Ip, 4))
}
