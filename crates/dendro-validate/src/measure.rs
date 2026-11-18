//! Engine measurements: long runs of isolated trees (several seeds), a long
//! climate series, and a few protocol runs (other soils). Read-only use of
//! the public `dendro_core` API: the bench never changes a simulation's
//! state other than by advancing it day by day.

use dendro_core::climate::Climate;
use dendro_core::{ClimateParams, FailureMode, Genome, Phase, ReproStage, SegmentRole, Simulation, SoilProfile, WindLoad};
use std::collections::BTreeMap;

/// One simulated year of one tree (values at the end of the year, dates as
/// 0-based day of year, converted to 1-based DOY when reported).
#[derive(Clone, Debug, Default)]
pub struct YearRec {
    pub age: u32,
    pub height_m: f64,
    pub dbh_cm: f64,
    pub crown_radius_m: f64,
    pub root_depth_m: f64,
    pub root_spread_m: f64,
    pub ring_mm: Option<f64>,
    /// Height (m) at which this year's ring was measured (1.3 once the tree is taller).
    pub ring_at_m: f64,
    pub budburst: Option<u32>,
    pub colouring: Option<u32>,
    pub fall_start: Option<u32>,
    pub bare: Option<u32>,
    pub bloom_start: Option<u32>,
    pub bloom_full: Option<u32>,
    pub bloom_days: u32,
    pub ripe: Option<u32>,
    pub flower_frost_loss: f64,
}

/// Detailed measurements at selected ages.
#[derive(Clone, Debug, Default)]
pub struct AgeRec {
    pub height_m: f64,
    pub dbh_cm: f64,
    pub crown_width_m: f64,
    /// Woody above-ground mass, dry (t): green wood mass / green density x basic density.
    pub agb_dry_t: f64,
    pub leaf_area_m2: f64,
    pub lma_g_m2: f64,
    /// Leaf area / crown projection (isolated crown, not a stand LAI).
    pub lai_crown: f64,
    pub crown_cd_5ms: f64,
    pub first_crit_leafless: Option<f64>,
    pub first_crit_leafy: Option<f64>,
    /// First failure mode of the leafless crown ("stem", "limb", "uproot").
    pub first_mode_leafless: Option<&'static str>,
    pub root_frac_10: f64,
    pub root_frac_30: f64,
    pub root_frac_40: f64,
    pub area_ratio: f64,
    pub area_ratio_n: usize,
    /// Mean daily transpiration in July (L/day), from the flux snapshot.
    pub july_transp_l_day: f64,
    pub clear_bole_m: f64,
    #[cfg(feature = "vent")]
    pub vent: crate::vent::AgeDyn,
}

/// Measurement at the year a DBH threshold is first reached.
#[derive(Clone, Debug, Default)]
pub struct AtDbh {
    pub dbh_cm: f64,
    pub height_m: f64,
    pub agb_dry_t: f64,
    /// Trunk volume (m3) up to a 7 cm top diameter, and its length (m).
    pub stem_vol_7cm_m3: f64,
    pub stem_len_7cm_m: f64,
}

#[derive(Clone, Debug, Default)]
pub struct TreeRun {
    pub seed: u64,
    pub years: Vec<YearRec>,
    pub ages: BTreeMap<u32, AgeRec>,
    pub at_dbh: BTreeMap<u32, AtDbh>,
    pub first_flowering_age: Option<u32>,
    /// Age (years) at which the tree died (uprooted by the wind on the
    /// `vent` branch); never set on main. Later years are not used.
    pub death_age: Option<u32>,
    #[cfg(feature = "vent")]
    pub vent: crate::vent::RunDyn,
}

pub fn sample_ages(species: &str) -> Vec<u32> {
    if species.starts_with("Quercus") {
        vec![5, 10, 15, 20, 25, 30, 40, 50, 60, 70, 80, 100, 130, 144, 150, 200, 250, 300, 350, 400]
    } else {
        vec![5, 10, 15, 20, 25, 30, 40, 50, 56, 59, 60, 70, 75, 80, 100, 120, 150]
    }
}

pub const DBH_THRESHOLDS: [u32; 5] = [25, 30, 40, 50, 60];

fn dry_woody_t(g: &Genome, wood_mass_green_kg: f32) -> f64 {
    (wood_mass_green_kg / g.mechanics.green_density_kg_m3 * g.wood.density_kg_m3) as f64 / 1000.0
}

/// Trunk volume up to a 7 cm top (m3) and the corresponding length (m).
fn stem_volume_7cm(sim: &Simulation) -> (f64, f64) {
    let model = sim.mechanics_model();
    let snap = model.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) });
    let (segs, _) = sim.mechanics_inputs();
    let (mut v, mut top) = (0.0f64, 0.0f64);
    for (s, role) in segs.iter().zip(snap.segment_role.iter()) {
        if *role != SegmentRole::Trunk || s.radius_m < 0.035 {
            continue;
        }
        let d = [s.end[0] - s.start[0], s.end[1] - s.start[1], s.end[2] - s.start[2]];
        let len = ((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) as f64).sqrt();
        v += std::f64::consts::PI * (s.radius_m as f64).powi(2) * len;
        top = top.max(s.end[1].max(s.start[1]) as f64);
    }
    (v, top)
}

fn age_rec(sim: &Simulation, july_transp: f64) -> AgeRec {
    let g = sim.genome();
    let model = sim.mechanics_model();
    let still = model.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) });
    let (_, site) = sim.mechanics_inputs();
    let cr = sim.crown_radius_m() as f64;
    let leaf_area = site.leaf_area_m2 as f64;
    let lma = if leaf_area > 0.0 {
        still.leaf_mass_kg as f64 * dendro_core::mechanics::LEAF_DRY_MATTER_CONTENT as f64 * 1000.0 / leaf_area
    } else {
        0.0
    };
    let crit = |lf: f32| model.critical_speeds(&WindLoad { leaf_fraction: Some(lf), ..WindLoad::new(0.0) }).first();
    let mode = crit(0.0).map(|(m, _)| match m {
        FailureMode::StemBreak => "stem",
        FailureMode::LimbBreak => "limb",
        FailureMode::Uprooting => "uproot",
    });
    let (ar, n) = sim.debug_area_ratio(0.01);
    let m = sim.shape_metrics();
    AgeRec {
        height_m: sim.height_m() as f64,
        dbh_cm: sim.dbh_cm() as f64,
        crown_width_m: 2.0 * cr,
        agb_dry_t: dry_woody_t(g, still.wood_mass_kg),
        leaf_area_m2: leaf_area,
        lma_g_m2: lma,
        lai_crown: if cr > 0.0 { leaf_area / (std::f64::consts::PI * cr * cr) } else { 0.0 },
        crown_cd_5ms: model.crown_cd(5.0) as f64,
        first_crit_leafless: crit(0.0).map(|(_, v)| v as f64),
        first_crit_leafy: crit(1.0).map(|(_, v)| v as f64),
        first_mode_leafless: mode,
        root_frac_10: sim.root_length_fraction_above(0.10).1 as f64,
        root_frac_30: sim.root_length_fraction_above(0.30).1 as f64,
        root_frac_40: sim.root_length_fraction_above(0.40).1 as f64,
        area_ratio: ar as f64,
        area_ratio_n: n,
        july_transp_l_day: july_transp,
        clear_bole_m: m.clear_bole_m as f64,
        #[cfg(feature = "vent")]
        vent: crate::vent::age_dyn(sim),
    }
}

/// Grows one isolated tree on the reference site (default Rennes climate,
/// default soil) for `years`, day by day, recording everything.
pub fn run_tree(genome: &Genome, seed: u64, years: u32) -> TreeRun {
    run_tree_on(genome, seed, years, SoilProfile::default(), true)
}

pub fn run_tree_on(genome: &Genome, seed: u64, years: u32, soil: SoilProfile, detailed: bool) -> TreeRun {
    let mut sim = Simulation::with_site(genome.clone(), seed, ClimateParams::default(), soil);
    #[cfg(feature = "vent")]
    crate::vent::configure(&mut sim);
    let ages = if detailed { sample_ages(&genome.species) } else { Vec::new() };
    let mut run = TreeRun { seed, ..Default::default() };
    let mut prev_dbh = 0.0f64;
    let mut july_acc: BTreeMap<u32, (f64, u32)> = BTreeMap::new();
    for age in 1..=years {
        let mut y = YearRec { age, ..Default::default() };
        let (mut prev_phase, mut prev_stage) = (sim.phase(), sim.repro_stage());
        let mut july = (0.0f64, 0u32);
        // July transpiration is averaged over the 5 years centred on each sample age.
        let want_flux = detailed && ages.iter().any(|&a| a + 2 >= age && age + 2 >= a);
        let n_rings = sim.rings().len();
        for _ in 0..365 {
            let doy = sim.day_of_year();
            sim.advance_days(1);
            let ph = sim.phase();
            if ph != prev_phase {
                match ph {
                    Phase::BudBurst if y.budburst.is_none() => y.budburst = Some(doy),
                    Phase::Colouring if y.colouring.is_none() => y.colouring = Some(doy),
                    Phase::LeafFall if y.fall_start.is_none() => y.fall_start = Some(doy),
                    Phase::Dormant if y.bare.is_none() && doy > 200 => y.bare = Some(doy),
                    _ => {}
                }
                prev_phase = ph;
            }
            let st = sim.repro_stage();
            if st == ReproStage::Blooming {
                if prev_stage != ReproStage::Blooming {
                    y.bloom_start = Some(doy);
                }
                if y.bloom_full.is_none() && sim.bloom_progress() >= 0.3 {
                    y.bloom_full = Some(doy);
                }
                y.bloom_days += 1;
                y.flower_frost_loss = y.flower_frost_loss.max(sim.flower_frost_loss() as f64);
            }
            if st == ReproStage::Fruiting && y.ripe.is_none() && sim.fruit_ripeness() >= 0.99 {
                y.ripe = Some(doy);
            }
            prev_stage = st;
            if want_flux && (181..212).contains(&doy) {
                july.0 += sim.flux_snapshot().transpiration_l as f64;
                july.1 += 1;
            }
        }
        if july.1 > 0 {
            let jt = july.0 / july.1 as f64;
            for &a in ages.iter().filter(|&&a| a + 2 >= age && age + 2 >= a) {
                let e = july_acc.entry(a).or_insert((0.0, 0u32));
                e.0 += jt;
                e.1 += 1;
            }
        }
        if run.first_flowering_age.is_none() && y.bloom_start.is_some() {
            run.first_flowering_age = Some(age);
        }
        let s = sim.stats();
        y.height_m = s.height_m as f64;
        y.dbh_cm = s.dbh_cm as f64;
        y.crown_radius_m = s.crown_radius_m as f64;
        y.root_depth_m = s.root_depth_m as f64;
        y.root_spread_m = s.root_spread_m as f64;
        if sim.rings().len() > n_rings {
            y.ring_mm = sim.rings().last().map(|r| r.width_mm as f64);
            y.ring_at_m = sim.rings().last().map(|r| r.measured_at_m as f64).unwrap_or(0.0);
        }
        if detailed {
            for thr in DBH_THRESHOLDS {
                let t = thr as f64;
                if prev_dbh < t && y.dbh_cm >= t {
                    let model = sim.mechanics_model();
                    let snap = model.evaluate(&WindLoad { leaf_fraction: Some(1.0), ..WindLoad::new(0.0) });
                    let (v, l) = stem_volume_7cm(&sim);
                    run.at_dbh.insert(
                        thr,
                        AtDbh { dbh_cm: y.dbh_cm, height_m: y.height_m, agb_dry_t: dry_woody_t(sim.genome(), snap.wood_mass_kg), stem_vol_7cm_m3: v, stem_len_7cm_m: l },
                    );
                }
            }
            if ages.contains(&age) {
                run.ages.insert(age, age_rec(&sim, 0.0));
            }
        }
        prev_dbh = y.dbh_cm;
        #[cfg(feature = "vent")]
        crate::vent::year_end(&sim, &mut run, age);
        run.years.push(y);
    }
    for (a, (sum, n)) in july_acc {
        if let Some(r) = run.ages.get_mut(&a) {
            r.july_transp_l_day = sum / n as f64;
        }
    }
    run
}

/// Climate statistics over a long series (all seeds pooled per seed).
#[derive(Clone, Debug, Default)]
pub struct ClimateStats {
    pub years: u32,
    pub t_mean: f64,
    pub t_month: [f64; 12],
    pub tx_mean: f64,
    pub tn_mean: f64,
    pub dtr_month: [f64; 12],
    pub frost_days: f64,
    pub tn_le_m5: f64,
    pub tn_le_m10: f64,
    pub tx_le_0: f64,
    pub tx_ge_25: f64,
    pub tx_ge_30: f64,
    pub pet_yr: f64,
    pub rain_yr: f64,
    pub rain_month: [f64; 12],
    pub rain_ge_1: f64,
    pub rain_ge_5: f64,
    pub rain_ge_10: f64,
    /// Median over 80-year windows of the window maximum (records span ~80 years).
    pub record_tx_80y: f64,
    pub record_tx_june_80y: f64,
    pub record_tn_80y: f64,
    pub record_rain_80y: f64,
    #[cfg(feature = "vent")]
    pub wind: crate::vent::WindStats,
}

pub const MONTH_START: [u32; 13] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];

pub fn month_of(doy: u32) -> usize {
    MONTH_START.iter().rposition(|&s| s <= doy).unwrap().min(11)
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

pub fn climate_stats(seed: u64, years: u32) -> ClimateStats {
    let mut c = Climate::new(ClimateParams::default(), seed);
    let mut s = ClimateStats { years, ..Default::default() };
    let (mut tx_y, mut txj_y, mut tn_y, mut rain_y) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    #[cfg(feature = "vent")]
    let mut wacc = crate::vent::WindAcc::default();
    for y in 0..years {
        let (mut tx, mut txj, mut tn, mut rmax) = (f64::MIN, f64::MIN, f64::MAX, 0.0f64);
        for d in 0..365u32 {
            let w = c.day(y, d, false);
            let m = month_of(d);
            let (t, mx, mn, p) = (w.temp_c as f64, w.tmax_c as f64, w.tmin_c as f64, w.precip_mm as f64);
            s.t_mean += t;
            s.t_month[m] += t;
            s.tx_mean += mx;
            s.tn_mean += mn;
            s.dtr_month[m] += mx - mn;
            s.frost_days += (mn <= 0.0) as u8 as f64;
            s.tn_le_m5 += (mn <= -5.0) as u8 as f64;
            s.tn_le_m10 += (mn <= -10.0) as u8 as f64;
            s.tx_le_0 += (mx <= 0.0) as u8 as f64;
            s.tx_ge_25 += (mx >= 25.0) as u8 as f64;
            s.tx_ge_30 += (mx >= 30.0) as u8 as f64;
            s.pet_yr += w.pet_mm as f64;
            s.rain_yr += p;
            s.rain_month[m] += p;
            s.rain_ge_1 += (p >= 1.0) as u8 as f64;
            s.rain_ge_5 += (p >= 5.0) as u8 as f64;
            s.rain_ge_10 += (p >= 10.0) as u8 as f64;
            tx = tx.max(mx);
            if m == 5 {
                txj = txj.max(mx);
            }
            tn = tn.min(mn);
            rmax = rmax.max(p);
            #[cfg(feature = "vent")]
            wacc.day(&w, m, y);
        }
        tx_y.push(tx);
        txj_y.push(txj);
        tn_y.push(tn);
        rain_y.push(rmax);
    }
    let yf = years as f64;
    let nd = yf * 365.0;
    s.t_mean /= nd;
    s.tx_mean /= nd;
    s.tn_mean /= nd;
    for m in 0..12 {
        let days = (MONTH_START[m + 1] - MONTH_START[m]) as f64 * yf;
        s.t_month[m] /= days;
        s.dtr_month[m] /= days;
        s.rain_month[m] /= yf;
    }
    for v in [&mut s.frost_days, &mut s.tn_le_m5, &mut s.tn_le_m10, &mut s.tx_le_0, &mut s.tx_ge_25, &mut s.tx_ge_30, &mut s.pet_yr, &mut s.rain_yr, &mut s.rain_ge_1, &mut s.rain_ge_5, &mut s.rain_ge_10] {
        *v /= yf;
    }
    let win = |v: &[f64], f: fn(f64, f64) -> f64| -> f64 {
        let maxes: Vec<f64> = v.chunks(80).filter(|c| c.len() == 80).map(|c| c.iter().copied().reduce(f).unwrap()).collect();
        median(maxes)
    };
    s.record_tx_80y = win(&tx_y, f64::max);
    s.record_tx_june_80y = win(&txj_y, f64::max);
    s.record_tn_80y = win(&tn_y, f64::min);
    s.record_rain_80y = win(&rain_y, f64::max);
    #[cfg(feature = "vent")]
    {
        s.wind = wacc.finish(years);
    }
    s
}
