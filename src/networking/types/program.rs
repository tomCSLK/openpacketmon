// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/program.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use listeners::Process;
use serde::{Deserialize, Serialize};

/// Program / App.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Program {
    /// A known program.
    NamePath((String, String)),
    /// Not identified
    Unknown,
    /// Not applicable (ARP, ICMP, and IGMP)
    #[default]
    NotApplicable,
}

impl Program {
    pub fn is_unknown(&self) -> bool {
        matches!(self, Program::Unknown)
    }

    pub fn path(&self) -> String {
        match self {
            Program::NamePath((_, path)) => path.clone(),
            _ => String::new(),
        }
    }

    pub fn from_proc(proc: Option<&Process>) -> Self {
        proc.map_or(Program::Unknown, |proc| {
            Program::NamePath((proc.name.clone(), proc.path.clone()))
        })
    }
}

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Program::NamePath((name, _)) => write!(f, "{name}"),
            Program::Unknown => write!(f, "?"),
            Program::NotApplicable => write!(f, "-"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_display_unknown() {
        assert_eq!(Program::Unknown.to_string(), "?");
    }

    #[test]
    fn test_program_display_not_applicable() {
        assert_eq!(Program::NotApplicable.to_string(), "-");
    }

    #[test]
    fn test_program_display_known() {
        assert_eq!(
            Program::NamePath(("Telegram".to_string(), String::new())).to_string(),
            "Telegram"
        );
        assert_eq!(
            Program::NamePath(("Google Chrome Helper".to_string(), "/Gg.exe".to_string()))
                .to_string(),
            "Google Chrome Helper"
        );
    }
}
