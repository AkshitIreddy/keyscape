//! System-audio analysis for music-reactive lighting.
//!
//! Captures what's currently PLAYING via WASAPI loopback (an input stream
//! opened on the default output device — never the microphone), runs a
//! 1024-point FFT at ~30 Hz on its own thread, and ships smoothed
//! `AudioFeatures` to the engine. Everything here is opt-in: the thread only
//! exists while settings.audio.enabled is true.

use crate::effects::AudioFeatures;
use crate::engine::Cmd;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const FFT_N: usize = 1024;
const RING_CAP: usize = 8192;

pub struct AudioCapture {
    tx: Sender<Cmd>,
    running: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl AudioCapture {
    pub fn new(tx: Sender<Cmd>) -> AudioCapture {
        AudioCapture { tx, running: Arc::new(AtomicBool::new(false)), handle: None }
    }

    pub fn set_capture(&mut self, on: bool) {
        if on && self.handle.is_none() {
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let tx = self.tx.clone();
            self.handle = Some(
                std::thread::Builder::new()
                    .name("audio".into())
                    .spawn(move || capture_thread(tx, running))
                    .expect("spawn audio"),
            );
        } else if !on {
            self.running.store(false, Ordering::Relaxed);
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
            let _ = self.tx.send(Cmd::Audio(AudioFeatures::default()));
        }
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.set_capture(false);
    }
}

fn capture_thread(tx: Sender<Cmd>, running: Arc<AtomicBool>) {
    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None => return,
    };
    let config = match device.default_output_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    let sr = config.sample_rate().0 as f32;
    let ch = config.channels().max(1) as usize;

    let ring: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::with_capacity(RING_CAP)));
    let ring_w = ring.clone();
    let push = move |mono: f32| {
        let mut q = ring_w.lock().unwrap();
        if q.len() >= RING_CAP {
            q.pop_front();
        }
        q.push_back(mono);
    };

    // WASAPI loopback: an *input* stream built on the *output* device.
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for f in data.chunks(ch) {
                    push(f.iter().sum::<f32>() / ch as f32);
                }
            },
            |_| {},
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                for f in data.chunks(ch) {
                    let s: f32 = f.iter().map(|&v| v as f32 / 32768.0).sum();
                    push(s / ch as f32);
                }
            },
            |_| {},
            None,
        ),
        _ => return,
    };
    let stream = match stream {
        Ok(s) => s,
        Err(_) => return,
    };
    if stream.play().is_err() {
        return;
    }

    let mut dsp = Dsp::new(sr);
    let mut buf = vec![0.0f32; FFT_N];
    while running.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(33));
        {
            let q = ring.lock().unwrap();
            if q.len() < FFT_N {
                continue;
            }
            let start = q.len() - FFT_N;
            for (i, v) in q.iter().skip(start).enumerate() {
                buf[i] = *v;
            }
        }
        let feat = dsp.process(&buf);
        if tx.send(Cmd::Audio(feat)).is_err() {
            break;
        }
    }
    drop(stream);
}

// ------------------------------------------------------------------- DSP

struct Dsp {
    sr: f32,
    hann: Vec<f32>,
    prev_mag: Vec<f32>,
    flux_hist: VecDeque<f32>,
    feat: AudioFeatures,
    /// slow-decaying maxima for auto-normalization: level/bass/mid/treble
    norms: [f32; 4],
    beat: f32,
}

impl Dsp {
    fn new(sr: f32) -> Dsp {
        let hann: Vec<f32> = (0..FFT_N)
            .map(|i| {
                let x = i as f32 / (FFT_N - 1) as f32;
                0.5 - 0.5 * (x * std::f32::consts::TAU).cos()
            })
            .collect();
        Dsp {
            sr,
            hann,
            prev_mag: vec![0.0; FFT_N / 2],
            flux_hist: VecDeque::with_capacity(45),
            feat: AudioFeatures::default(),
            norms: [1e-4; 4],
            beat: 0.0,
        }
    }

    fn band(&self, mags: &[f32], lo_hz: f32, hi_hz: f32) -> f32 {
        let bin_hz = self.sr / FFT_N as f32;
        let lo = ((lo_hz / bin_hz) as usize).max(1);
        let hi = ((hi_hz / bin_hz) as usize).min(mags.len() - 1);
        if hi <= lo {
            return 0.0;
        }
        let sum: f32 = mags[lo..hi].iter().map(|m| m * m).sum();
        (sum / (hi - lo) as f32).sqrt()
    }

    fn process(&mut self, samples: &[f32]) -> AudioFeatures {
        let mut re: Vec<f32> = samples.iter().zip(&self.hann).map(|(s, w)| s * w).collect();
        let mut im = vec![0.0f32; FFT_N];
        fft(&mut re, &mut im);
        let mags: Vec<f32> =
            (0..FFT_N / 2).map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt() / FFT_N as f32).collect();

        let level_raw = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        let bass_raw = self.band(&mags, 20.0, 250.0);
        let mid_raw = self.band(&mags, 250.0, 2000.0);
        let treble_raw = self.band(&mags, 2000.0, 8000.0);

        // spectral flux (positive changes only, low/mid bins) → onset
        let bin_hz = self.sr / FFT_N as f32;
        let flux_hi = ((4000.0 / bin_hz) as usize).min(mags.len());
        let mut flux = 0.0;
        for i in 1..flux_hi {
            flux += (mags[i] - self.prev_mag[i]).max(0.0);
        }
        self.prev_mag.copy_from_slice(&mags);
        let mean_flux: f32 = if self.flux_hist.is_empty() {
            0.0
        } else {
            self.flux_hist.iter().sum::<f32>() / self.flux_hist.len() as f32
        };
        let onset = self.flux_hist.len() > 10 && flux > mean_flux * 1.6 + 1e-4;
        if self.flux_hist.len() >= 43 {
            self.flux_hist.pop_front();
        }
        self.flux_hist.push_back(flux);
        self.beat = if onset { 1.0 } else { self.beat * (-0.033f32 / 0.18).exp() };

        // spectral centroid → 0..1 timbre brightness (log-mapped 100Hz..6.4kHz)
        let (mut num, mut den) = (0.0, 0.0);
        for (i, m) in mags.iter().enumerate().skip(1) {
            num += i as f32 * bin_hz * m;
            den += m;
        }
        let centroid_hz = if den > 1e-6 { num / den } else { 100.0 };
        let centroid = ((centroid_hz / 100.0).log2() / 6.0).clamp(0.0, 1.0);

        // auto-gain: normalize by slowly-decaying running maxima
        let raws = [level_raw, bass_raw, mid_raw, treble_raw];
        let mut vals = [0.0f32; 4];
        for i in 0..4 {
            self.norms[i] = (self.norms[i] * 0.9985).max(raws[i]).max(1e-4);
            vals[i] = (raws[i] / self.norms[i]).clamp(0.0, 1.0);
        }

        // attack/release smoothing keeps the lighting calm
        let smooth = |cur: &mut f32, target: f32| {
            let k = if target > *cur { 0.45 } else { 0.10 };
            *cur += (target - *cur) * k;
        };
        smooth(&mut self.feat.level, vals[0]);
        smooth(&mut self.feat.bass, vals[1]);
        smooth(&mut self.feat.mid, vals[2]);
        smooth(&mut self.feat.treble, vals[3]);
        smooth(&mut self.feat.centroid, centroid);
        self.feat.beat = self.beat;
        self.feat.active = true;
        self.feat
    }
}

/// In-place radix-2 Cooley-Tukey; len must be a power of two.
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -std::f32::consts::TAU / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let a = start + k;
                let b = start + k + len / 2;
                let (ur, ui) = (re[a], im[a]);
                let (vr, vi) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[a] = ur + vr;
                im[a] = ui + vi;
                re[b] = ur - vr;
                im[b] = ui - vi;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len <<= 1;
    }
}
