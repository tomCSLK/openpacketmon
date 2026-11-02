//! Npcap under Windows: openpacketmon captures and reads capture files through its `wpcap.dll`,
//! which the user installs (it is never shipped with openpacketmon, its license does not allow it).
//!
//! `build.rs` has Windows load `wpcap.dll` on first use, so the program starts without it and
//! can say what is missing; the first call into it would otherwise end the program.

use std::path::PathBuf;

use crate::view::i18n::Lang;

/// Where Npcap puts `wpcap.dll` in its compatibility mode, where Windows looks for it
fn wpcap_path() -> Option<PathBuf> {
    let windows = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("windir"))?;
    Some(PathBuf::from(windows).join("System32").join("wpcap.dll"))
}

/// Whether `wpcap.dll` can be loaded
pub fn installed() -> bool {
    wpcap_path().is_some_and(|path| path.is_file())
}

/// Explains in a dialog that Npcap is missing and offers to open its download page
pub fn explain_missing(t: Lang) {
    let answer = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title(t.get("npcap-missing-title"))
        .set_description(t.get("npcap-missing-text"))
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();
    if answer == rfd::MessageDialogResult::Yes {
        let _ = webbrowser::open("https://npcap.com/#download");
    }
}
