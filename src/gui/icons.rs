//! The icons of the window: the same drawings as on the browser pages, white so they can be tinted.

use eframe::egui::{ImageSource, include_image};

pub const OVERVIEW: ImageSource<'static> = include_image!("../../resources/icons/overview.svg");
pub const CONNECTIONS: ImageSource<'static> =
    include_image!("../../resources/icons/connections.svg");
pub const PACKETS: ImageSource<'static> = include_image!("../../resources/icons/packets.svg");
pub const STATISTICS: ImageSource<'static> = include_image!("../../resources/icons/statistics.svg");
pub const HOSTS: ImageSource<'static> = include_image!("../../resources/icons/hosts.svg");
pub const SERVICES: ImageSource<'static> = include_image!("../../resources/icons/services.svg");
pub const PROGRAMS: ImageSource<'static> = include_image!("../../resources/icons/programs.svg");
pub const NOTIFICATIONS: ImageSource<'static> =
    include_image!("../../resources/icons/notifications.svg");
pub const CAPTURE: ImageSource<'static> = include_image!("../../resources/icons/capture.svg");
pub const SETTINGS: ImageSource<'static> = include_image!("../../resources/icons/settings.svg");
pub const THEME: ImageSource<'static> = include_image!("../../resources/icons/theme.svg");
pub const BACK: ImageSource<'static> = include_image!("../../resources/icons/back.svg");
pub const STAR: ImageSource<'static> = include_image!("../../resources/icons/star.svg");
pub const STAR_ON: ImageSource<'static> = include_image!("../../resources/icons/star-on.svg");
pub const LOGO: ImageSource<'static> = include_image!("../../static/logo-mark.png");
