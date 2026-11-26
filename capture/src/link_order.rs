//! Makes the linker take `WaitOnAddress`, `WakeByAddressSingle` and `WakeByAddressAll` from
//! `api-ms-win-core-synch-l1-2-0.dll`, not from `vertdll.dll` (Windows with MSVC only).
//!
//! `pcap` 2.5.0 brings `windows-sys` 0.36, whose import library knows these functions only through
//! `vertdll.dll`, the runtime of VBS enclaves (windows-rs issue #2027). Since Rust 1.100 the standard
//! library looks them up after that library, so a program with `pcap` imported them from there and
//! crashed at the first contended lock. These references come from the program's own object, before
//! any library, and `windows-sys` 0.61 links them to the right DLL. Without `unsafe`: only the
//! addresses are taken.
//!
//! Included with `#[path]` by every crate whose programs or test programs link `pcap` under Windows:
//! the helper (here), openpacketmon (`src/main.rs`) and the tests of `sniffnet-packet-parser`. Remove
//! it when `pcap` comes with a newer `windows-sys` (`PLAN.md` 5.9).

use core::ffi::c_void;

use windows_sys::Win32::System::Threading::{WaitOnAddress, WakeByAddressAll, WakeByAddressSingle};
use windows_sys::core::BOOL;

#[used]
static WAIT_ON_ADDRESS: unsafe extern "system" fn(
    *const c_void,
    *const c_void,
    usize,
    u32,
) -> BOOL = WaitOnAddress;
#[used]
static WAKE_BY_ADDRESS_SINGLE: unsafe extern "system" fn(*const c_void) = WakeByAddressSingle;
#[used]
static WAKE_BY_ADDRESS_ALL: unsafe extern "system" fn(*const c_void) = WakeByAddressAll;
