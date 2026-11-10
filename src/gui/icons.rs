//! The icons of the window, drawn after Lucide (ISC, `resources/icons/LICENSE-Lucide.txt`) as in the
//! draft (`docs/entwurf/assets/icons.js`); white, so they can be tinted.

use eframe::egui::{ImageSource, include_image};

macro_rules! icons {
    ($($name:ident = $file:literal),* $(,)?) => {
        $(pub const $name: ImageSource<'static> =
            include_image!(concat!("../../resources/icons/", $file, ".svg"));)*
    };
}

icons! {
    // the mark of the program: a magnifier over packets (logo C)
    LOGO = "logo-mark",
    OVERVIEW = "overview",
    CONNECTIONS = "connections",
    PACKETS = "packets",
    STATISTICS = "statistics",
    HOSTS = "hosts",
    SERVICES = "services",
    PROGRAMS = "programs",
    BELL = "bell",
    SETTINGS = "settings",
    BACK = "back",
    STAR = "star",
    STAR_ON = "star-on",
    SUN = "sun",
    MOON = "moon",
    HELP = "help-circle",
    MORE = "more",
    CHEV_DOWN = "chev-down",
    CHEV_LEFT = "chev-left",
    CHEV_RIGHT = "chev-right",
    COPY = "copy",
    PLAY = "play",
    PAUSE = "pause",
    STOP = "stop",
    DOWNLOAD = "download",
    ARROW_UP_CIRCLE = "arrow-up-circle",
    X = "x",
    X_CIRCLE = "x-circle",
    FILTER = "filter",
    FILE = "file",
    EXTERNAL = "external",
    ALERT = "alert",
    INFO = "info",
    CHECK_CIRCLE = "check-circle",
    ACTIVITY = "activity",
    ARROW_DOWN = "arrow-down",
    ARROW_UP = "arrow-up",
    STREAM = "stream",
    CLOCK = "clock",
    TRASH = "trash",
    DISPLAY = "display",
    DATABASE = "database",
    GLOBE = "globe",
    BOT = "bot",
    REFRESH = "refresh",
    VOLUME = "volume",
    ADAPTER = "adapter",
    RADIO = "radio",
    FOLDER = "folder",
    ARROW_RIGHT = "arrow-right",
    PHONE = "phone",
    LOCK = "lock",
    // the large icons of empty pages (`.ico.xl`)
    CAPTURE_XL = "capture-xl",
    BELL_XL = "bell-xl",
    PHONE_XL = "phone-xl",
    LOCK_XL = "lock-xl",
}
