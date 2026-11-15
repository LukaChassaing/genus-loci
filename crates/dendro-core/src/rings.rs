//! Annual growth rings.
//!
//! Ring-porous oak forms its earlywood (wide vessels) around budburst from
//! the previous year's reserves, so earlywood width is fairly constant
//! between years, while latewood is formed during summer and carries most of
//! the climate signal (drought => narrow latewood). The model follows this:
//! earlywood increment scales with last year's productivity, latewood with
//! the current summer's productivity (see `sim.rs`).
//!
//! Basis: Barbaroux & Breda 2002, "Contrasting distribution and seasonal
//! dynamics of carbohydrate reserves in stem wood of adult ring-porous
//! sessile oak and diffuse-porous beech trees", Tree Physiol 22:1201-1210,
//! doi:10.1093/treephys/22.17.1201 (oak starch reserves drop sharply during
//! budburst and early wood growth, consistent with earlywood being built from
//! stored carbohydrate). The `ring_porous` / `earlywood_share` split itself,
//! and the climate signal being carried by latewood, is the standard
//! dendrochronological picture of oak; the model's exact rule (previous-year
//! productivity for earlywood) is a stylisation, not a fitted relation.

/// One annual ring, measured on the stem.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    /// Calendar year index (0 = first simulated year).
    pub year: u32,
    /// Total ring width (mm).
    pub width_mm: f32,
    /// Earlywood width (mm).
    pub earlywood_mm: f32,
    /// Latewood width (mm).
    pub latewood_mm: f32,
    /// Earlywood fraction of the ring 0..1.
    pub earlywood_fraction: f32,
    /// Mean water stress over the leafy season 0..1 (0 = none, 1 = stomata
    /// closed every day).
    pub water_stress: f32,
    /// True if a drought was forced from outside during this growing season.
    pub forced_drought: bool,
    /// Height (m) where the ring is measured: 1.3 m, or the collar while the
    /// stem is shorter than ~1.3 m.
    pub measured_at_m: f32,
}

/// Radius increment (mm) produced by an area increment `da` (m2) on a stem
/// section of area `a` (m2). Pure geometry (circular section), no source
/// needed; the stem is treated as round, ring eccentricity is ignored.
pub(crate) fn width_mm(a_before: f32, a_after: f32) -> f32 {
    let pi = std::f32::consts::PI;
    ((a_after / pi).sqrt() - (a_before / pi).sqrt()).max(0.0) * 1000.0
}
