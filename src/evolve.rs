//! Picks a generated scene for one segment: score a small population against the segment's
//! own music, mutate the best two, explore more random looks, then let the seed pick among the
//! candidates that pass the gate. Budget = candidates rendered.
use crate::genome::{self, Blocks, Genome};
use crate::score::Judge;
use crate::types::*;

#[allow(clippy::too_many_arguments)]
pub fn best(
    judge: &mut Judge,
    blocks: &Blocks,
    gen_dir: &std::path::Path,
    a: &Analysis,
    start: f64,
    end: f64,
    seed: u64,
    seeds: &[Genome],
    budget: usize,
) -> Option<Genome> {
    // Score window: 4 s, 30 s into the segment (or its middle if shorter).
    let t0 = (start + 30.0).min((start + end) / 2.0);
    // Each distinct genome is rendered once (mutants and random picks repeat).
    let mut seen = std::collections::HashMap::new();
    let mut score = |g: &Genome| -> Option<f32> {
        *seen.entry(g.id()).or_insert_with(|| {
            let sc = genome::write(g, blocks, gen_dir).ok()?;
            judge.judge(sc, a, t0, 4.0).ok().flatten()
        })
    };
    let first = (budget * 2 / 5).max(2);
    let mut pop: Vec<Genome> = seeds.iter().take(first).cloned().collect();
    let mut k = 0u64;
    while pop.len() < first {
        pop.push(genome::random(blocks, hash(seed, k)));
        k += 1;
    }
    let mut scored: Vec<(f32, Genome)> = pop.into_iter().filter_map(|g| Some((score(&g)?, g))).collect();
    scored.sort_by(|x, y| y.0.total_cmp(&x.0));
    let parents: Vec<Genome> = scored.iter().take(2).map(|x| x.1.clone()).collect();
    // Half of what is left mutates the top two, half explores fresh random looks.
    let rest = budget.saturating_sub(first);
    let per = rest.div_ceil(2) / parents.len().max(1);
    for (pi, p) in parents.iter().enumerate() {
        for j in 0..per {
            let child = genome::mutate(p, blocks, hash(seed, 1000 + (pi * 100 + j) as u64));
            if let Some(s) = score(&child) {
                scored.push((s, child));
            }
        }
    }
    for _ in 0..rest - per * parents.len() {
        let g = genome::random(blocks, hash(seed, k));
        k += 1;
        if let Some(s) = score(&g) {
            scored.push((s, g));
        }
    }
    pick(scored, seed)
}

/// Share of the batch's best score a candidate needs to be picked. The verdict already rejects
/// black, static, flooded and noisy renders with calibrated thresholds; the score above that is an
/// uncalibrated sum, so it only gates (relative to this batch), it doesn't rank.
const GATE: f32 = 0.7;

/// Seed-chosen candidate among those scoring at least GATE × the best (the best when it's alone).
fn pick(scored: Vec<(f32, Genome)>, seed: u64) -> Option<Genome> {
    let top = scored.iter().map(|x| x.0).fold(f32::MIN, f32::max);
    let mut ids = std::collections::HashSet::new();
    let pass: Vec<Genome> = scored.into_iter().filter(|(s, g)| *s >= GATE * top && ids.insert(g.id())).map(|x| x.1).collect();
    let n = pass.len() as u64;
    pass.into_iter().nth((hash(seed, 0x6A7E) % n.max(1)) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_pick_different_passing_genomes_and_never_a_weak_one() {
        let g = |m: &str| Genome { motion: m.into(), ..Default::default() };
        let pool = vec![(1.0, g("a")), (0.9, g("b")), (0.8, g("c")), (0.2, g("weak"))];
        let picks: std::collections::HashSet<String> = (0..32).map(|s| pick(pool.clone(), s).unwrap().motion).collect();
        assert!(picks.len() > 1, "seeds should vary the pick: {picks:?}");
        assert!(!picks.contains("weak"));
        assert_eq!(pick(pool.clone(), 5).map(|g| g.id()), pick(pool, 5).map(|g| g.id()), "deterministic");
    }
}
