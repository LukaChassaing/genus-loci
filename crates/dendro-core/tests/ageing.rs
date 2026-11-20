//! Emergent ageing and death (`dendro_core::vitality`): no lifespan
//! parameter, the trunk dies when its carbon balance and vigour fail.
//!
//! Published ranges are those of `data/validation/<id>.ron` (entry ids in
//! the comments). `LongevityTraits::maintenance_per_m` was calibrated on
//! these ranges (seeds 1-5): the longevity checks are calibration guards,
//! the diameter records are independent.

use dendro_core::{DeathCause, Genome, Simulation};
use std::sync::OnceLock;

const SEEDS: [u64; 5] = [1, 2, 3, 4, 5];

/// Age of trunk death (None: alive at `years`) and lifetime maximum DBH.
fn life(id: &str, seed: u64, years: u32) -> (Option<u32>, f32) {
    let mut sim = Simulation::new(Genome::by_id(id).unwrap(), seed);
    let mut max_dbh = 0.0f32;
    for _ in 0..years {
        sim.advance_years(1);
        max_dbh = max_dbh.max(sim.dbh_cm());
        if !sim.trunk_alive() {
            break;
        }
    }
    (sim.vitality().death_age_years, max_dbh)
}

fn lives(id: &'static str, years: u32) -> Vec<(Option<u32>, f32)> {
    lives_seeds(id, years, &SEEDS)
}

fn lives_seeds(id: &'static str, years: u32, seeds: &[u64]) -> Vec<(Option<u32>, f32)> {
    let h: Vec<_> = seeds.iter().map(|&s| std::thread::spawn(move || life(id, s, years))).collect();
    h.into_iter().map(|h| h.join().unwrap()).collect()
}

/// Seeds of the Norway spruce checks: 20 seeds fixed in advance (1-20, as
/// `examples/life_dist.rs`). The spruce's death age hangs on surviving a
/// string of drought crises (reserves collapse every 10-20 years in the
/// Rennes climate), so 5 seeds do not fix a median (interquartile range
/// from 165 to more than 400 years); the move from 5 to 20 seeds was
/// approved by the user (2026-11-20); the published bands are unchanged.
const SPRUCE_SEEDS: [u64; 20] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];

fn spruce() -> &'static Vec<(Option<u32>, f32)> {
    static R: OnceLock<Vec<(Option<u32>, f32)>> = OnceLock::new();
    R.get_or_init(|| lives_seeds("picea_abies", 350, &SPRUCE_SEEDS))
}

fn median(mut v: Vec<f32>) -> f32 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

/// Median death age (all seeds must die within `years`) and median
/// lifetime maximum DBH.
fn summary(r: &[(Option<u32>, f32)], id: &str) -> (f32, f32) {
    let ages: Vec<f32> = r.iter().map(|(d, _)| d.unwrap_or_else(|| panic!("{id}: a seed is still alive")) as f32).collect();
    (median(ages), median(r.iter().map(|x| x.1).collect()))
}

fn plum() -> &'static Vec<(Option<u32>, f32)> {
    static R: OnceLock<Vec<(Option<u32>, f32)>> = OnceLock::new();
    R.get_or_init(|| lives("prunus_domestica", 150))
}

fn cherry() -> &'static Vec<(Option<u32>, f32)> {
    static R: OnceLock<Vec<(Option<u32>, f32)>> = OnceLock::new();
    R.get_or_init(|| lives("prunus_avium", 250))
}

fn pear() -> &'static Vec<(Option<u32>, f32)> {
    static R: OnceLock<Vec<(Option<u32>, f32)>> = OnceLock::new();
    R.get_or_init(|| lives("pyrus_communis", 250))
}

/// Wild plum, own-rooted: 30-50 years [fr_wiki_longevity_franc].
#[test]
fn plum_dies_within_its_published_longevity() {
    let (age, _) = summary(plum(), "plum");
    assert!((30.0..=50.0).contains(&age), "plum median death age {age} y, published 30-50");
}

/// Wild cherry: 100-150 years [atlas_longevity_a] (70-100 in EUFORGEN
/// [euf_longevity]: the two ranges overlap at 100); trunk at most 90-120 cm
/// [atlas_dbh_max, euf_dbh_max 120].
#[test]
fn cherry_dies_within_its_published_longevity_and_stays_below_the_record() {
    let (age, dbh) = summary(cherry(), "cherry");
    assert!((70.0..=150.0).contains(&age), "cherry median death age {age} y, published 70-150");
    let max = cherry().iter().map(|x| x.1).fold(0.0, f32::max);
    assert!(max <= 120.0, "cherry lifetime DBH up to {max:.1} cm (median {dbh:.1}), record 120 cm");
}

/// Wild pear: 80-150 years [euforgen_pyraster_age]; DBH record 130 cm
/// [euforgen_pyraster_dbh_max].
#[test]
fn pear_dies_within_its_published_longevity_and_stays_below_the_record() {
    let (age, _) = summary(pear(), "pear");
    assert!((80.0..=150.0).contains(&age), "pear median death age {age} y, published 80-150");
    let max = pear().iter().map(|x| x.1).fold(0.0, f32::max);
    assert!(max <= 130.0, "pear lifetime DBH up to {max:.1} cm, record 130 cm");
}

/// Short-lived species die younger than long-lived ones on the same seeds
/// (plum < cherry), and a young cherry is vigorous with full reserves.
#[test]
fn longevity_follows_the_species() {
    let (p, _) = summary(plum(), "plum");
    let (c, _) = summary(cherry(), "cherry");
    assert!(p < c, "plum {p} y vs cherry {c} y");
    let mut s = Simulation::new(Genome::prunus_avium(), 1);
    s.advance_years(30);
    let v = s.vitality();
    assert!(v.modelled && s.trunk_alive() && v.death_age_years.is_none());
    assert!(v.reserve > 0.8 && v.death_probability < 0.001, "{v:?}");
}

/// The dead trunk stands (no leaves, no rings, no flowers), loses its fine
/// wood, then collapses; a species that resprouts keeps a live stump.
#[test]
fn dead_tree_stands_then_collapses() {
    // Plum seed 1 (dies young, quick to simulate).
    let mut s = Simulation::new(Genome::by_id("prunus_domestica").unwrap(), 1);
    while s.trunk_alive() && s.age_years() < 150.0 {
        s.advance_years(1);
    }
    let v = s.vitality();
    let d = v.death_age_years.expect("plum alive at 150 y");
    assert!(matches!(v.cause, DeathCause::Decline | DeathCause::Starvation));
    assert!(v.standing_dead && !v.collapsed, "{v:?}");
    let rings = s.rings().len();
    // The last ring is that of the season the trunk died in.
    assert_eq!(rings as u32, d, "rings stop at death: {rings} rings, death at {d} y");
    let h = s.height_m();
    let snags0 = s.stats().dead_wood_nodes;
    assert!(snags0 > 0);
    assert!(s.shoot_nodes().iter().skip(1).all(|n| n.dead), "every shoot above the collar is dead wood");
    // A year later: still standing, no new ring, no bloom.
    let mut bloom = 0.0f32;
    for _ in 0..365 {
        s.advance_days(1);
        bloom = bloom.max(s.flower_density());
    }
    assert_eq!(s.rings().len(), rings);
    assert_eq!(bloom, 0.0, "a dead trunk flowered");
    assert!(s.height_m() > 0.9 * h, "dead stem fell too early");
    let left = s.vitality().collapse_in_years.expect("standing");
    s.advance_years(left + 1);
    let v = s.vitality();
    assert!(v.collapsed && !v.standing_dead, "{v:?}");
    assert!(s.stats().dead_wood_nodes < snags0, "dead wood not removed");
    if v.resprouted {
        assert!(s.is_alive(), "resprouted stump died at once");
    } else {
        assert!(!s.is_alive());
        assert!(s.height_m() < 0.5, "collapsed stem still {:.1} m", s.height_m());
    }
    // The first death is kept even if a sprout stem dies later.
    assert_eq!(s.vitality().death_age_years, Some(d));
}

/// A species that does not resprout (Norway spruce: no stump sprouts nor
/// suckers) is dead for good; one that does (hazel, a coppice shrub)
/// regrows from the stump.
#[test]
fn resprouting_follows_the_species() {
    let g = Genome::by_id("picea_abies").unwrap();
    assert_eq!((g.pruning.stump_sprouting, g.pruning.root_suckering), (0.0, 0.0));
    let h = Genome::by_id("corylus_avellana").unwrap();
    assert!(h.pruning.stump_sprouting >= 0.99);
    let mut s = Simulation::new(h, 2);
    while s.trunk_alive() && s.age_years() < 200.0 {
        s.advance_years(1);
    }
    assert!(!s.trunk_alive(), "hazel alive at 200 y");
    assert!(s.vitality().resprouted && s.is_alive(), "hazel stool did not resprout");
    s.advance_years(5);
    assert!(s.stats().leafy_shoots > 0, "no leafy sprout 5 years after the stem died");
}

/// Same seed, same death; the death does not depend on how the days are
/// stepped.
#[test]
fn death_is_deterministic() {
    let g = Genome::by_id("prunus_domestica").unwrap();
    let mut a = Simulation::new(g.clone(), 4);
    let mut b = Simulation::new(g, 4);
    a.advance_years(60);
    for _ in 0..60 {
        for _ in 0..73 {
            b.advance_days(5);
        }
    }
    assert_eq!(a.vitality(), b.vitality());
    assert_eq!(a.state_hash(), b.state_hash());
}

/// Without the trait (`maintenance_per_m` = 0, genomes written before it),
/// the carbon balance is off: no reserves effect, no death.
#[test]
fn neutral_default_never_dies() {
    let mut g = Genome::by_id("prunus_domestica").unwrap();
    g.longevity.maintenance_per_m = 0.0;
    let mut s = Simulation::new(g, 1);
    s.advance_years(80);
    let v = s.vitality();
    assert!(!v.modelled && s.trunk_alive() && v.death_age_years.is_none(), "{v:?}");
}

/// A tree killed by pruning (a coppiced spruce cannot resprout and starves,
/// `pruning.rs`) is recorded as dead like an aged one: age, cause, no more
/// rings, and its dead wood collapses.
#[test]
fn pruning_starvation_is_a_recorded_death() {
    let mut s = Simulation::new(Genome::by_id("picea_abies").unwrap(), 3);
    s.advance_years(20);
    assert!(s.coppice().is_some());
    while s.trunk_alive() && s.age_years() < 60.0 {
        s.advance_years(1);
    }
    let v = s.vitality();
    assert!(!s.trunk_alive() && !s.is_alive(), "coppiced spruce still alive at {:.0} y: {v:?}", s.age_years());
    assert_eq!(v.cause, DeathCause::Starvation);
    let d = v.death_age_years.unwrap();
    assert!(d > 20, "{v:?}");
    let rings = s.rings().len();
    s.advance_years(2);
    assert_eq!(s.rings().len(), rings, "a dead tree keeps laying rings");
    assert!(v.standing_dead || v.collapsed, "{v:?}");
}

/// Wild apple: 80-100 years [euforgen_sylvestris_longevity, arche_noah_longevity].
#[test]
fn apple_dies_within_its_published_longevity() {
    let (age, _) = summary(&lives("malus_domestica", 200), "apple");
    assert!((80.0..=100.0).contains(&age), "apple median death age {age} y, published 80-100");
}

/// Hazel: 80-90 years [atlas_lifespan] (the stem; the stool resprouts).
#[test]
fn hazel_stem_dies_within_its_published_longevity() {
    let (age, _) = summary(&lives("corylus_avellana", 200), "hazel");
    assert!((80.0..=90.0).contains(&age), "hazel median death age {age} y, published 80-90");
}

/// Norway spruce: "rarely more than 150 cm" [atlas_dbh_max]; before the
/// emergent ageing the stem reached 166 cm at 350 y. Median over the 20
/// seeds `SPRUCE_SEEDS` (several seeds still exceed 150 cm: the record is a
/// typical maximum).
#[test]
fn spruce_lifetime_dbh_stays_below_the_record() {
    let dbh = median(spruce().iter().map(|x| x.1).collect());
    assert!(dbh <= 150.0, "spruce median lifetime DBH {dbh:.1} cm, record 150 cm");
}

/// Bark-beetle hazard (`vitality::bark_beetle_hazard`, traits of
/// `data/species/picea_abies.ron`): none outside a crisis year (reserves
/// above `crisis_reserve`) nor for a non-host; in a crisis year the
/// crisis rate times the age weight, so a young tree (< 60 y, weight 0.2)
/// is less at risk than an old one (> 100 y, weight 1) with the same
/// reserves. The sources give no size or age below which a spruce is never
/// attacked (the predisposition index keeps 0.2 below 60 years), so none is
/// tested.
#[test]
fn bark_beetle_hazard_follows_reserves_and_age() {
    use dendro_core::vitality::bark_beetle_hazard;
    let b = Genome::by_id("picea_abies").unwrap().biotic;
    assert!(b.host());
    let crisis = b.crisis_reserve;
    assert_eq!(bark_beetle_hazard(&b, 120.0, crisis + 0.01), 0.0);
    let old = bark_beetle_hazard(&b, 120.0, crisis);
    let young = bark_beetle_hazard(&b, 30.0, crisis);
    assert!((old - b.crisis_kill_rate).abs() < 1e-6, "{old}");
    assert!(young > 0.0 && young < old, "young {young}, old {old}");
    assert!((young / old - 0.2).abs() < 1e-6);
    assert!(bark_beetle_hazard(&b, 70.0, 0.0) < bark_beetle_hazard(&b, 90.0, 0.0));
    // A species without the block is not a host.
    let oak = Genome::by_id("quercus_petraea").unwrap().biotic;
    assert!(!oak.host());
    assert_eq!(bark_beetle_hazard(&oak, 200.0, 0.0), 0.0);
}

/// A dry spell raises the beetle risk of an old spruce: forced drought
/// seasons (`set_drought`: no rain, +40 % evaporative demand) run its
/// reserves down to a crisis year, where the yearly beetle death
/// probability becomes > 0, while the same tree in the generator's weather
/// has none.
#[test]
fn drought_raises_bark_beetle_risk() {
    let g = Genome::by_id("picea_abies").unwrap();
    let mut wet = Simulation::new(g.clone(), 1);
    wet.advance_years(120);
    let mut dry = Simulation::new(g, 1);
    dry.advance_years(120);
    let v0 = wet.vitality();
    assert!(wet.trunk_alive() && v0.reserve > dry.genome().biotic.crisis_reserve, "{v0:?}");
    assert_eq!(v0.bark_beetle_probability, 0.0);
    let mut seasons = 0;
    while dry.vitality().bark_beetle_probability == 0.0 && seasons < 6 {
        dry.set_drought(true);
        dry.advance_years(1);
        dry.set_drought(false);
        wet.advance_years(1);
        seasons += 1;
        assert!(dry.trunk_alive(), "drought killed the spruce outright: {:?}", dry.vitality());
    }
    let (vd, vw) = (dry.vitality(), wet.vitality());
    assert!(vd.reserve < vw.reserve, "dry {vd:?} wet {vw:?}");
    assert!(vd.bark_beetle_probability > vw.bark_beetle_probability, "after {seasons} dry seasons: dry {vd:?} wet {vw:?}");
    assert!(vd.bark_beetle_probability > 0.0);
}
