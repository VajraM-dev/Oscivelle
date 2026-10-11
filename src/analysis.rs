//! Audio analysis: one streaming STFT pass -> kicks, tempo, beats, downbeats, phrases,
//! curves, drops and track changes. Every detector is a hand-tuned heuristic; the constants
//! are first guesses to tune against real mixes.
use crate::types::{Analysis, ChangeKind, Curves, Drop, Kick, Phrase, TrackChange};
use anyhow::{bail, Context, Result};
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use std::f64::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Analysis frame rate (frames per second).
const FPS: f64 = 100.0;
const N_FFT: usize = 4096;
const CURVE_RATE: f64 = 50.0;
/// Seconds between a frame's centre and the attack it detects (calibrated on synthetic kicks).
const ONSET_LAG: f64 = 0.03;
/// Beat-tracking tightness (Ellis 2007).
const ALPHA: f32 = 100.0;

// ------------------------------------------------------------------ decoding

/// Stream-decodes `path`, calling `f(sample_rate, mono_chunk)` per packet. Prints progress.
fn decode_mono(path: &Path, label: &str, mut f: impl FnMut(u32, &[f32]) -> Result<()>) -> Result<()> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .with_context(|| format!("unsupported audio file {}", path.display()))?;
    let track = format.default_track(TrackType::Audio).context("no audio track")?;
    let params = track.codec_params.as_ref().and_then(|p| p.audio()).context("not an audio track")?;
    let mut decoder = symphonia::default::get_codecs().make_audio_decoder(params, &AudioDecoderOptions::default())?;
    let (track_id, total) = (track.id, track.num_frames.unwrap_or(0));

    let (mut inter, mut mono) = (Vec::<f32>::new(), Vec::<f32>::new());
    let (mut done, mut shown) = (0u64, u64::MAX);
    while let Some(packet) = format.next_packet()? {
        if packet.track_id != track_id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(b) => b,
            Err(SymError::DecodeError(_)) => continue, // skip a corrupt packet
            Err(e) => return Err(e.into()),
        };
        let ch = buf.spec().channels().count().max(1);
        let rate = buf.spec().rate();
        inter.resize(buf.samples_interleaved(), 0.0);
        buf.copy_to_slice_interleaved(&mut inter);
        mono.clear();
        mono.extend(inter.chunks(ch).map(|c| c.iter().sum::<f32>() / ch as f32));
        f(rate, &mono)?;
        done += mono.len() as u64;
        let p = if total > 0 { done * 100 / total } else { done / (rate as u64 * 60) };
        if p != shown {
            shown = p;
            if total > 0 { eprint!("\r{label}: {p}%") } else { eprint!("\r{label}: {p} min") }
        }
    }
    eprintln!();
    Ok(())
}

// ------------------------------------------------------------------ STFT features

#[derive(Default)]
struct Frame {
    // linear band powers
    sub: f32,
    bass: f32,
    mid: f32,
    high: f32,
    total: f32,
    /// Mean per-bin log-spectral flux in 40–120 Hz (kick) and 250–5000 Hz (snare/hat/broadband).
    kflux: f32,
    hflux: f32,
    /// log2 Hz
    centroid: f32,
    chroma: [f32; 12],
}

struct Features {
    rate: f64,
    hop: usize,
    buf: Vec<f32>,
    window: Vec<f32>,
    fft: Arc<dyn RealToComplex<f32>>,
    inp: Vec<f32>,
    spec: Vec<Complex<f32>>,
    prev: Vec<f32>,
    /// Pitch class per bin (150 Hz–5 kHz: above the sub/bass, whose presence changes at every drop), -1 elsewhere.
    pitch: Vec<i8>,
    samples: u64,
    frames: Vec<Frame>,
}

impl Features {
    fn new(rate: u32) -> Self {
        let rate = rate as f64;
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(N_FFT);
        let window = (0..N_FFT).map(|i| (0.5 - 0.5 * (2.0 * PI * i as f64 / N_FFT as f64).cos()) as f32).collect();
        let df = rate / N_FFT as f64;
        let pitch = (0..=N_FFT / 2)
            .map(|k| {
                let f = k as f64 * df;
                if (150.0..5000.0).contains(&f) {
                    ((12.0 * (f / 440.0).log2()).round() as i64 + 9).rem_euclid(12) as i8
                } else {
                    -1
                }
            })
            .collect();
        Self {
            rate,
            hop: (rate / FPS).round() as usize,
            buf: vec![0.0; N_FFT / 2], // frame i is centred on sample i*hop
            window,
            inp: fft.make_input_vec(),
            spec: fft.make_output_vec(),
            fft,
            prev: vec![0.0; N_FFT / 2 + 1],
            pitch,
            samples: 0,
            frames: Vec::new(),
        }
    }

    fn fps(&self) -> f64 {
        self.rate / self.hop as f64
    }

    fn push(&mut self, x: &[f32]) {
        self.samples += x.len() as u64;
        self.buf.extend_from_slice(x);
        let mut start = 0;
        while start + N_FFT <= self.buf.len() {
            self.frame(start);
            start += self.hop;
        }
        self.buf.drain(..start);
    }

    fn finish(&mut self) {
        self.buf.resize(self.buf.len() + N_FFT / 2, 0.0);
        self.push(&[]);
    }

    fn frame(&mut self, start: usize) {
        for ((o, s), w) in self.inp.iter_mut().zip(&self.buf[start..start + N_FFT]).zip(&self.window) {
            *o = s * w;
        }
        self.fft.process(&mut self.inp, &mut self.spec).expect("fft size");
        let df = (self.rate / N_FFT as f64) as f32;
        let scale = 4.0 / N_FFT as f32; // full-scale sine -> magnitude 1
        let mut fr = Frame::default();
        let (mut kf, mut kn, mut hf, mut hn, mut cw) = (0f32, 0usize, 0f32, 0usize, 0f32);
        for (k, c) in self.spec.iter().enumerate() {
            let f = k as f32 * df;
            let mag = c.norm() * scale;
            let p = mag * mag;
            let lm = (1.0 + 100.0 * mag).ln();
            let d = (lm - self.prev[k]).max(0.0);
            self.prev[k] = lm;
            if (40.0..120.0).contains(&f) {
                kf += d;
                kn += 1;
            } else if (250.0..5000.0).contains(&f) {
                hf += d;
                hn += 1;
            }
            match f {
                f if f < 20.0 => {}
                f if f < 60.0 => fr.sub += p,
                f if f < 250.0 => fr.bass += p,
                f if f < 4000.0 => fr.mid += p,
                f if f < 16000.0 => fr.high += p,
                _ => {}
            }
            if (20.0..16000.0).contains(&f) {
                fr.total += p;
                cw += f * p;
            }
            if self.pitch[k] >= 0 {
                fr.chroma[self.pitch[k] as usize] += mag;
            }
        }
        fr.kflux = kf / kn.max(1) as f32;
        fr.hflux = hf / hn.max(1) as f32;
        fr.centroid = (cw / (fr.total + 1e-12)).max(20.0).log2();
        self.frames.push(fr);
    }
}

// ------------------------------------------------------------------ small helpers

fn db(p: f32) -> f32 {
    10.0 * (p + 1e-10).log10()
}

fn mean(v: &[f32]) -> f32 {
    v.iter().sum::<f32>() / v.len().max(1) as f32
}

fn percentile<T: Copy + PartialOrd + Default>(v: &[T], q: f64) -> T {
    if v.is_empty() {
        return T::default();
    }
    let mut v = v.to_vec();
    let k = ((v.len() - 1) as f64 * q).round() as usize;
    *v.select_nth_unstable_by(k, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).1
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Frame index range covering [t0, t1) seconds.
fn span(len: usize, fps: f64, t0: f64, t1: f64) -> Range<usize> {
    let a = ((t0 * fps).max(0.0) as usize).min(len);
    let b = ((t1 * fps).max(0.0) as usize).min(len).max(a);
    a..b
}

fn band_db(fr: &[Frame], fps: f64, t0: f64, t1: f64, get: impl Fn(&Frame) -> f32) -> f32 {
    let r = span(fr.len(), fps, t0, t1);
    let n = r.len().max(1) as f32;
    db(fr[r].iter().map(get).sum::<f32>() / n)
}

/// Greedy non-maximum suppression: best `score` first, keep each candidate that is `far` from all kept.
fn keep_spaced<T>(mut cands: Vec<T>, score: impl Fn(&T) -> f32, far: impl Fn(&T, &T) -> bool) -> Vec<T> {
    cands.sort_by(|a, b| score(b).total_cmp(&score(a)));
    let mut kept: Vec<T> = Vec::new();
    for c in cands {
        if kept.iter().all(|k| far(k, &c)) {
            kept.push(c);
        }
    }
    kept
}

// ------------------------------------------------------------------ kicks, tempo, beats

/// Onset peaks of `env`, skipping frames where `env < ratio * other` (the other band dominates).
/// Kicks: (kflux, hflux, 1.5). Snares/claps: (hflux, kflux, 1.0).
fn detect_onsets(fr: &[Frame], fps: f64, env: fn(&Frame) -> f32, other: fn(&Frame) -> f32, ratio: f32) -> Vec<Kick> {
    let e: Vec<f32> = fr.iter().map(env).collect();
    let n = e.len();
    let floor = 0.2 * mean(&e);
    let (half, gap) = ((0.5 * fps) as usize, (0.2 * fps) as usize);
    let mut peaks: Vec<(usize, f32)> = Vec::new();
    for i in 1..n {
        let v = e[i];
        if v <= floor || v <= e[i - 1] || e[i.saturating_sub(3)..(i + 4).min(n)].iter().any(|&x| x > v) {
            continue;
        }
        if env(&fr[i]) < ratio * other(&fr[i]) {
            continue; // the other band dominates: not this kind of onset
        }
        // adaptive threshold: local median + 3 * MAD over ±0.5 s
        let win = &e[i.saturating_sub(half)..(i + half).min(n)];
        let med = percentile(win, 0.5);
        let dev: Vec<f32> = win.iter().map(|x| (x - med).abs()).collect();
        if v < med + 3.0 * percentile(&dev, 0.5) {
            continue;
        }
        match peaks.last_mut() {
            Some(last) if i - last.0 < gap => {
                if v > last.1 {
                    *last = (i, v);
                }
            }
            _ => peaks.push((i, v)),
        }
    }
    // strength relative to the loudest kick within ±8 s, so quiet tracks still get full kicks
    let w = (8.0 * fps) as usize;
    peaks
        .iter()
        .map(|&(i, v)| {
            let lo = peaks.partition_point(|p| p.0 + w < i);
            let hi = peaks.partition_point(|p| p.0 <= i + w);
            let m = peaks[lo..hi].iter().map(|p| p.1).fold(0.0, f32::max);
            Kick { time: r3(i as f64 / fps + ONSET_LAG), strength: (v / m).min(1.0) }
        })
        .collect()
}

/// Onset envelope divided by its local RMS (±5 s), so quiet tracks track as well as loud ones.
fn normalize_onsets(env: &[f32], fps: f64) -> Vec<f32> {
    let (n, w) = (env.len(), (5.0 * fps) as usize);
    let mut pre = vec![0f64; n + 1];
    for i in 0..n {
        pre[i + 1] = pre[i] + (env[i] as f64).powi(2);
    }
    let global = (pre[n] / n.max(1) as f64).sqrt();
    (0..n)
        .map(|i| {
            let (a, b) = (i.saturating_sub(w), (i + w).min(n));
            let rms = ((pre[b] - pre[a]) / (b - a) as f64).sqrt();
            (env[i] as f64 / (rms + 0.1 * global + 1e-9)) as f32
        })
        .collect()
}

/// Beat period in frames (fractional) of one window, by autocorrelation, 70–200 BPM.
/// Octave errors: a candidate also scores the periodicity at twice its period, and a broad
/// prior centred on 140 BPM breaks the tie between e.g. 87.5 and 175 (half-time DnB kicks with
/// snares on 2/4). House at ~125 is safe: its double, 250, is out of range.
fn window_period(x: &[f32], fps: f64) -> Option<f64> {
    let (lmin, lmax) = ((60.0 * fps / 200.0).floor() as usize, (60.0 * fps / 70.0).ceil() as usize);
    if x.len() < 5 * lmax {
        return None;
    }
    let m = mean(x);
    let ac = |l: usize| x.iter().zip(&x[l..]).map(|(a, b)| ((a - m) * (b - m)) as f64).sum::<f64>() / (x.len() - l) as f64;
    let ac0 = ac(0);
    let prior = |l: usize| (-0.5 * ((60.0 * fps / l as f64 / 140.0).log2() / 0.4).powi(2)).exp();
    let score = |l: usize| (ac(l) + ac(2 * l) + ac(4 * l)) * prior(l);
    let best = (lmin..=lmax).max_by(|&a, &b| score(a).total_cmp(&score(b)))?;
    let (a, b, c) = (ac(best - 1), ac(best), ac(best + 1));
    if ac0 <= 0.0 || b < 0.1 * ac0 {
        return None; // no clear pulse (breakdown, silence)
    }
    let den = a - 2.0 * b + c;
    let delta = if den < 0.0 { (0.5 * (a - c) / den).clamp(-0.5, 0.5) } else { 0.0 };
    Some(best as f64 + delta)
}

/// Local beat periods (frames), one per `step` from `win`-frame windows, and the global median bpm.
struct Tempo {
    periods: Vec<f64>,
    bpm: f64,
    win: usize,
    step: usize,
}

impl Tempo {
    /// Beat period at frame `i`: the window whose centre is nearest.
    fn period_at(&self, i: usize) -> f64 {
        let k = ((i as f64 - self.win as f64 / 2.0) / self.step as f64).round();
        self.periods[k.clamp(0.0, (self.periods.len() - 1) as f64) as usize]
    }
}

/// 10 s windows every 5 s.
fn local_tempo(o: &[f32], fps: f64) -> Tempo {
    let (win, step) = ((10.0 * fps) as usize, (5.0 * fps) as usize);
    let raw: Vec<Option<f64>> =
        (0..o.len().max(1)).step_by(step).map(|s| window_period(&o[s.min(o.len())..(s + win).min(o.len())], fps)).collect();
    let valid: Vec<f64> = raw.iter().flatten().copied().collect();
    let mut last = if valid.is_empty() { 60.0 * fps / 128.0 } else { valid[0] };
    let filled: Vec<f64> = raw.iter().map(|p| {
        last = p.unwrap_or(last); // breakdowns keep the previous tempo
        last
    }).collect();
    // median filter (±2 windows) removes octave slips
    let periods: Vec<f64> =
        (0..filled.len()).map(|i| percentile(&filled[i.saturating_sub(2)..(i + 3).min(filled.len())], 0.5)).collect();
    let bpm = 60.0 * fps / percentile(&periods, 0.5);
    Tempo { periods, bpm, win, step }
}

/// Ellis (2007) dynamic-programming beat tracker. Returns beat frame indices, ascending.
/// The transition cost keeps the grid going through breakdowns with no onsets.
fn track_beats(o: &[f32], period_at: impl Fn(usize) -> f64) -> Vec<usize> {
    let n = o.len();
    if n == 0 {
        return Vec::new();
    }
    let mut score = vec![0f32; n];
    let mut back = vec![usize::MAX; n];
    for i in 0..n {
        let p = period_at(i);
        let lo = (i as f64 - 2.0 * p).ceil().max(0.0) as usize;
        let hi = i as f64 - 0.5 * p;
        let mut best = (0f32, usize::MAX);
        if hi >= lo as f64 {
            for j in lo..=hi as usize {
                let c = score[j] - ALPHA * ((i - j) as f64 / p).ln().powi(2) as f32;
                if best.1 == usize::MAX || c > best.0 {
                    best = (c, j);
                }
            }
        }
        score[i] = o[i] + best.0;
        back[i] = best.1;
    }
    let tail = n.saturating_sub(period_at(n - 1) as usize).min(n - 1);
    let mut i = (tail..n).max_by(|&a, &b| score[a].total_cmp(&score[b])).unwrap_or(n - 1);
    let mut beats = vec![i];
    while back[i] != usize::MAX {
        i = back[i];
        beats.push(i);
    }
    beats.reverse();
    beats
}

// ------------------------------------------------------------------ downbeats and bars

/// Per-beat downbeat evidence: kick strength + rise in low-band and overall energy at the beat.
fn beat_salience(beats: &[f64], kicks: &[Kick], fr: &[Frame], fps: f64) -> Vec<f32> {
    beats
        .iter()

        .map(|&t| {
            let lo = kicks.partition_point(|k| k.time < t - 0.06);
            let kick = kicks[lo..].iter().take_while(|k| k.time <= t + 0.06).map(|k| k.strength).fold(0.0, f32::max);
            // 100 ms after vs before the beat, skipping ±50 ms: a frame sees ~46 ms either side
            let low = |a: f64| band_db(fr, fps, t + a, t + a + 0.1, |f| f.sub + f.bass);
            let all = |a: f64| band_db(fr, fps, t + a, t + a + 0.1, |f| f.total);
            // clamped: rises out of near-silence are huge in dB
            kick + ((low(0.05) - low(-0.15)) / 6.0).clamp(0.0, 1.0) + ((all(0.05) - all(-0.15)) / 6.0).clamp(0.0, 1.0)
        })
        .collect()
}

/// Every 4th beat, with the phase of 4 chosen separately in each segment (split at `bounds`).
fn downbeats(beats: &[f64], sal: &[f32], bounds: &[f64]) -> Vec<f64> {
    let mut cuts = vec![0];
    cuts.extend(bounds.iter().map(|&t| beats.partition_point(|&b| b < t - 0.05)));
    cuts.push(beats.len());
    let mut out = Vec::new();
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1].max(w[0]));
        let sum = |p: usize| (a + p..b).step_by(4).map(|j| sal[j]).sum::<f32>();
        let p = (0..4).max_by(|&x, &y| sum(x).total_cmp(&sum(y))).unwrap_or(0);
        out.extend((a + p..b).step_by(4).map(|j| beats[j]));
    }
    out
}

struct Bar {
    start: f64,
    end: f64,
    /// sub+bass dB, kicks in bar
    low: f32,
    kicks: usize,
    /// band dBs
    sub: f32,
    bass: f32,
    mid: f32,
    high: f32,
    /// mean log2 Hz
    centroid: f32,
    /// normalised to sum 1
    chroma: [f32; 12],
    bpm: f32,
}

fn make_bars(downbeats: &[f64], duration: f64, fr: &[Frame], fps: f64, kicks: &[Kick]) -> Vec<Bar> {
    downbeats
        .iter()
        .enumerate()
        .map(|(k, &start)| {
            // last bar: same length as the previous one, cut at the end of the file
            let prev_len = if k > 0 { start - downbeats[k - 1] } else { duration - start };
            let end = downbeats.get(k + 1).copied().unwrap_or((start + prev_len).min(duration)).max(start + 0.01);
            let fs = &fr[span(fr.len(), fps, start, end)];
            let m = |g: fn(&Frame) -> f32| fs.iter().map(g).sum::<f32>() / fs.len().max(1) as f32;
            let mut chroma = [0f32; 12];
            for f in fs {
                for (c, x) in chroma.iter_mut().zip(&f.chroma) {
                    *c += x;
                }
            }
            let csum = chroma.iter().sum::<f32>() + 1e-9;
            Bar {
                start,
                end,
                low: db(m(|f| f.sub + f.bass)),
                kicks: kicks.partition_point(|x| x.time < end) - kicks.partition_point(|x| x.time < start),
                sub: db(m(|f| f.sub)),
                bass: db(m(|f| f.bass)),
                mid: db(m(|f| f.mid)),
                high: db(m(|f| f.high)),
                centroid: m(|f| f.centroid),
                chroma: chroma.map(|c| c / csum),
                bpm: (240.0 / (end - start)) as f32,
            }
        })
        .collect()
}

// ------------------------------------------------------------------ structure

/// `band[i][d]` = cosine similarity of bars i and i+d (d <= w), on z-scored, weighted features.
fn sim_band(bars: &[Bar], w: usize) -> Vec<Vec<f32>> {
    let n = bars.len();
    if n == 0 {
        return Vec::new();
    }
    // Key (chroma) and tempo identify a track; band energies also change at breakdowns, so weigh less.
    let mut z: Vec<Vec<f32>> = bars
        .iter()
        .map(|b| [b.sub, b.bass, b.mid, b.high, b.centroid].into_iter().chain(b.chroma).chain([b.bpm]).collect())
        .collect();
    let dims = z[0].len();
    let weight = |d: usize| if d == dims - 1 { 1.5 } else { 0.5 }; // last dim = bpm
    for d in 0..dims {
        let col: Vec<f32> = z.iter().map(|v| v[d]).collect();
        let mu = mean(&col);
        let sd = (col.iter().map(|x| (x - mu).powi(2)).sum::<f32>() / n as f32).sqrt().max(0.05 * mu.abs() + 1e-3);
        for v in &mut z {
            v[d] = (v[d] - mu) / sd * weight(d);
        }
    }
    for v in &mut z {
        let len = v.iter().map(|x| x * x).sum::<f32>().sqrt() + 1e-9;
        v.iter_mut().for_each(|x| *x /= len);
    }
    (0..n)
        .map(|i| (0..=w.min(n - 1 - i)).map(|d| z[i].iter().zip(&z[i + d]).map(|(a, b)| a * b).sum()).collect())
        .collect()
}

/// Checkerboard-kernel novelty (FMP C4), evaluated along the diagonal only: O(n * h²).
/// nov[i] measures a boundary between bar i-1 and bar i. Needs a band of width >= 2h-1.
fn novelty(band: &[Vec<f32>], h: usize) -> Vec<f32> {
    let (n, h) = (band.len() as isize, h as isize);
    let sig = h as f32 / 2.0;
    let g = |a: isize| (-(a as f32 + 0.5).powi(2) / (2.0 * sig * sig)).exp();
    let total = (-h..h).map(g).sum::<f32>().powi(2);
    (0..n)
        .map(|i| {
            let mut s = 0.0;
            for a in -h..h {
                for b in -h..h {
                    let (x, y) = (i + a, i + b);
                    if x < 0 || y < 0 || x >= n || y >= n {
                        continue;
                    }
                    let sign = if (a < 0) == (b < 0) { 1.0 } else { -1.0 };
                    s += sign * g(a) * g(b) * band[x.min(y) as usize][(x - y).unsigned_abs()];
                }
            }
            s / total
        })
        .collect()
}

/// Key and tempo change across bar p (0 = same track, 1 = clearly different). Key compares
/// the chroma of the 32 bars before vs after; tempo compares the 8 bars each side, and only
/// when both have drums (tempo is a guess without them). Energy is ignored on purpose:
/// intros, breakdowns and drops change energy but keep key and tempo.
fn identity_change(bars: &[Bar], p: usize) -> f32 {
    let n = bars.len();
    let chroma = |r: Range<usize>| {
        let mut c = [0f32; 12];
        for b in &bars[r] {
            for (x, y) in c.iter_mut().zip(&b.chroma) {
                *x += y;
            }
        }
        let m = mean(&c);
        c.map(|x| x - m) // centred, so cosine reflects key, not "all positive"
    };
    let (ca, cb) = (chroma(p.saturating_sub(32)..p), chroma(p..(p + 32).min(n)));
    let dot = |a: &[f32; 12], b: &[f32; 12]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    let cos = dot(&ca, &cb) / (dot(&ca, &ca) * dot(&cb, &cb)).sqrt().max(1e-12);
    let key = ((0.5 - cos) / 0.4).clamp(0.0, 1.0);

    let tempo = |r: Range<usize>| {
        let drums = bars[r.clone()].iter().map(|b| b.kicks).sum::<usize>() >= 2 * r.len();
        drums.then(|| percentile(&bars[r].iter().map(|b| b.bpm).collect::<Vec<_>>(), 0.5))
    };
    let tempo = match (tempo(p.saturating_sub(8)..p), tempo(p..(p + 8).min(n))) {
        (Some(a), Some(b)) => {
            // difference in octaves, folded so a half/double-time reading counts as the same tempo
            let d = (a / b.max(1.0)).log2().abs().fract();
            ((d.min(1.0 - d) - 0.015) / 0.03).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };
    key.max(tempo)
}

fn track_changes(bars: &[Bar]) -> Vec<TrackChange> {
    const H: usize = 16; // kernel = 32 bars
    let band = sim_band(bars, 2 * H);
    let (nov, sharp) = (novelty(&band, H), novelty(&band, 2));
    let n = nov.len();
    let cands: Vec<(usize, f32, f32)> = (1..n.saturating_sub(4)) // need a few bars of the new track
        .filter(|&p| nov[p.saturating_sub(2)..(p + 3).min(n)].iter().all(|&x| x <= nov[p]))
        .map(|p| {
            let left = nov[p.saturating_sub(H)..p].iter().copied().fold(f32::MAX, f32::min);
            let right = nov[p + 1..(p + H + 1).min(n)].iter().copied().fold(f32::MAX, f32::min);
            let prom = nov[p] - left.max(if right == f32::MAX { left } else { right });
            let id = identity_change(bars, p);
            // novelty finds where something changes; key/tempo decide whether it's another track
            (p, prom, ((prom * 2.0).clamp(0.0, 1.0) * id))
        })
        .filter(|c| c.2 >= 0.15)
        .collect();
    let kept = keep_spaced(cands, |c| c.2, |a, b| (bars[a.0].start - bars[b.0].start).abs() >= 60.0);
    let mut out: Vec<TrackChange> = kept
        .into_iter()
        .map(|(p, prom, confidence)| {
            // sharp = a 2-bar kernel sees (almost) the whole change -> it happens within ~1 bar
            let q = (p.saturating_sub(1)..(p + 2).min(n)).max_by(|&a, &b| sharp[a].total_cmp(&sharp[b])).unwrap_or(p);
            if sharp[q] >= 0.5 * nov[p] {
                return TrackChange { time: bars[q].start, confidence, kind: ChangeKind::Cut, blend_len: 0.0 };
            }
            // ponytail: blend length = half-prominence width of the novelty peak; crude but monotone in blend length
            let half = nov[p] - prom / 2.0;
            let (mut l, mut r) = (p, p);
            while l > 0 && nov[l - 1] > half {
                l -= 1;
            }
            while r + 1 < n && nov[r + 1] > half {
                r += 1;
            }
            TrackChange { time: bars[p].start, confidence, kind: ChangeKind::Blend, blend_len: r3(bars[r].end - bars[l].start) }
        })
        .collect();
    out.sort_by(|a, b| a.time.total_cmp(&b.time));
    out
}

/// Drops: (bar, strength, build-start bar).
fn detect_drops(bars: &[Bar]) -> Vec<(usize, f32, usize)> {
    let dens = |b: usize| bars[b].kicks as f32 / 4.0;
    let mut cands = Vec::new();
    for b in 4..bars.len().saturating_sub(1) {
        if dens(b) < 0.5 {
            continue;
        }
        // dB jump of the next 2 bars vs the previous 4
        let jump = |get: fn(&Bar) -> f32| (get(&bars[b]) + get(&bars[b + 1])) / 2.0 - bars[b - 4..b].iter().map(get).sum::<f32>() / 4.0;
        let (low, high) = (jump(|x| x.low), jump(|x| x.high));
        if low >= 6.0 {
            // bass drop: build = run of kick-less bars right before, or (failing that) rising highs
            let mut k = b;
            while k > 0 && b - k < 32 && dens(k - 1) <= 0.25 {
                k -= 1;
            }
            if k < b || bars[b - 1].high - bars[b - 4].high >= 3.0 {
                cands.push((b, (low / 15.0).min(1.0), if k < b { k } else { b - 4 }));
            }
        } else if high >= 6.0 && bars[b - 1].high - bars[b.saturating_sub(8)].high >= 4.0 {
            // filter drop: the low end kept going, highs were swept up, then the full spectrum returns
            let mut k = b;
            while k > 0 && b - k < 32 && bars[k - 1].high < bars[b].high - 6.0 {
                k -= 1;
            }
            if k + 2 <= b {
                cands.push((b, (high / 15.0).min(1.0), k));
            }
        }
    }
    let mut kept = keep_spaced(cands, |c| c.1, |a, b| a.0.abs_diff(b.0) >= 16);
    kept.sort_by_key(|k| k.0);
    kept
}

/// Every 8 bars from each anchor; anchors = segment starts, drops and strong bar-novelty peaks.
fn phrases(bars: &[Bar], seg_starts: &[usize], drop_bars: &[usize]) -> Vec<Phrase> {
    let nov = novelty(&sim_band(bars, 8), 4);
    let n = nov.len();
    let strong = |b: usize| {
        drop_bars.contains(&b) || (nov[b] >= 0.3 && nov[b.saturating_sub(4)..(b + 5).min(n)].iter().all(|&x| x <= nov[b]))
    };
    let mut anchor = 0;
    let mut out = Vec::new();
    for b in 0..n {
        if seg_starts.contains(&b) || ((b - anchor) % 8 != 0 && strong(b)) {
            anchor = b;
        }
        let k = b - anchor;
        if k % 8 == 0 {
            let len = if k % 32 == 0 { 32 } else if k % 16 == 0 { 16 } else { 8 };
            out.push(Phrase { time: bars[b].start, bars: len });
        }
    }
    out
}

// ------------------------------------------------------------------ curves

fn make_curves(fr: &[Frame], fps: f64, duration: f64, bounds: &[f64], drops: &[Drop]) -> Curves {
    let n = (duration * CURVE_RATE).ceil() as usize;
    let mut segs: Vec<usize> = vec![0];
    segs.extend(bounds.iter().map(|&t| ((t * CURVE_RATE) as usize).min(n)));
    segs.push(n);
    let curve = |get: fn(&Frame) -> f32| -> Vec<f32> {
        let mut s = 0f32;
        let mut v: Vec<f32> = (0..n)
            .map(|c| {
                let x = band_db(fr, fps, c as f64 / CURVE_RATE, (c + 1) as f64 / CURVE_RATE, get);
                // fast attack, slow release
                s = if c == 0 { x } else { s + if x > s { 0.5 } else { 0.1 } * (x - s) };
                s
            })
            .collect();
        // level per track: 5th..95th percentile -> 0..1
        for w in segs.windows(2) {
            let seg = &mut v[w[0]..w[1].max(w[0])];
            let (lo, hi) = (percentile(seg, 0.05), percentile(seg, 0.95));
            seg.iter_mut().for_each(|x| *x = r3((((*x - lo) / (hi - lo).max(1.0)).clamp(0.0, 1.0)) as f64) as f32);
        }
        v
    };
    let tension = (0..n)
        .map(|c| {
            let t = c as f64 / CURVE_RATE;
            let x = drops
                .iter()
                .filter(|d| t >= d.build_start && t < d.time)
                .map(|d| (t - d.build_start) / (d.time - d.build_start).max(1e-3))
                .fold(0.0, f64::max);
            r3(x) as f32
        })
        .collect();
    Curves {
        rate: CURVE_RATE,
        sub: curve(|f| f.sub),
        bass: curve(|f| f.bass),
        mid: curve(|f| f.mid),
        high: curve(|f| f.high),
        energy: curve(|f| f.total),
        tension,
    }
}

// ------------------------------------------------------------------ contract

/// Decode + analyse the whole file. Prints progress to stderr.
pub fn analyze(audio: &Path) -> Result<Analysis> {
    let mut feat: Option<Features> = None;
    decode_mono(audio, "analysing", |rate, x| {
        feat.get_or_insert_with(|| Features::new(rate)).push(x);
        Ok(())
    })?;
    let mut feat = feat.context("no audio decoded")?;
    feat.finish();
    let (fps, fr) = (feat.fps(), &feat.frames);
    let duration = feat.samples as f64 / feat.rate;

    let kicks = detect_onsets(fr, fps, |f| f.kflux, |f| f.hflux, 1.5);
    let snares = detect_onsets(fr, fps, |f| f.hflux, |f| f.kflux, 1.0);
    // full onset envelope: kick + snare/hats, each levelled separately so neither drowns the other
    let level = |get: fn(&Frame) -> f32| normalize_onsets(&fr.iter().map(get).collect::<Vec<_>>(), fps);
    let (ko, ho) = (level(|f| f.kflux), level(|f| f.hflux));
    let onset: Vec<f32> = ko.iter().zip(&ho).map(|(a, b)| a + b).collect();
    // beat placement leans on kicks: offbeat hats alone (breakdowns) must not pull the grid half a beat
    let beat_onset: Vec<f32> = ko.iter().zip(&ho).map(|(a, b)| a + 0.5 * b).collect();
    let tempo = local_tempo(&onset, fps);
    let beats: Vec<f64> = track_beats(&beat_onset, |i| tempo.period_at(i))
        .into_iter()
        .map(|i| r3(i as f64 / fps + ONSET_LAG))
        .filter(|&t| (0.0..duration).contains(&t))
        .collect();

    // Track changes need bars, bars need downbeat phase, which is re-chosen per track afterwards.
    let sal = beat_salience(&beats, &kicks, fr, fps);
    let mut changes = track_changes(&make_bars(&downbeats(&beats, &sal, &[]), duration, fr, fps, &kicks));
    let bounds: Vec<f64> = changes.iter().map(|c| c.time).collect();
    let downbeats = downbeats(&beats, &sal, &bounds);
    for c in &mut changes {
        c.time = crate::types::nearest(&downbeats, c.time).unwrap_or(c.time);
    }
    let bounds: Vec<f64> = changes.iter().map(|c| c.time).collect();
    let bars = make_bars(&downbeats, duration, fr, fps, &kicks);
    let seg_starts: Vec<usize> = bounds.iter().map(|&t| bars.partition_point(|b| b.start < t - 1e-6)).collect();

    let drop_bars = detect_drops(&bars);
    let drops: Vec<Drop> =
        drop_bars.iter().map(|&(b, strength, s)| Drop { time: bars[b].start, build_start: bars[s].start, strength }).collect();
    let phrases = phrases(&bars, &seg_starts, &drop_bars.iter().map(|d| d.0).collect::<Vec<_>>());
    let curves = make_curves(fr, fps, duration, &bounds, &drops);

    Ok(Analysis {
        version: crate::types::ANALYSIS_VERSION,
        source: audio.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        duration: r3(duration),
        bpm: (tempo.bpm * 100.0).round() / 100.0,
        beats,
        downbeats,
        phrases,
        kicks,
        curves,
        drops,
        track_changes: changes,
        snares,
    })
}

fn wav_header(w: &mut impl Write, rate: u32, data_bytes: u64) -> std::io::Result<()> {
    let data = data_bytes.min((u32::MAX - 36) as u64) as u32;
    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data).to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&1u16.to_le_bytes())?; // PCM
    w.write_all(&1u16.to_le_bytes())?; // mono
    w.write_all(&rate.to_le_bytes())?;
    w.write_all(&(rate * 2).to_le_bytes())?;
    w.write_all(&2u16.to_le_bytes())?;
    w.write_all(&16u16.to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&data.to_le_bytes())
}

/// Writes a WAV: original audio (mono, -6 dB) + click on every kick and an accented click on downbeats.
pub fn write_click_track(audio: &Path, analysis: &Analysis, out_wav: &Path) -> Result<()> {
    // (start s, freq Hz, amplitude, length s)
    let mut events: Vec<(f64, f64, f32, f64)> = analysis
        .kicks
        .iter()
        .map(|k| (k.time, 1000.0, 0.2 + 0.4 * k.strength, 0.02))
        .chain(analysis.downbeats.iter().map(|&t| (t, 2500.0, 0.7, 0.04)))
        .collect();
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = BufWriter::new(File::create(out_wav).with_context(|| format!("create {}", out_wav.display()))?);
    let (mut rate, mut n, mut next) = (0u32, 0u64, 0usize);
    let mut active: Vec<(f64, f64, f32, f64)> = Vec::new();
    decode_mono(audio, "click track", |r, x| {
        if rate == 0 {
            rate = r;
            wav_header(&mut out, r, 0)?;
        }
        for &s in x {
            let t = n as f64 / rate as f64;
            while next < events.len() && events[next].0 <= t {
                active.push(events[next]);
                next += 1;
            }
            active.retain(|e| t < e.0 + e.3);
            let click: f32 = active
                .iter()
                .map(|&(t0, f, a, len)| a * ((1.0 - (t - t0) / len) * (2.0 * PI * f * (t - t0)).sin()) as f32)
                .sum();
            out.write_all(&(((0.5 * s + click).clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())?;
            n += 1;
        }
        Ok(())
    })?;
    if rate == 0 {
        bail!("no audio decoded from {}", audio.display());
    }
    out.seek(SeekFrom::Start(0))?;
    wav_header(&mut out, rate, n * 2)?;
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onsets_split_kicks_from_snares() {
        let fps = 100.0;
        let mut fr: Vec<Frame> = (0..1000).map(|_| Frame { kflux: 0.1, hflux: 0.1, ..Default::default() }).collect();
        for i in (50..1000).step_by(100) {
            fr[i].kflux = 5.0;
        }
        for i in (100..1000).step_by(100) {
            fr[i].hflux = 5.0;
        }
        let kicks = detect_onsets(&fr, fps, |f| f.kflux, |f| f.hflux, 1.5);
        let snares = detect_onsets(&fr, fps, |f| f.hflux, |f| f.kflux, 1.0);
        assert_eq!((kicks.len(), snares.len()), (10, 9));
        assert!((snares[0].time - (1.0 + ONSET_LAG)).abs() < 0.011);
    }

    /// 128 BPM impulse train with a 10 s gap (breakdown): tempo ≈ 128 and the beat grid
    /// keeps going through the gap.
    #[test]
    fn tempo_and_beats_on_synthetic_envelope() {
        let fps = 100.0;
        let period = 60.0 * fps / 128.0;
        let mut seed = 1u32;
        let mut o: Vec<f32> = (0..6000)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 8) as f32 / (1u32 << 24) as f32 * 0.1
            })
            .collect();
        let mut k = 0.0;
        while (k as usize) < o.len() {
            let i = (k as f64).round() as usize;
            if i < o.len() && !(2000..3000).contains(&i) {
                o[i] += 1.0;
            }
            k += period;
        }
        let tempo = local_tempo(&o, fps);
        assert!((tempo.bpm - 128.0).abs() < 0.5, "bpm {}", tempo.bpm);
        let beats = track_beats(&o, |i| tempo.period_at(i));
        for w in beats.windows(2) {
            let d = (w[1] - w[0]) as f64;
            assert!((d - period).abs() <= 1.5, "interval {d} at {}", w[0]);
        }
        assert!(beats.iter().any(|b| (2000..3000).contains(b)), "no beats in the breakdown");
    }
}
