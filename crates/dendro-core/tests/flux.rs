//! Flux layer (`dendro_core::flux`): balances, seasonality, stress responses
//! and path geometry.

use dendro_core::flux::{M_CO2, M_O2};
use dendro_core::{
    ClimateParams, FluxBudget, FluxSnapshot, GasSiteKind, Genome, MycorrhizaType, Phase,
    Simulation, SoilProfile, DAYS_PER_YEAR,
};

fn oak(years: u32) -> Simulation {
    let mut s = Simulation::new(Genome::quercus_robur(), 42);
    s.advance_years(years);
    s
}

/// Snapshots of every day of the next year.
fn year_of_days(sim: &mut Simulation) -> Vec<FluxSnapshot> {
    (0..DAYS_PER_YEAR)
        .map(|_| {
            sim.advance_days(1);
            sim.flux_snapshot()
        })
        .collect()
}

#[test]
fn carbon_and_water_balances_close() {
    let mut sim = oak(30);
    for f in year_of_days(&mut sim) {
        let scale = 1.0 + f.gpp_gc + f.reserves_used_gc;
        assert!(
            f.carbon_residual_gc().abs() < 1e-4 * scale,
            "carbon residual {} ({:?})",
            f.carbon_residual_gc(),
            f.phase
        );
        assert!(
            f.water_residual_l().abs() < 1e-4 * (1.0 + f.uptake_l),
            "water residual {}",
            f.water_residual_l()
        );
        let layers: f32 = f.uptake_layers_l.iter().sum();
        assert!((layers - f.uptake_l).abs() < 1e-4 * (1.0 + f.uptake_l));
        // Growth keeps at most a share of the sap (the rest is transpired).
        assert!(
            f.transpiration_l >= 0.0 && f.growth_water_l <= 0.5 * f.xylem_l + 1e-6,
            "{} vs {}",
            f.growth_water_l,
            f.xylem_l
        );
        for x in [
            f.gpp_gc,
            f.alloc_wood_gc,
            f.alloc_roots_gc,
            f.alloc_shoots_gc,
            f.alloc_reserves_gc,
            f.alloc_myco_gc,
            f.phloem_down_gc,
            f.phloem_up_gc,
        ] {
            assert!(x >= 0.0 && x.is_finite());
        }
    }
}

#[test]
fn no_xylem_flow_in_leafless_winter_but_respiration_goes_on() {
    let mut sim = oak(40);
    sim.advance_days(14); // mid-January
    assert_eq!(sim.phase(), Phase::Dormant);
    let f = sim.flux_snapshot();
    assert_eq!(f.uptake_l, 0.0);
    assert_eq!(f.xylem_l, 0.0);
    assert_eq!(f.transpiration_l, 0.0);
    assert_eq!(f.gpp_gc, 0.0);
    assert_eq!(f.co2_in_g, 0.0);
    assert!(
        f.co2_out_g > 0.0 && f.o2_in_g > 0.0,
        "winter respiration of wood and roots"
    );
    assert_eq!(f.resp_leaf_gc, 0.0);
    assert!(
        f.reserves_used_gc > 0.0,
        "winter respiration is paid from reserves"
    );
    assert_eq!(f.phloem_down_gc, 0.0);
}

#[test]
fn phloem_reverses_in_spring() {
    for genome in [Genome::quercus_robur(), Genome::prunus_avium()] {
        let mut sim = Simulation::new(genome, 42);
        sim.advance_years(25);
        let days = year_of_days(&mut sim);
        let bb: Vec<&FluxSnapshot> = days
            .iter()
            .filter(|f| f.phase == Some(Phase::BudBurst))
            .collect();
        assert!(!bb.is_empty());
        // At budburst the sugar flows up from the reserves to the buds...
        assert!(
            bb[0].phloem_up_gc > bb[0].phloem_down_gc,
            "budburst: up {} down {}",
            bb[0].phloem_up_gc,
            bb[0].phloem_down_gc
        );
        // ...and in summer down from the leaves, never up.
        let july = &days[195];
        assert_eq!(july.phase, Some(Phase::Mature));
        assert_eq!(july.phloem_up_gc, 0.0);
        assert!(july.phloem_down_gc > 0.0);
        // Reserves fill again from midsummer.
        let stored: f32 = days[200..280].iter().map(|f| f.alloc_reserves_gc).sum();
        assert!(stored > 0.0);
    }
}

#[test]
fn gas_exchange_stoichiometry() {
    let mut sim = oak(30);
    for f in year_of_days(&mut sim) {
        // Photosynthesis: one O2 released per CO2 fixed.
        assert!((f.co2_in_g / M_CO2 - f.o2_out_g / M_O2).abs() < 1e-4 * (1.0 + f.co2_in_g));
        // Respiration: O2 taken up never exceeds CO2 released (equal when
        // aerobic; fermentation releases CO2 without O2).
        let o2 = f.o2_in_g / M_O2;
        let co2 = f.co2_out_g / M_CO2;
        assert!(o2 <= co2 * (1.0 + 1e-5) + 1e-9);
        if f.root_hypoxia == 0.0 {
            assert!((o2 - co2).abs() < 1e-4 * (1.0 + co2));
        }
        let organs: f32 = f.co2_out_organs_g.iter().sum();
        assert!((organs - f.co2_out_g).abs() < 1e-3 * (1.0 + f.co2_out_g));
    }
}

#[test]
fn drought_reduces_sap_and_sugar_flow() {
    let mut wet = oak(30);
    let mut dry = wet.clone();
    // Drought from May 1; compare July-August, once the soil has dried.
    wet.advance_days(120);
    dry.advance_days(120);
    dry.set_drought(true);
    wet.advance_days(75);
    dry.advance_days(75);
    let (mut xw, mut xd, mut gw, mut gd) = (0.0, 0.0, 0.0, 0.0);
    for _ in 0..45 {
        wet.advance_days(1);
        dry.advance_days(1);
        let (a, b) = (wet.flux_snapshot(), dry.flux_snapshot());
        xw += a.xylem_l;
        xd += b.xylem_l;
        gw += a.gpp_gc;
        gd += b.gpp_gc;
    }
    assert!(xd < 0.7 * xw, "xylem: drought {xd:.0} L vs {xw:.0} L");
    assert!(gd < 0.7 * gw, "GPP: drought {gd:.0} vs {gw:.0} g C");
    let f = dry.flux_snapshot();
    assert!(f.water_stress > 0.3, "stress {}", f.water_stress);
}

#[test]
fn annual_carbon_budget_is_plausible() {
    let mut sim = oak(40);
    let days = year_of_days(&mut sim);
    let gpp: f32 = days.iter().map(|f| f.gpp_gc).sum();
    let ra: f32 = days.iter().map(|f| f.respiration_gc()).sum();
    let area = days[195].crown_area_m2;
    // NPP/GPP ~0.47 in forests (Waring et al. 1998), with a wide spread.
    let ratio = ra / gpp;
    assert!((0.3..0.75).contains(&ratio), "Ra/GPP {ratio:.2}");
    let per_m2 = gpp / area;
    assert!(
        (500.0..2000.0).contains(&per_m2),
        "GPP {per_m2:.0} g C m-2 yr-1"
    );
    // Midsummer water-use efficiency (GPP / transpiration) in a plausible
    // range (g C per kg H2O).
    let j = &days[195];
    let wue = j.gpp_gc / j.transpiration_l.max(1e-6);
    assert!((0.5..12.0).contains(&wue), "WUE {wue:.2}");
    // A summer day moves tens to hundreds of litres through a 40-year oak.
    assert!(j.xylem_l > 5.0 && j.xylem_l < 2000.0, "{} L", j.xylem_l);
}

#[test]
fn mycorrhizae_follow_the_genome() {
    let o = Genome::quercus_robur();
    let c = Genome::prunus_avium();
    assert_eq!(o.symbiosis.mycorrhiza, MycorrhizaType::Ecto);
    assert_eq!(c.symbiosis.mycorrhiza, MycorrhizaType::Arbuscular);
    for g in [o, c] {
        let cost = g.symbiosis.carbon_cost;
        let mut sim = Simulation::new(g, 7);
        sim.advance_years(20);
        sim.advance_days(170);
        // A summer day with a carbon surplus (on hot, dry days respiration
        // can eat all of it and the fungus gets nothing).
        let mut seen = false;
        for _ in 0..60 {
            sim.advance_days(1);
            let f = sim.flux_snapshot();
            let npp = f.gpp_gc - f.resp_leaf_gc - f.resp_wood_gc - f.resp_root_gc;
            assert!(f.alloc_myco_gc <= cost * npp.max(0.0) + 1e-3);
            if npp > 0.0 && f.phase == Some(Phase::Mature) {
                assert!(f.alloc_myco_gc > 0.0 && f.myco_nutrients_g[1] > 0.0);
                seen = true;
            }
        }
        assert!(seen);
    }
    // A genome without the section has no fungus.
    let mut g = Genome::quercus_robur();
    g.symbiosis = Default::default();
    let mut sim = Simulation::new(g, 7);
    sim.advance_years(10);
    sim.advance_days(200);
    let f = sim.flux_snapshot();
    assert_eq!(f.alloc_myco_gc, 0.0);
    assert_eq!(f.myco_water_l, 0.0);
    assert!(sim.flux_geometry(FluxBudget::default()).hyphae.is_empty());
}

#[test]
fn waterlogged_roots_ferment() {
    let mut sim = Simulation::with_site(
        Genome::prunus_avium(),
        42,
        ClimateParams::default(),
        SoilProfile::gley_clay(),
    );
    sim.advance_years(15);
    let mut max_h: f32 = 0.0;
    for _ in 0..120 {
        sim.advance_days(1);
        let f = sim.flux_snapshot();
        max_h = max_h.max(f.root_hypoxia);
        if f.root_hypoxia > 0.0 {
            assert!(f.ethanol_gc > 0.0);
        }
    }
    assert!(
        max_h > 0.2,
        "gley in winter / spring should starve roots of oxygen ({max_h})"
    );
}

#[test]
fn geometry_is_deterministic_and_within_budget() {
    let sim = oak(60);
    let b = FluxBudget::default();
    let g1 = sim.flux_geometry(b);
    let g2 = sim.clone().flux_geometry(b);
    assert_eq!(g1, g2);
    assert!(!g1.shoot_paths.is_empty() && g1.shoot_paths.len() <= b.shoot_paths);
    assert!(!g1.root_paths.is_empty() && g1.root_paths.len() <= b.root_paths);
    assert!(!g1.hyphae.is_empty() && g1.hyphae.len() <= b.hyphae);
    // Only the first shoot path reaches the collar; distances grow along
    // shoots and fall (negative) along roots.
    assert_eq!(g1.shoot_paths[0].dist[0], 0.0);
    for p in &g1.shoot_paths {
        assert!(p.dist.windows(2).all(|w| w[1] >= w[0]) && p.dist[0] >= 0.0);
        assert!(p.weight.iter().all(|w| (0.0..=1.0001).contains(w)));
    }
    for p in &g1.root_paths {
        assert!(p.dist.windows(2).all(|w| w[1] <= w[0]) && p.dist[0] <= 0.0);
        assert!(p.points.iter().all(|q| q[1] <= 1e-4));
    }
    assert!(
        g1.shoot_paths[0].weight[0] > 0.99,
        "the collar carries the whole flow"
    );
    for k in [GasSiteKind::Leaf, GasSiteKind::Wood, GasSiteKind::Root] {
        assert!(g1.gas_sites.iter().any(|s| s.kind == k), "{k:?}");
    }
    let m = dendro_core::FluxGeometry::ribbon_mesh(&g1.shoot_paths, 0.0);
    assert_eq!(m.positions.len(), m.uv2.len());
    assert_eq!(m.indices.len() / 6 * 4, m.positions.len());
    assert!(m.triangle_count() < 40_000, "{}", m.triangle_count());
}
