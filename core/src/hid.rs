//! Transport for the ASUS N-KEY keyboard (0B05:19B6).
//!
//! Protocol cross-checked against OpenRGB's AsusAuraCoreLaptop controller,
//! asusctl's rog-aura, and g-helper (the implementation proven on 2023
//! SCAR hardware under Windows):
//! - 64-byte HID *feature* reports, ID 0x5D, on the usage 0xFF31/0x0079
//!   collection.
//! - Direct mode init: `5D BC 01`, once, ~50 ms settle (g-helper).
//! - Per-key frame: 11 packets `5D BC 00 01 01 01 <start> <count> 00` + RGB
//!   triplets for LED indices 0..=166, then ONE aux packet
//!   `5D BC 00 01 04 00 00 00 00` whose triplets are consumed positionally
//!   as indices 167..=177: 167 lid logo, 169-174 front light bar, 176/177
//!   the rear lid strip halves (start/count are ignored on the aux page).
//!   Packets latch immediately; partial updates are valid — we exploit that
//!   and only send dirty blocks.
//! - Hardware brightness `5D BA C5 C4 <0-3>` must be nonzero.
//! - Zone power `5D BD 01 3F 0F 77 77 FF` (g-helper byte quirks: awake-bar
//!   bit doubled, lid/rear nibbles duplicated, trailing 0xFF) — a zone with
//!   its power bits off ignores color data entirely.

use crate::frame::FRAME_BYTES;
use hidapi::{HidApi, HidDevice};

const VID: u16 = 0x0B05;
const PID: u16 = 0x19B6;
const USAGE_PAGE: u16 = 0xFF31;
const USAGE: u16 = 0x0079;
const REPORT: u8 = 0x5D;

struct Block {
    start: u8,
    count: u8,
    aux: bool,
}

fn blocks() -> Vec<Block> {
    let mut v: Vec<Block> = (0..10)
        .map(|i| Block { start: i * 16, count: 16, aux: false })
        .collect();
    v.push(Block { start: 160, count: 7, aux: false });
    // one aux page packet: triplets consumed positionally as indices 167-177
    v.push(Block { start: 167, count: 11, aux: true });
    v
}

pub struct Keyboard {
    dev: HidDevice,
    last: [u8; FRAME_BYTES],
    ever_sent: bool,
}

impl Keyboard {
    pub fn open() -> Result<Keyboard, String> {
        let api = HidApi::new().map_err(|e| e.to_string())?;
        let info = api
            .device_list()
            .find(|d| {
                d.vendor_id() == VID
                    && d.product_id() == PID
                    && d.usage_page() == USAGE_PAGE
                    && d.usage() == USAGE
            })
            .ok_or_else(|| {
                format!("ASUS N-KEY device {VID:04X}:{PID:04X} (usage 0xFF31/0x0079) not found")
            })?;
        let dev = info.open_device(&api).map_err(|e| e.to_string())?;
        let kb = Keyboard { dev, last: [0; FRAME_BYTES], ever_sent: false };
        kb.init_direct()?;
        Ok(kb)
    }

    /// Enter direct mode: `5D BC 01` + short settle (g-helper). Required
    /// after the firmware has been in a built-in effect mode.
    pub fn init_direct(&self) -> Result<(), String> {
        let mut buf = [0u8; 64];
        buf[0] = REPORT;
        buf[1] = 0xBC;
        buf[2] = 0x01;
        self.dev.send_feature_report(&buf).map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(())
    }

    /// Hardware brightness 0-3. Must be nonzero for per-key color to show.
    pub fn set_brightness(&self, level: u8) -> Result<(), String> {
        let mut buf = [0u8; 64];
        buf[0] = REPORT;
        buf[1] = 0xBA;
        buf[2] = 0xC5;
        buf[3] = 0xC4;
        buf[4] = level.min(3);
        self.dev.send_feature_report(&buf).map_err(|e| e.to_string())
    }

    /// Enable power for every lighting zone in boot/awake/sleep states
    /// (shutdown stays off). Zones that are power-gated off ignore color
    /// data entirely — a dark rear bar usually means this was never sent.
    ///
    /// Byte layout per g-helper (the encoding proven on 2023 SCARs under
    /// Windows): `5D BD 01 <keyb+logo> <bar> <lid> <rear> FF`. Quirks: the
    /// awake-bar bit is doubled (bits 0 and 2), lid/rear duplicate their
    /// low nibble into the high one, and the trailing 0xFF is required.
    pub fn set_zone_power_all(&self) -> Result<(), String> {
        let mut buf = [0u8; 64];
        buf[0] = REPORT;
        buf[1] = 0xBD;
        buf[2] = 0x01;
        buf[3] = 0x3F; // logo+keyboard: boot/awake/sleep on, shutdown off
        buf[4] = 0x0F; // front bar: awake(doubled)/boot/sleep
        buf[5] = 0x77; // lid: boot/awake/sleep, nibble duplicated
        buf[6] = 0x77; // rear glow: boot/awake/sleep, nibble duplicated
        buf[7] = 0xFF;
        self.dev.send_feature_report(&buf).map_err(|e| e.to_string())
    }

    fn send_block(&self, b: &Block, bytes: &[u8; FRAME_BYTES]) -> Result<(), String> {
        let mut buf = [0u8; 64];
        buf[0] = REPORT;
        buf[1] = 0xBC;
        buf[2] = 0x00;
        buf[3] = 0x01;
        if b.aux {
            // aux page: start/count are ignored by the firmware; triplets
            // are consumed positionally as indices 167..=177 (OpenRGB)
            buf[4] = 0x04;
        } else {
            buf[4] = 0x01;
            buf[5] = 0x01;
            buf[6] = b.start;
            buf[7] = b.count;
        }
        let s = b.start as usize * 3;
        let n = b.count as usize * 3;
        buf[9..9 + n].copy_from_slice(&bytes[s..s + n]);
        self.dev.send_feature_report(&buf).map_err(|e| e.to_string())
    }

    /// Send only the 16-LED blocks that changed since the last send.
    /// Returns the number of blocks written (0 = nothing to do).
    pub fn send_frame(&mut self, bytes: &[u8; FRAME_BYTES]) -> Result<usize, String> {
        let mut sent = 0;
        for b in blocks() {
            let s = b.start as usize * 3;
            let n = b.count as usize * 3;
            if !self.ever_sent || bytes[s..s + n] != self.last[s..s + n] {
                self.send_block(&b, bytes)?;
                sent += 1;
            }
        }
        self.last = *bytes;
        self.ever_sent = true;
        Ok(sent)
    }

    /// Unconditional full resend of the last frame — used as a keepalive when
    /// ASUS's LightingService is alive and fighting us for the device.
    pub fn resend_all(&mut self) -> Result<(), String> {
        let bytes = self.last;
        for b in blocks() {
            self.send_block(&b, &bytes)?;
        }
        Ok(())
    }

    /// Escape hatch for protocol experiments (--zone-test): send a raw
    /// 64-byte feature report.
    pub fn send_raw(&self, buf: &[u8; 64]) -> Result<(), String> {
        self.dev.send_feature_report(buf).map_err(|e| e.to_string())
    }

    /// Re-assert everything the firmware can silently reset on power/lid
    /// events: hardware brightness, zone power, and the aux LED state
    /// (logo / bars / rear strip). Cheap — a handful of feature reports.
    pub fn reassert_state(&mut self, brightness: u8) -> Result<(), String> {
        self.set_brightness(brightness)?;
        self.set_zone_power_all()?;
        let bytes = self.last;
        for b in blocks().iter().filter(|b| b.aux) {
            self.send_block(b, &bytes)?;
        }
        Ok(())
    }
}

/// Print every 0B05:19B6 interface — debugging aid for `--identify`.
pub fn identify() -> Result<(), String> {
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let mut found = false;
    for d in api.device_list() {
        if d.vendor_id() == VID && d.product_id() == PID {
            found = true;
            println!(
                "{:04X}:{:04X} usage_page={:#06X} usage={:#06X} path={}",
                d.vendor_id(),
                d.product_id(),
                d.usage_page(),
                d.usage(),
                d.path().to_string_lossy()
            );
        }
    }
    if !found {
        println!("no {VID:04X}:{PID:04X} interfaces found");
    }
    Ok(())
}
