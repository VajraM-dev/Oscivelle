//! Picks a generated scene for one segment: score a small population against the segment's
//! own music, mutate the best, keep the winner. Budget = candidates rendered.
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
    let per = budget.saturating_sub(first) / parents.len().max(1);
    for (pi, p) in parents.iter().enumerate() {
        for j in 0..per {
            let child = genome::mutate(p, blocks, hash(seed, 1000 + (pi * 100 + j) as u64));
            if let Some(s) = score(&child) {
                scored.push((s, child));
            }
        }
    }
    scored.into_iter().max_by(|x, y| x.0.total_cmp(&y.0)).map(|x| x.1)
}
