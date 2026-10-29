// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/service_category.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use std::fmt::Display;

/// General purpose of an upper layer service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ServiceCategory {
    /// Web content, generic web APIs, and web transfer protocols.
    Web,
    /// Email delivery, access, notification, and mail-specific routing.
    Email,
    /// Human messaging, chat, discussion/news, SMS and paging.
    Chat,
    /// Audio/video delivery, calls, conferencing, and audiovisual sessions.
    Media,
    /// File/object transfer, synchronization, sharing, and network filesystems.
    Files,
    /// Storage infrastructure, block/tape access, backup, restore, and protected replication.
    Storage,
    /// Database/query engines, data caches, database access and replication.
    Database,
    /// Interactive computer/desktop/application access and remote execution.
    Remote,
    /// IT monitoring, logs, diagnostics, configuration, deployment, availability, and resource management.
    Management,
    /// Name resolution and locating network devices/services.
    Discovery,
    /// Address assignment, network boot, time synchronization, routing, switching, mobility, and telecom/network control.
    Network,
    /// General connection tunneling, network relays, VPN negotiation, and proxies.
    Vpn,
    /// Authentication, authorization, directories, identity lookup, and credential/key/certificate services.
    Identity,
    /// Printing, document scanning, fax, and their dedicated peripheral protocols.
    Printing,
    /// Game sessions, multiplayer transports, game lobbies, and game-specific communication.
    Gaming,
    /// Physical process/equipment control, building/home automation, industrial telemetry and metering.
    Industrial,
    /// Source control, compilation/build/test tools, debugging, and development support.
    Development,
    /// General application messaging, RPC, distributed objects, transaction coordination and shared application infrastructure.
    Middleware,
    /// Threat/vulnerability detection, prevention, integrity checking, security policy enforcement and incident exchange.
    Security,
    /// Software license distribution, entitlement verification, activation, and floating-license services.
    Licensing,
    /// Fallback for specialized applications outside this taxonomy, ambiguous identities, or insufficiently documented purposes.
    #[default]
    Other,
}

impl Display for ServiceCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let category_str = match self {
            ServiceCategory::Web => "Web",
            ServiceCategory::Email => "Email",
            ServiceCategory::Chat => "Chat & Messaging",
            ServiceCategory::Media => "Voice & Media",
            ServiceCategory::Files => "File Transfer & Sharing",
            ServiceCategory::Storage => "Storage & Backup",
            ServiceCategory::Database => "Databases & Caches",
            ServiceCategory::Remote => "Remote Access & Execution",
            ServiceCategory::Management => "Monitoring & Administration",
            ServiceCategory::Discovery => "Naming & Service Discovery",
            ServiceCategory::Network => "Network Infrastructure",
            ServiceCategory::Vpn => "VPNs, Tunnels & Proxies",
            ServiceCategory::Identity => "Identity & Directory",
            ServiceCategory::Printing => "Printing & Scanning",
            ServiceCategory::Gaming => "Gaming",
            ServiceCategory::Industrial => "Industrial & Automation",
            ServiceCategory::Development => "Software Development",
            ServiceCategory::Middleware => "Middleware",
            ServiceCategory::Security => "Security",
            ServiceCategory::Licensing => "Software Licensing",
            ServiceCategory::Other => "",
        };
        write!(f, "{category_str}")
    }
}
