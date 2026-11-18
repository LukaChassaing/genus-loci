//! Daily maximum and minimum temperature: a WGEN-type stochastic generator.
//!
//! Structure (Richardson 1981, "Stochastic simulation of daily
//! precipitation, temperature, and solar radiation", Water Resour Res
//! 17(1):182-190, doi:10.1029/WR017i001p00182; Richardson & Wright 1984, "WGEN: a model for generating
//! daily weather variables", USDA-ARS ARS-8; both cited, not read; the
//! equations below are those of the SWAT+ theoretical documentation,
//! section 1:3.4.1 "Daily residuals", read):
//! * Tmax and Tmin are each a seasonal mean, conditioned on whether the day
//!   is wet or dry, plus a seasonal standard deviation times a standardised
//!   residual;
//! * the two residuals follow a multivariate first-order autoregression
//!   `chi_t = A chi_(t-1) + B eps_t` (eps: independent standard normal
//!   deviates), with `A = M1 M0^-1` and `B B^T = M0 - A M1^T`, where M0 holds
//!   the lag-0 and M1 the lag-1 cross-correlations of the residuals.
//!
//! Values and calibration:
//! * M0 and M1 are the generalised values of the SWAT+ documentation
//!   (eqs. 1:3.4.6-7, attributed there to Richardson 1982, "Dependence
//!   structure of daily temperature and solar radiation", Trans ASAE
//!   25(3):735-739, doi:10.13031/2013.33604, not read: 20 years of data at
//!   31 US stations): lag-0 correlation Tmax-Tmin 0.633, lag-1
//!   autocorrelation 0.621 (Tmax) and 0.674 (Tmin), lag-1 cross
//!   correlations 0.445 (Tmax today / Tmin yesterday) and 0.563 (Tmin today
//!   / Tmax yesterday). Generic values, not fitted to Rennes. The solar
//!   radiation variable of WGEN is dropped and A, B recomputed for the two
//!   temperatures from these correlations (`ar_matrices`); the result is
//!   close to the 3x3 published matrices (A11 0.566 vs 0.567, B11 0.781).
//! * Seasonal means: the 12 monthly normals of Tn and Tx of the Meteo-France
//!   fiche of Rennes-St Jacques (35281001, 1991-2020, read in full),
//!   interpolated linearly between mid-months with knots solved so that each
//!   calendar month keeps its normal (mean-preserving interpolation).
//!   WGEN uses one Fourier harmonic instead; the monthly table follows the
//!   asymmetric oceanic seasons (coldest Tn in February, Tx flat from July to
//!   August) that a single cosine misses.
//! * Wet/dry conditioning: on a wet day the diurnal range is 0.75 x the
//!   monthly normal (Tx lowered, Tn raised by the same amount); dry days are
//!   shifted the other way so that the monthly means are kept. The 0.75 is
//!   estimated (no daily Rennes data were used): cloudy wet days have a
//!   smaller range, clear dry days a larger one (warmer afternoons, colder
//!   nights).
//! * Monthly standard deviations of the residuals: calibrated by simulation
//!   on the monthly counts of the same fiche, for each month on one count
//!   only: Tn <= 0 degC (October to April), Tx >= 25 degC (April-June,
//!   September-October), Tx <= 0 degC (December to February); the other
//!   months are linearly interpolated between their neighbours (values and
//!   details: `ClimateParams` defaults). The counts Tn <= -5, Tn <= -10 and
//!   Tx >= 30 degC and the records were left out of the fit (out-of-sample
//!   checks of the tails, see docs/validation).
//! * Residuals are Gaussian (as in WGEN). The station counts show where this
//!   fails: in summer the Tx distribution is right-skewed (too few Tx >= 30
//!   days once Tx >= 25 is matched) and in winter the Tn tail below -5 degC
//!   is lighter than a Gaussian fitted on Tn <= 0 (too many Tn <= -5 days):
//!   see "Ecarts connus" in docs/sources.md.

/// First day (0-based) of each month in a 365-day year, and the year end.
pub const MONTH_START: [u32; 13] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];

/// Lag-0 correlation of the Tmax and Tmin residuals (SWAT+ doc. eq.
/// 1:3.4.6, after Richardson 1982).
const RHO0_X_N: f32 = 0.633;
/// Lag-1 correlations `[[Tx_t~Tx_t-1, Tx_t~Tn_t-1], [Tn_t~Tx_t-1, Tn_t~Tn_t-1]]`
/// (SWAT+ doc. eq. 1:3.4.7, after Richardson 1982).
const RHO1: [[f32; 2]; 2] = [[0.621, 0.445], [0.563, 0.674]];

/// Ratio of the wet-day diurnal range to the monthly normal range
/// (estimated, see the module header).
pub const WET_DTR_RATIO: f32 = 0.75;

/// Smallest daily range kept (degC): when the two residuals cross, Tx and Tn
/// are moved apart symmetrically around their midpoint.
pub const MIN_DTR_C: f32 = 1.0;

/// `(A, B)` of the bivariate AR(1) `chi_t = A chi_(t-1) + B eps_t`, from the
/// lag-0 and lag-1 correlations (B: lower Cholesky factor).
pub fn ar_matrices() -> ([[f32; 2]; 2], [[f32; 2]; 2]) {
    let r = RHO0_X_N;
    let det = 1.0 - r * r;
    // M0^-1 = [[1, -r], [-r, 1]] / det.
    let inv = [[1.0 / det, -r / det], [-r / det, 1.0 / det]];
    let mut a = [[0.0f32; 2]; 2];
    for (i, row) in a.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = RHO1[i][0] * inv[0][j] + RHO1[i][1] * inv[1][j];
        }
    }
    // C = M0 - A M1^T (symmetric, positive definite for these values).
    let am1t = |i: usize, j: usize| a[i][0] * RHO1[j][0] + a[i][1] * RHO1[j][1];
    let c00 = 1.0 - am1t(0, 0);
    let c10 = r - am1t(1, 0);
    let c11 = 1.0 - am1t(1, 1);
    let b00 = c00.sqrt();
    let b10 = c10 / b00;
    let b11 = (c11 - b10 * b10).sqrt();
    (a, [[b00, 0.0], [b10, b11]])
}

/// Knots of a cyclic piecewise-linear daily curve whose calendar-month means
/// equal `monthly` (knots at mid-month; fixed-point iteration, exact
/// arithmetic only).
pub fn mean_preserving_knots(monthly: &[f32; 12]) -> [f32; 12] {
    let mut k = *monthly;
    for _ in 0..40 {
        let mut means = [0.0f32; 12];
        let mut counts = [0.0f32; 12];
        for d in 0..365u32 {
            let m = month_of(d);
            means[m] += interpolate(&k, d);
            counts[m] += 1.0;
        }
        for m in 0..12 {
            k[m] += monthly[m] - means[m] / counts[m];
        }
    }
    k
}

/// Month (0..12) of a 0-based day of year (days past 364 count as December).
pub fn month_of(doy: u32) -> usize {
    MONTH_START.iter().rposition(|&s| s <= doy).unwrap_or(0).min(11)
}

fn mid_month(m: usize) -> f32 {
    0.5 * (MONTH_START[m] + MONTH_START[m + 1]) as f32
}

/// Value on day `doy` of the cyclic piecewise-linear curve through the 12
/// mid-month `knots` (evaluated at the middle of the day).
pub fn interpolate(knots: &[f32; 12], doy: u32) -> f32 {
    let t = doy.min(364) as f32 + 0.5;
    // Find the knot at or before t (cyclically).
    let mut i = 11usize;
    for m in 0..12 {
        if mid_month(m) <= t {
            i = m;
        }
    }
    let j = (i + 1) % 12;
    let (ti, mut tj) = (mid_month(i), mid_month(j));
    let mut tt = t;
    if tj <= ti {
        tj += 365.0;
    }
    if tt < ti {
        tt += 365.0;
    }
    let f = (tt - ti) / (tj - ti);
    knots[i] + f * (knots[j] - knots[i])
}

/// Day-of-year tables computed once from the monthly parameters.
#[derive(Clone, Debug)]
pub struct TempTables {
    pub tn_knots: [f32; 12],
    pub tx_knots: [f32; 12],
    pub tn_sd_knots: [f32; 12],
    pub tx_sd_knots: [f32; 12],
    pub a: [[f32; 2]; 2],
    pub b: [[f32; 2]; 2],
}

impl TempTables {
    pub fn new(tn: &[f32; 12], tx: &[f32; 12], tn_sd: &[f32; 12], tx_sd: &[f32; 12]) -> Self {
        let (a, b) = ar_matrices();
        TempTables {
            tn_knots: mean_preserving_knots(tn),
            tx_knots: mean_preserving_knots(tx),
            // Standard deviations: plain interpolation through the monthly
            // values (they are calibrated through the generator itself).
            tn_sd_knots: *tn_sd,
            tx_sd_knots: *tx_sd,
            a,
            b,
        }
    }

    /// Advance the residual pair by one day with two standard normal
    /// deviates.
    pub fn step(&self, chi: [f32; 2], eps: [f32; 2]) -> [f32; 2] {
        let (a, b) = (&self.a, &self.b);
        [
            a[0][0] * chi[0] + a[0][1] * chi[1] + b[0][0] * eps[0],
            a[1][0] * chi[0] + a[1][1] * chi[1] + b[1][0] * eps[0] + b[1][1] * eps[1],
        ]
    }
}

/// Day-weighted annual mean of a monthly table.
pub fn annual_mean(monthly: &[f32; 12]) -> f32 {
    let mut s = 0.0f32;
    for (m, v) in monthly.iter().enumerate() {
        s += v * (MONTH_START[m + 1] - MONTH_START[m]) as f32;
    }
    s / 365.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ar_matrices_match_richardson_published_values() {
        // SWAT+ doc. eqs. 1:3.4.8-9 (3 variables): A11 0.567, A12 0.086,
        // A21 0.253, A22 0.504; B11 0.781, B21 0.328, B22 0.637. Dropping
        // radiation changes A slightly, B hardly.
        let (a, b) = ar_matrices();
        assert!((a[0][0] - 0.567).abs() < 0.01 && (a[0][1] - 0.086).abs() < 0.01, "{a:?}");
        assert!((a[1][0] - 0.253).abs() < 0.04 && (a[1][1] - 0.504).abs() < 0.04, "{a:?}");
        assert!((b[0][0] - 0.781).abs() < 0.005 && (b[1][0] - 0.328).abs() < 0.005 && (b[1][1] - 0.637).abs() < 0.005, "{b:?}");
        // Stationary process: unit variances and lag-0 correlation 0.633.
        let t = TempTables::new(&[0.0; 12], &[0.0; 12], &[1.0; 12], &[1.0; 12]);
        let (mut chi, mut s) = ([0.0f32; 2], [0.0f64; 3]);
        let n = 200_000u64;
        for i in 0..n {
            let e = [crate::rng::normal(crate::rng::hash(1, 99, i, 0)), crate::rng::normal(crate::rng::hash(1, 99, i, 1))];
            chi = t.step(chi, e);
            s[0] += (chi[0] * chi[0]) as f64;
            s[1] += (chi[1] * chi[1]) as f64;
            s[2] += (chi[0] * chi[1]) as f64;
        }
        let (vx, vn, c) = (s[0] / n as f64, s[1] / n as f64, s[2] / n as f64);
        assert!((vx - 1.0).abs() < 0.03 && (vn - 1.0).abs() < 0.03, "{vx} {vn}");
        assert!((c / (vx * vn).sqrt() - 0.633).abs() < 0.02, "{c}");
    }

    #[test]
    fn interpolation_keeps_monthly_means() {
        let tn = [3.3, 2.9, 4.5, 6.0, 9.3, 12.1, 13.7, 13.8, 11.4, 9.3, 5.9, 3.6];
        let k = mean_preserving_knots(&tn);
        for m in 0..12 {
            let (a, b) = (MONTH_START[m], MONTH_START[m + 1]);
            let mean = (a..b).map(|d| interpolate(&k, d)).sum::<f32>() / (b - a) as f32;
            assert!((mean - tn[m]).abs() < 0.01, "month {m}: {mean} vs {}", tn[m]);
        }
        // Continuous across the year end.
        assert!((interpolate(&k, 364) - interpolate(&k, 0)).abs() < 0.2);
    }
}
