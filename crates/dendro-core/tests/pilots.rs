//! Pilot species of the registry: Norway spruce (evergreen conifer) and apple
//! (fruit tree). Architecture and growth on the median of 5 seeds fixed in
//! advance (1-5). Every band comes from a reference entry of
//! `data/validation/<id>.ron` (id in brackets) or is a stated bench
//! convention; none is read off the simulation.

use dendro_core::{Genome, Phase, ReproStage, Simulation};
use std::sync::OnceLock;

const SEEDS: [u64; 5] = [1, 2, 3, 4, 5];

fn median(mut v: Vec<f32>) -> f32 {
    v.sort_by(f32::total_cmp);
    v[v.len() / 2]
}

/// Per-year record of one run.
#[derive(Clone, Default)]
struct Year {
    height: f32,
    dbh: f32,
    crown_w_h: f32,
    scaffold_limbs: u32,
    root_depth: f32,
    /// Leaf density on Jan 15 and on Jul 15.
    winter_leaf: f32,
    summer_leaf: f32,
    budburst: Option<u32>,
    /// 1-based day of year of the first day with open flowers.
    bloom: Option<u32>,
    peak_flowers: f32,
    /// 1-based day of year when the fruit is ripe (ripeness >= 0.9).
    ripe: Option<u32>,
}

fn run(id: &str, seed: u64, years: u32) -> Vec<Year> {
    let mut sim = Simulation::new(Genome::by_id(id).unwrap_or_else(|| panic!("{id}")), seed);
    let mut out = Vec::new();
    for _ in 0..years {
        let mut y = Year::default();
        let mut prev = sim.phase();
        for _ in 0..365 {
            let doy = sim.day_of_year();
            sim.advance_days(1);
            if doy == 14 {
                y.winter_leaf = sim.leaf_density();
            }
            if doy == 195 {
                y.summer_leaf = sim.leaf_density();
            }
            if sim.phase() != prev && sim.phase() == Phase::BudBurst && y.budburst.is_none() {
                y.budburst = Some(doy + 1);
            }
            prev = sim.phase();
            let f = sim.flower_density();
            if f > 0.0 && y.bloom.is_none() {
                y.bloom = Some(doy + 1);
            }
            y.peak_flowers = y.peak_flowers.max(f);
            if sim.repro_stage() == ReproStage::Fruiting && sim.fruit_ripeness() >= 0.9 && y.ripe.is_none() {
                y.ripe = Some(doy + 1);
            }
        }
        let m = sim.shape_metrics();
        y.height = sim.height_m();
        y.dbh = sim.dbh_cm();
        y.crown_w_h = m.crown_width_to_height;
        y.scaffold_limbs = m.scaffold_limbs;
        y.root_depth = sim.root_depth_m();
        out.push(y);
    }
    out
}

fn runs(id: &'static str, years: u32) -> Vec<Vec<Year>> {
    let hs: Vec<_> = SEEDS.iter().map(|&s| std::thread::spawn(move || run(id, s, years))).collect();
    hs.into_iter().map(|h| h.join().unwrap()).collect()
}

fn spruce() -> &'static Vec<Vec<Year>> {
    static R: OnceLock<Vec<Vec<Year>>> = OnceLock::new();
    R.get_or_init(|| runs("picea_abies", 100))
}

fn apple() -> &'static Vec<Vec<Year>> {
    static R: OnceLock<Vec<Vec<Year>>> = OnceLock::new();
    R.get_or_init(|| runs("malus_domestica", 80))
}

fn at(r: &[Vec<Year>], age: usize, f: impl Fn(&Year) -> f32) -> f32 {
    median(r.iter().map(|y| f(&y[age - 1])).collect())
}

/// Mean over the run of a yearly date, per seed, then the median.
fn mean_date(r: &[Vec<Year>], from_age: usize, f: impl Fn(&Year) -> Option<u32>) -> f32 {
    median(
        r.iter()
            .map(|ys| {
                let v: Vec<f32> = ys[from_age - 1..].iter().filter_map(&f).map(|d| d as f32).collect();
                v.iter().sum::<f32>() / v.len().max(1) as f32
            })
            .collect(),
    )
}

fn first_bloom_age(ys: &[Year]) -> f32 {
    ys.iter().position(|y| y.bloom.is_some()).map_or(f32::NAN, |i| (i + 1) as f32)
}

#[test]
fn registry_lists_the_pilots() {
    let s = Genome::by_id("picea_abies").unwrap();
    let a = Genome::by_id("malus_domestica").unwrap();
    assert_eq!(s.category, dendro_core::SpeciesCategory::Conifer);
    assert_eq!(a.category, dendro_core::SpeciesCategory::FruitTree);
    assert!(s.leaf.evergreen() && s.leaf.leaf_type == dendro_core::LeafType::Needle);
    assert!(!a.leaf.evergreen() && a.leaf.leaf_type == dendro_core::LeafType::Broad);
    // Menu order: broadleaves, fruit trees, conifers.
    assert!(Genome::index_of("quercus_robur").unwrap() < Genome::index_of("malus_domestica").unwrap());
    assert!(Genome::index_of("malus_domestica").unwrap() < Genome::index_of("picea_abies").unwrap());
}

#[test]
fn spruce_keeps_several_needle_cohorts() {
    // Needles kept 5-7 years [wfr_needle_longevity], 4-7 [wde_needle_longevity]:
    // before the spring flush a tree carries (L-1)/L of its summer foliage,
    // 0.75-0.86 for L = 4-7 (equilibrium of equal yearly cohorts).
    let r = spruce();
    for age in [20usize, 60] {
        let ratio = median(r.iter().map(|ys| ys[age - 1].winter_leaf / ys[age - 1].summer_leaf.max(1e-6)).collect());
        assert!((0.75..=0.86).contains(&ratio), "spruce {age} y: winter / summer foliage {ratio:.2}");
    }
}

#[test]
fn spruce_height_and_leader_growth() {
    let r = spruce();
    // Mean stand height ~34 m at 100 y (Assmann-Franz table [utschig_h100_ass]);
    // usual adult height 35-40 m [wfr_height]. Band: 0.7 x 34 (bench
    // convention for an open-grown tree, no open-grown table found) to 40 m.
    let h100 = at(r, 100, |y| y.height);
    assert!((23.8..=40.0).contains(&h100), "spruce height at 100 y {h100:.1} m");
    // Leader growth up to 1 m/yr [wen_leader_growth] (record; +10 %: bench
    // convention for records).
    let max_inc = median(r.iter().map(|ys| ys.windows(2).map(|w| w[1].height - w[0].height).fold(0.0, f32::max)).collect());
    assert!(max_inc <= 1.1, "spruce max yearly height growth {max_inc:.2} m");
    eprintln!("spruce medians: H100 {h100:.1} m, DBH100 {:.1} cm, max leader {max_inc:.2} m/yr", at(r, 100, |y| y.dbh));
}

#[test]
fn spruce_is_a_monopodial_cone() {
    // Monopodial, excurrent stem with whorls of short branches and a conical
    // crown (Atlas [atlas_*], trait sheet): no scaffold limb >= 20 % of DBH
    // and crown width / height <= 0.6 (bench convention for "conical").
    let r = spruce();
    for age in [40usize, 100] {
        let wh = at(r, age, |y| y.crown_w_h);
        let limbs = at(r, age, |y| y.scaffold_limbs as f32);
        assert!(wh <= 0.6, "spruce {age} y: crown W/H {wh:.2}");
        assert!(limbs <= 1.0, "spruce {age} y: {limbs} scaffold limbs");
    }
}

#[test]
fn spruce_cones_and_late_flush() {
    let r = spruce();
    // First flowering 20-30 y open-grown [atlas_first_flowering], 30-40
    // [wde_first_flowering] [wsv_first_cones]: 20-40.
    let first = median(r.iter().map(|ys| first_bloom_age(ys)).collect());
    assert!((20.0..=40.0).contains(&first), "spruce first cones at {first} y");
    // Cones ripe September-October [wfr_cone_maturity] (doy 244-304).
    let ripe = mean_date(r, 30, |y| y.ripe);
    assert!((244.0..=304.0).contains(&ripe), "spruce cones ripe on day {ripe:.0}");
    // Flowering May-June [wde_flowering] (doy 121-181).
    let bloom = mean_date(r, 30, |y| y.bloom);
    assert!((121.0..=181.0).contains(&bloom), "spruce flowering day {bloom:.0}");
    // Masting: flowering every 3-4 years (trait sheet, Wikipedia de): fewer
    // than half of the years after first flowering are full crops.
    let full: Vec<f32> = r.iter().map(|ys| ys[40..].iter().filter(|y| y.peak_flowers > 0.2).count() as f32 / (ys.len() - 40) as f32).collect();
    let m = median(full);
    assert!((0.15..=0.5).contains(&m), "spruce share of full crop years {m:.2}");
}

#[test]
fn spruce_shallow_roots() {
    // Excavated trees: max depth 1.62 m [kall_depth] (max record, +10 %).
    let d = median(spruce().iter().map(|ys| ys.iter().map(|y| y.root_depth).fold(0.0, f32::max)).collect());
    assert!(d <= 1.62 * 1.1, "spruce max root depth {d:.2} m");
}

#[test]
fn apple_standard_tree_size() {
    let r = apple();
    // Free-standing crown 8-15 m [de_wiki_freestanding_height] (own-rooted /
    // standard tree; grafted orchard trees are out of scope).
    let h80 = at(r, 80, |y| y.height);
    assert!((8.0..=15.0).contains(&h80), "apple height at 80 y {h80:.1} m");
    // Max height 20 m [de_wiki_max_height] (record).
    let hmax = median(r.iter().map(|ys| ys.iter().map(|y| y.height).fold(0.0, f32::max)).collect());
    assert!(hmax <= 20.0 * 1.1, "apple max height {hmax:.1} m");
    // DBH 23-45 cm of the (adult, age not given) wild apple
    // [euforgen_sylvestris_dbh], read at the bench's adult age (0.6 x the
    // typical 90-year lifespan, rounded: 50 y; bench convention).
    let d50 = at(r, 50, |y| y.dbh);
    assert!((23.0..=45.0).contains(&d50), "apple DBH at 50 y {d50:.1} cm");
    eprintln!("apple medians: H80 {h80:.1} m, DBH50 {d50:.1} cm, DBH80 {:.1} cm", at(r, 80, |y| y.dbh));
}

#[test]
fn apple_spreading_crown() {
    // Spreading crown, often bushy in the wild [euforgen_sylvestris_height]:
    // crown at least as wide as 0.9 x its height at 80 y (bench convention).
    let wh = at(apple(), 80, |y| y.crown_w_h);
    assert!(wh >= 0.9, "apple crown W/H at 80 y {wh:.2}");
}

#[test]
fn apple_flowering_and_alternate_bearing() {
    let r = apple();
    // First fruit at 8-10 y on seedling rootstock [de_wiki_seedling_rootstock_first_fruit].
    let first = median(r.iter().map(|ys| first_bloom_age(ys)).collect());
    assert!((8.0..=10.0).contains(&first), "apple first bloom at {first} y");
    // Bloom in May [de_wiki_bloom_may] (doy 121-151).
    let bloom = mean_date(r, 12, |y| y.bloom);
    assert!((121.0..=151.0).contains(&bloom), "apple bloom day {bloom:.0}");
    // Deciduous: no foliage in January.
    assert!(r.iter().all(|ys| ys[40].winter_leaf == 0.0));
    // Alternate bearing, strong / weak years 2:1 [cider_uk_biennial_ratio]:
    // the mean peak bloom of strong years is about twice that of weak ones
    // (1.5-3, bench band around the single published ratio).
    let ratio = median(
        r.iter()
            .map(|ys| {
                let v: Vec<f32> = ys[15..].iter().map(|y| y.peak_flowers).filter(|&f| f > 0.0).collect();
                let mean = v.iter().sum::<f32>() / v.len().max(1) as f32;
                let (hi, lo): (Vec<f32>, Vec<f32>) = v.iter().partition(|&&f| f >= mean);
                (hi.iter().sum::<f32>() / hi.len().max(1) as f32) / (lo.iter().sum::<f32>() / lo.len().max(1) as f32).max(1e-6)
            })
            .collect(),
    );
    assert!((1.5..=3.0).contains(&ratio), "apple strong / weak bloom ratio {ratio:.2}");
}

#[test]
fn pruning_an_evergreen_removes_its_needles() {
    // Raising, thinning and topping a 30-year spruce: no panic, the cuts take
    // needle cohorts with them (foliage proxy drops), the tree lives on and
    // keeps green foliage. Smoke test of the cohort bookkeeping in pruning.rs.
    let mut s = Simulation::new(Genome::by_id("picea_abies").unwrap(), 2);
    s.advance_years(30);
    let before = s.prune_metrics().leaf_area_m2;
    let raised = s.raise_crown(3.0);
    let thinned = s.thin_crown(0.3);
    let topped = s.top_at(0.8 * s.height_m());
    let after = s.prune_metrics().leaf_area_m2;
    assert!(raised + thinned > 0 && topped.is_some(), "cuts: raise {raised}, thin {thinned}, top {topped:?}");
    assert!(after < 0.9 * before, "foliage proxy {before:.1} -> {after:.1} m2");
    s.advance_years(2);
    assert!(s.is_alive() && s.leaf_density() > 0.5, "after 2 years: alive {}, leaf density {:.2}", s.is_alive(), s.leaf_density());
}
