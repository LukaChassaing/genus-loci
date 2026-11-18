//! The species registry: every `data/species/*.ron` is embedded at compile
//! time (build.rs). This is the gate for a new species file: it must parse,
//! carry a unique id and latin name, and grow without panicking. Messages
//! say what to fix in the file (see docs/species/ADDING_A_SPECIES.md).

use dendro_core::{Genome, Simulation};

#[test]
fn every_species_file_parses_with_unique_names() {
    let reg = Genome::registry();
    assert!(reg.len() >= 4, "registry has {} species", reg.len());
    let mut ids: Vec<&str> = reg.iter().map(|g| g.id.as_str()).collect();
    let mut latin: Vec<&str> = reg.iter().map(|g| g.species.as_str()).collect();
    for g in reg {
        assert!(!g.common_name_fr.trim().is_empty(), "data/species/{}.ron: common_name_fr is empty (menu label)", g.id);
        assert!(!g.species.trim().is_empty(), "data/species/{}.ron: species (latin binomial) is empty", g.id);
        // The file name is the latin binomial in snake case.
        let want = g.species.to_lowercase().replace([' ', '-'], "_");
        assert_eq!(g.id, want, "data/species/{}.ron: name the file after `species: \"{}\"` ({want}.ron)", g.id, g.species);
        // Round trip (the save format can embed a genome).
        assert_eq!(&Genome::from_ron(&g.to_ron()).unwrap(), g, "data/species/{}.ron: RON round trip", g.id);
        assert!(g.leaf.leaf_longevity_years >= 1.0, "data/species/{}.ron: leaf_longevity_years < 1", g.id);
        if !g.leaf.deciduous {
            assert!(g.leaf.evergreen(), "data/species/{}.ron: deciduous: false needs leaf_longevity_years > 1", g.id);
        }
        assert!(g.leaf.lma_g_m2 > 0.0 && g.leaf.crown_lai > 0.0, "data/species/{}.ron: lma_g_m2 and crown_lai must be > 0", g.id);
        assert!(g.reproduction.crop_interval_years >= 1.0, "data/species/{}.ron: crop_interval_years < 1", g.id);
    }
    ids.sort_unstable();
    ids.dedup();
    latin.sort_unstable();
    latin.dedup();
    assert_eq!(ids.len(), reg.len(), "duplicate species id");
    assert_eq!(latin.len(), reg.len(), "two files with the same latin name");
    // Lookups by id and by latin name agree; the oak is the default species.
    for (i, g) in reg.iter().enumerate() {
        assert_eq!(Genome::index_of(&g.id), Some(i));
        assert_eq!(Genome::index_of(&g.species), Some(i));
    }
    assert_eq!(reg[Genome::default_index()].id, "quercus_robur");
    // Menu order: grouped by category.
    assert!(reg.windows(2).all(|w| w[0].category <= w[1].category), "registry not grouped by category");
}

#[test]
fn every_species_grows_for_a_few_years() {
    let hs: Vec<_> = Genome::builtin()
        .into_iter()
        .map(|g| {
            std::thread::spawn(move || {
                let id = g.id.clone();
                let mut s = Simulation::new(g, 7);
                s.advance_years(8);
                let (h, d) = (s.height_m(), s.dbh_cm());
                let ok = s.is_alive() && h > 0.5 && h < 20.0 && s.rings().len() == 8 && !s.wood_mesh().positions.is_empty();
                (id, ok, h, d)
            })
        })
        .collect();
    for h in hs {
        let (id, ok, height, dbh) = h.join().unwrap_or_else(|_| panic!("a species panicked while growing"));
        assert!(ok, "data/species/{id}.ron: after 8 years height {height:.2} m, DBH {dbh:.1} cm, or rings / mesh missing");
    }
}

/// Foliage and wood meshes must build for every species at an age where it
/// is in leaf (mid-July of year 12). Catches a genome trait that sends the
/// mesher out of its assumed range (a panicking `clamp`, an empty crown).
#[test]
fn every_species_builds_leaf_and_wood_meshes() {
    let hs: Vec<_> = Genome::builtin()
        .into_iter()
        .map(|g| {
            std::thread::spawn(move || {
                let id = g.id.clone();
                let mut s = Simulation::new(g, 7);
                s.advance_days(365 * 12 + 200);
                (id, s.leaf_mesh().positions.len(), s.wood_mesh().positions.len())
            })
        })
        .collect();
    for h in hs {
        let (id, leaf, wood) = h.join().unwrap_or_else(|_| panic!("a species panicked while building its meshes (data/species/*.ron: check leaf.cluster_size_leaves and other leaf traits)"));
        assert!(leaf > 0, "data/species/{id}.ron: no foliage mesh in July of year 12");
        assert!(wood > 0, "data/species/{id}.ron: no wood mesh at year 12");
    }
}

/// Any positive `cluster_size_leaves` gives a valid foliage mesh (it used to
/// panic below ~2.2 leaf lengths).
#[test]
fn small_leaf_clusters_do_not_break_the_foliage_mesh() {
    for size in [0.3_f32, 1.0, 1.5, 2.1] {
        let mut g = Genome::builtin().into_iter().find(|g| g.id == "quercus_robur").unwrap();
        g.leaf.cluster_size_leaves = size;
        let mut s = Simulation::new(g, 7);
        s.advance_days(365 * 8 + 200);
        let m = s.leaf_mesh();
        assert!(!m.positions.is_empty(), "cluster_size_leaves = {size}: empty foliage");
        assert!(m.positions.iter().flatten().all(|v| v.is_finite()), "cluster_size_leaves = {size}: non-finite vertex");
        assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()), "cluster_size_leaves = {size}: bad index");
    }
}
