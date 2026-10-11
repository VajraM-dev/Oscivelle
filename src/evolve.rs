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

/// Engine 2, same budget as `best`. The verdict only gates (by the segment's seed-chosen regime), the
/// mutation parents are picked by seed, not score, and the final pick spreads over look space: the seed
/// picks an occupied cell (score::cell), then the candidate there farthest from `avoid` (the hand-made
/// scenes and this plan's earlier picks). The segment's grammar (genome::GRAMMARS) is drawn by seed, like its
/// regime; candidates and mutations use only that grammar's blocks. Returns the genome and its descriptor.
#[allow(clippy::too_many_arguments)]
pub fn best_v2(
    judge: &mut Judge,
    blocks: &Blocks,
    gen_dir: &std::path::Path,
    a: &Analysis,
    start: f64,
    end: f64,
    seed: u64,
    budget: usize,
    avoid: &[[f32; 6]],
) -> Option<(Genome, [f32; 6])> {
    use crate::{regime::REGIMES, score};
    let t0 = (start + 30.0).min((start + end) / 2.0);
    let regime = &REGIMES[(hash(seed, 0x4E61) % REGIMES.len() as u64) as usize];
    // The look's visual grammar, drawn before any block: every candidate and mutation comes from its pool.
    let grammar = genome::GRAMMARS[(hash(seed, 0x6A4D) % genome::GRAMMARS.len() as u64) as usize];
    let blocks = &blocks.grammar_pool(grammar);
    // A look already seen yields None, so `pass` has no duplicates.
    let mut seen = std::collections::HashSet::new();
    let mut measure = |g: Genome| -> Option<(Genome, [f32; 6])> {
        if !seen.insert(g.id()) {
            return None;
        }
        let sc = genome::write(&g, blocks, gen_dir).ok()?;
        let m = judge.measure(sc, a, t0, 4.0).ok()?;
        // Each recipe widens the regime's ranges its own way (regime::gate).
        score::passes(&m, &crate::regime::gate(regime, &g.template, &g.grammar)).then(|| (g, score::descriptor(&m)))
    };
    let fresh = |k: usize| genome::random_v2(blocks, hash(seed, k as u64), regime, grammar);
    let first = (budget * 2 / 5).max(2);
    let mut pass: Vec<(Genome, [f32; 6])> = (0..first).map(fresh).filter_map(&mut measure).collect();
    let parents: Vec<Genome> = (0..2.min(pass.len())).map(|j| pass[(hash(seed, 0xA0 + j as u64) % pass.len() as u64) as usize].0.clone()).collect();
    let rest = budget.saturating_sub(first);
    let per = rest.div_ceil(2) / parents.len().max(1);
    for (pi, p) in parents.iter().enumerate() {
        pass.extend((0..per).map(|j| genome::mutate_v2(p, blocks, hash(seed, 1000 + (pi * 100 + j) as u64))).filter_map(&mut measure));
    }
    pass.extend((first..first + rest - per * parents.len()).map(fresh).filter_map(&mut measure));
    pick_v2(pass, seed, avoid)
}

/// Seed-chosen occupied cell, then the candidate in it farthest from everything in `avoid`.
fn pick_v2(pass: Vec<(Genome, [f32; 6])>, seed: u64, avoid: &[[f32; 6]]) -> Option<(Genome, [f32; 6])> {
    use crate::score::cell;
    let mut cells: Vec<usize> = pass.iter().map(|(_, d)| cell(d)).collect();
    cells.sort_unstable();
    cells.dedup();
    let c = *cells.get((hash(seed, 0xCE11) % cells.len().max(1) as u64) as usize)?;
    let dist = |d: &[f32; 6]| avoid.iter().map(|a| a.iter().zip(d).map(|(x, y)| (x - y).powi(2)).sum::<f32>()).fold(f32::MAX, f32::min);
    pass.into_iter().filter(|(_, d)| cell(d) == c).max_by(|x, y| dist(&x.1).total_cmp(&dist(&y.1)))
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

    #[test]
    fn pick_v2_spreads_over_cells_and_avoids_seen_looks() {
        let g = |m: &str| Genome { motion: m.into(), ..Default::default() };
        let dark = [0.1, 0.1, 0.5, 0.1, 0.5, 0.5];
        let bright = [0.9, 0.9, 0.5, 0.9, 0.5, 0.5];
        let bright2 = [0.95, 0.95, 0.9, 0.95, 0.9, 0.9];
        let pool = vec![(g("d"), dark), (g("b"), bright), (g("b2"), bright2)];
        let picks: std::collections::HashSet<String> = (0..32).map(|s| pick_v2(pool.clone(), s, &[]).unwrap().0.motion).collect();
        assert!(picks.contains("d") && picks.len() >= 2, "{picks:?}");
        // In the bright cell, the look farthest from an already-chosen bright look wins.
        let s = (0..64).find(|&s| pick_v2(pool.clone(), s, &[]).unwrap().0.motion != "d").unwrap();
        assert_eq!(pick_v2(pool.clone(), s, &[bright2]).unwrap().0.motion, "b");
        assert_eq!(pick_v2(pool.clone(), 3, &[dark]).map(|x| x.0.id()), pick_v2(pool, 3, &[dark]).map(|x| x.0.id()));
        assert!(pick_v2(vec![], 1, &[]).is_none());
    }
}
