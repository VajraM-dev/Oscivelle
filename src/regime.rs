//! Aesthetic regimes (engine 2): a segment's mood. Each sets the judge's acceptance ranges, how long trails
//! last, how hard the music drives the emitter, and a finishing look. The seed picks one per segment.
use crate::score::{Thresholds, V1};
use crate::types::Post;

pub struct Regime {
    pub name: &'static str,
    pub th: Thresholds,
    /// Multiplies the motion block's decay (the template caps the product at 0.995).
    pub decay: f32,
    /// Multiplies the emitter's music drive.
    pub drive: f32,
    pub post: Post,
}

const CLEAN: Post = Post { vhs: 0.0, scanlines: 0.0, posterize: 0.0, tint_amount: 0.0, tint: [1.0; 3], strobe: 0.0, rgb_split: 0.0, tear: 0.0, invert: 0.0 };

pub const REGIMES: [Regime; 6] = [
    Regime { name: "calm", th: Thresholds { mean: (0.03, 0.6), min_contrast: 0.03, motion: (0.0005, 0.03), max_clipped: 0.2, max_grain: 0.15 }, decay: 1.03, drive: 0.55, post: CLEAN },
    Regime { name: "dark", th: Thresholds { mean: (0.012, 0.18), min_contrast: 0.02, motion: (0.001, 0.15), max_clipped: 0.1, max_grain: 0.2 }, decay: 0.97, drive: 0.8, post: CLEAN },
    // Tint colour is set per genome from its palette (genome::random_v2).
    Regime { name: "mono", th: V1, decay: 1.0, drive: 1.0, post: Post { tint_amount: 0.85, ..CLEAN } },
    Regime { name: "sparse", th: Thresholds { mean: (0.01, 0.25), min_contrast: 0.02, motion: (0.001, 0.2), max_clipped: 0.15, max_grain: 0.2 }, decay: 0.93, drive: 1.1, post: CLEAN },
    Regime { name: "glitch", th: Thresholds { motion: (0.002, 0.35), max_grain: 0.35, ..V1 }, decay: 0.95, drive: 1.2, post: Post { rgb_split: 0.6, tear: 0.5, invert: 0.25, scanlines: 0.2, ..CLEAN } },
    Regime { name: "maximal", th: Thresholds { mean: (0.08, 0.8), min_contrast: 0.06, max_clipped: 0.35, ..V1 }, decay: 1.0, drive: 1.4, post: Post { strobe: 0.5, ..CLEAN } },
];

pub fn get(name: &str) -> Option<&'static Regime> {
    REGIMES.iter().find(|r| r.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regimes_are_unique_and_sane() {
        for (i, r) in REGIMES.iter().enumerate() {
            assert!(REGIMES[..i].iter().all(|o| o.name != r.name), "{}", r.name);
            assert!(r.th.mean.0 < r.th.mean.1 && r.th.motion.0 < r.th.motion.1, "{}", r.name);
            assert!((0.5..=1.5).contains(&r.decay) && (0.3..=2.0).contains(&r.drive), "{}", r.name);
        }
        assert!(get("mono").is_some() && get("nope").is_none());
    }
}
