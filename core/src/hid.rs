//! Transport for the ASUS N-KEY keyboard (0B05:19B6).
//!
//! Protocol (established on this exact machine; matches OpenRGB's Aura-USB
//! family with G634-specific fixes):
//! - 64-byte HID *feature* reports, ID 0x5D, sent to the interface whose
//!   collection is usage_page 0xFF31 / usage 0x0079.
//! - Per-key frame: 11 packets `5D BC 00 01 01 01 <start> <count> 00` + RGB
//!   triplets, LED indices 0..=166 in 16-LED blocks (last block count 7),
//!   plus one aux packet (byte[4] = 0x04, start 167, count 11) for the lid
//!   logo (168) and light bar (169/170/172/173). Packets latch immediately;
//!   partial updates are valid — we exploit that and only send dirty blocks.
//! - Hardware brightness `5D BA C5 C4 <0-3>` must be nonzero or per-key
//!   colors are invisible.

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
        Ok(Keyboard { dev, last: [0; FRAME_BYTES], ever_sent: false })
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

    fn send_block(&self, b: &Block, bytes: &[u8; FRAME_BYTES]) -> Result<(), String> {
        let mut buf = [0u8; 64];
        buf[0] = REPORT;
        buf[1] = 0xBC;
        buf[2] = 0x00;
        buf[3] = 0x01;
        buf[4] = if b.aux { 0x04 } else { 0x01 };
        buf[5] = 0x01;
        buf[6] = b.start;
        buf[7] = b.count;
        buf[8] = 0x00;
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
