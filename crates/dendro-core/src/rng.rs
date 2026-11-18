//! Deterministic, order-independent pseudo-randomness.
//!
//! Every random draw in the simulation is a pure function of
//! `(seed, stream, key...)` hashed with SplitMix64. This makes results
//! independent of evaluation order and of how many days are advanced per
//! call, which is what guarantees `advance_days(365) == 365 x advance_days(1)`.

use crate::fmath::DMath;

#[inline]
pub(crate) fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash of a seed, a stream id and two keys.
#[inline]
pub(crate) fn hash(seed: u64, stream: u64, a: u64, b: u64) -> u64 {
    mix(mix(mix(seed ^ stream.wrapping_mul(0xA24B_AED4_963E_E407)) ^ a) ^ b.wrapping_mul(0x9FB2_1C65_1E98_DF25))
}

/// Uniform in [0, 1).
#[inline]
pub(crate) fn unit(h: u64) -> f32 {
    (h >> 40) as f32 * (1.0 / (1u64 << 24) as f32)
}

/// Uniform in (0, 1] as f64 (safe for `ln`).
#[inline]
fn unit_open(h: u64) -> f64 {
    ((h >> 11) as f64 + 1.0) * (1.0 / (1u64 << 53) as f64)
}

/// Standard normal deviate (Box-Muller) from one 64-bit hash.
#[inline]
pub(crate) fn normal(h: u64) -> f32 {
    let u1 = unit_open(h);
    let u2 = unit_open(mix(h));
    ((-2.0 * u1.dln()).sqrt() * (std::f64::consts::TAU * u2).dcos()) as f32
}

/// Exponential deviate with mean 1.
#[inline]
pub(crate) fn exponential(h: u64) -> f32 {
    (-unit_open(h).dln()) as f32
}

// Stream identifiers (arbitrary distinct constants).
pub(crate) const S_YEAR: u64 = 1;
pub(crate) const S_TEMP: u64 = 2;
pub(crate) const S_RAIN: u64 = 3;
pub(crate) const S_RAIN_AMT: u64 = 4;
pub(crate) const S_SHOOT: u64 = 10;
pub(crate) const S_BUD: u64 = 11;
pub(crate) const S_RETRENCH: u64 = 12;
pub(crate) const S_ROOT: u64 = 20;
pub(crate) const S_LEAF: u64 = 30;
pub(crate) const S_FLOWER: u64 = 31;
/// Crop years (masting, alternate bearing), see `phenology::Reproduction`.
pub(crate) const S_CROP: u64 = 32;
