//! Procedural temperate-oceanic climate and a soil water bucket.
//!
//! Weather generator for western Europe, anchored on the Meteo-France
//! 1991-2020 statistics of Rennes-St Jacques (48.07 degN, station 35281001,
//! "Fiche climatologique, statistiques 1991-2020 et records", edition of
//! 06/06/2025, read in full): annual mean 12.4 degC, monthly mean Tn from
//! 2.9 degC (February) to 13.8 degC (August) and Tx from 9.2 degC (January)
//! to 24.8 degC (July), 30.1 days a year with Tn <= 0 degC, 43.5 with Tx >=
//! 25 degC; 691 mm of precipitation (115 days >= 1 mm), wetter
//! autumn-winter, Penman PET 837 mm/yr (2001-2020).
//!
//! Temperature: daily Tmax and Tmin from a WGEN-type generator (Richardson
//! 1981; see `crate::tempgen` for the model, its sources and its calibration
//! on the monthly normals and day counts of the fiche); the daily mean is
//! (Tx + Tn) / 2, the Meteo-France definition. On top of it each year draws
//! seeded anomalies (warm/cold year, cold spring, dry summer), whose
//! expected values are removed from the seasonal means so that the normals
//! are kept. The anomaly probabilities and sizes are stylised settings (no
//! source).
//!
//! Precipitation: occurrence and exponential amounts with a seasonal cosine
//! (calibration, see `Climate::day`).
//!
//! Potential evapotranspiration: Hargreaves & Samani 1985, "Reference crop
//! evapotranspiration from temperature", Appl Eng Agric 1(2):96-99, in the
//! form given as the temperature-only fallback of FAO-56 (Allen et al.
//! 1998, FAO Irrigation and Drainage Paper 56, eq. 52), with the
//! extraterrestrial radiation Ra of FAO-56 eq. 21 (eqs. 23-25 for the
//! inverse Earth-Sun distance, declination and sunset hour angle). Inputs:
//! the generated daily mean, the generated diurnal range and latitude.
//! Full Penman-Monteith would need radiation, humidity and wind that this
//! generator does not produce honestly.

use crate::fmath::DMath;
use crate::rng::{self, S_RAIN, S_RAIN_AMT, S_TEMP, S_YEAR};
use crate::tempgen::{self, TempTables};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// Climate parameters (defaults: Rennes-St Jacques, temperate oceanic,
/// 48 degN).
///
/// Saved in every save file: fields are never reused with a new meaning,
/// and new fields carry a serde default (older saves, which also hold the
/// fields of the former AR(1) temperature model, `temp_amplitude_c`,
/// `coldest_doy`, `daily_temp_sd`, `daily_temp_autocorr`, `dtr_mean_c`,
/// `dtr_amplitude_c`, load with the current default temperature model:
/// unknown fields are ignored).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClimateParams {
    pub latitude_deg: f32,
    /// Annual mean temperature (degC). The monthly normal tables below are
    /// shifted uniformly so that their day-weighted annual mean of
    /// (Tn + Tx) / 2 equals this value.
    pub mean_temp_c: f32,
    /// Standard deviation of the annual mean anomaly (degC).
    pub annual_temp_sd: f32,
    /// Mean annual precipitation (mm).
    pub annual_precip_mm: f32,
    /// Mean probability that a day is wet.
    pub wet_day_prob: f32,
    /// Probability that a year has a dry, hot summer.
    pub dry_year_prob: f32,
    /// Probability that a year has a cold spring.
    pub cold_spring_prob: f32,
    /// Monthly normals of the daily minimum and maximum temperature (degC,
    /// January first; see `crate::tempgen`).
    #[serde(default = "tn_normals_default")]
    pub tn_normals_c: [f32; 12],
    #[serde(default = "tx_normals_default")]
    pub tx_normals_c: [f32; 12],
    /// Monthly standard deviations of the Tn and Tx residuals (degC;
    /// calibrated, see `crate::tempgen`).
    #[serde(default = "tn_sd_default")]
    pub tn_sd_c: [f32; 12],
    #[serde(default = "tx_sd_default")]
    pub tx_sd_c: [f32; 12],
    /// Multiplier of the residual standard deviations (1 = the calibrated
    /// station climate; larger values give a more variable climate).
    #[serde(default = "temp_sd_scale_default")]
    pub temp_sd_scale: f32,
}

/// Meteo-France, fiche climatologique Rennes-St Jacques (35281001),
/// "Temperature minimale (moyenne en degC)", 1991-2020.
fn tn_normals_default() -> [f32; 12] {
    [3.3, 2.9, 4.5, 6.0, 9.3, 12.1, 13.7, 13.8, 11.4, 9.3, 5.9, 3.6]
}

/// Same fiche, "Temperature maximale (moyenne en degC)", 1991-2020.
fn tx_normals_default() -> [f32; 12] {
    [9.2, 10.2, 13.2, 16.0, 19.3, 22.6, 24.8, 24.7, 21.9, 17.2, 12.5, 9.6]
}

/// Calibration (`crate::tempgen` header): fitted by simulation (5 000 years
/// x 4 seeds, scratch script not shipped) to the monthly counts Tn <= 0 degC
/// of the fiche, January-April and October-December; May-September linearly
/// interpolated between April and October.
fn tn_sd_default() -> [f32; 12] {
    [4.67, 4.6, 3.32, 2.71, 2.88, 3.04, 3.2, 3.36, 3.52, 3.68, 4.16, 5.14]
}

/// Calibration (same fit): monthly counts Tx >= 25 degC in April, May, June,
/// September and October, Tx <= 0 degC in December-February; March, July,
/// August and November linearly interpolated. July and August are not fitted:
/// their mean Tx (24.8, 24.7 degC) sits at the 25 degC threshold, and the
/// station's 13.8 and 12.2 days (fewer than half the days although the mean
/// is almost 25) reveal a right-skewed distribution that a Gaussian residual
/// cannot reproduce at any standard deviation.
fn tx_sd_default() -> [f32; 12] {
    [4.3, 4.0, 4.11, 4.22, 4.35, 2.91, 2.99, 3.07, 3.15, 2.96, 3.35, 3.73]
}

fn temp_sd_scale_default() -> f32 {
    1.0
}

impl Default for ClimateParams {
    /// Rennes-St Jacques 1991-2020 (see the module header).
    fn default() -> Self {
        ClimateParams {
            latitude_deg: 48.07,
            mean_temp_c: 12.4,
            annual_temp_sd: 0.6,
            // Calibration: 760 gives the station's ~690 mm once dry years
            // and the generator's other terms are applied (tested below).
            annual_precip_mm: 760.0,
            wet_day_prob: 0.42,
            dry_year_prob: 0.12,
            cold_spring_prob: 0.15,
            tn_normals_c: tn_normals_default(),
            tx_normals_c: tx_normals_default(),
            tn_sd_c: tn_sd_default(),
            tx_sd_c: tx_sd_default(),
            temp_sd_scale: temp_sd_scale_default(),
        }
    }
}

/// One day of weather.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Weather {
    /// Daily mean air temperature (degC).
    pub temp_c: f32,
    /// Daily minimum / maximum air temperature (degC; `temp_c` is their
    /// mean).
    pub tmin_c: f32,
    pub tmax_c: f32,
    /// Precipitation (mm).
    pub precip_mm: f32,
    /// Potential evapotranspiration (mm).
    pub pet_mm: f32,
    /// Day length (h).
    pub photoperiod_h: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct YearAnomaly {
    temp_offset: f32,
    cold_spring: bool,
    dry_summer: bool,
    precip_mult: f32,
}

/// Stateful daily weather generator.
#[derive(Clone, Debug)]
pub struct Climate {
    params: ClimateParams,
    seed: u64,
    /// Standardised Tmax / Tmin residuals of the previous day.
    chi: [f32; 2],
    tables: TempTables,
    /// Uniform shift of the normal tables to `mean_temp_c` (degC).
    shift_c: f32,
    year: u32,
    anomaly: YearAnomaly,
}

/// Day length (h) from latitude and 0-based day of year, using solar
/// declination and the standard -0.833 deg horizon correction.
///
/// Same family as the daylength models compared in Forsythe et al. 1995, "A
/// model comparison for daylength as a function of latitude and day of year",
/// Ecol Model 80:87-95, doi:10.1016/0304-3800(94)00034-f (who find such
/// formulas within minutes of almanac values at mid-latitudes). The
/// declination expression here is the common sinusoidal approximation, not
/// their exact CBM formula.
pub fn photoperiod_h(latitude_deg: f32, doy: u32) -> f32 {
    let decl = 23.44_f32.to_radians() * (TAU * (284.0 + doy as f32 + 1.0) / 365.0).dsin();
    let lat = latitude_deg.to_radians();
    let c = ((-0.833_f32).to_radians().dsin() - lat.dsin() * decl.dsin()) / (lat.dcos() * decl.dcos());
    let h = c.clamp(-1.0, 1.0).dacos();
    24.0 * h / std::f32::consts::PI
}

/// Daily extraterrestrial radiation Ra (MJ m-2 day-1) at a latitude and
/// 0-based day of year: FAO-56 (Allen et al. 1998) eq. 21, with eq. 23
/// (inverse relative Earth-Sun distance), eq. 24 (declination) and eq. 25
/// (sunset hour angle); solar constant 0.0820 MJ m-2 min-1.
pub fn extraterrestrial_radiation_mj(latitude_deg: f32, doy: u32) -> f32 {
    let j = doy as f32 + 1.0;
    let phi = latitude_deg.to_radians();
    let dr = 1.0 + 0.033 * (TAU * j / 365.0).dcos();
    let decl = 0.409 * (TAU * j / 365.0 - 1.39).dsin();
    let ws = (-phi.dtan() * decl.dtan()).clamp(-1.0, 1.0).dacos();
    let ra = 24.0 * 60.0 / std::f32::consts::PI
        * 0.0820
        * dr
        * (ws * phi.dsin() * decl.dsin() + phi.dcos() * decl.dcos() * ws.dsin());
    ra.max(0.0)
}

/// Reference evapotranspiration (mm/day) after Hargreaves & Samani 1985
/// (Appl Eng Agric 1:96-99), as FAO-56 eq. 52:
/// `ET0 = 0.0023 (Tmean + 17.8) (Tmax - Tmin)^0.5 x 0.408 Ra`.
pub fn hargreaves_pet_mm(tmean_c: f32, dtr_c: f32, ra_mj: f32) -> f32 {
    (0.0023 * (tmean_c + 17.8) * dtr_c.max(0.0).sqrt() * 0.408 * ra_mj).max(0.0)
}

/// Cooling of a cold spring and warming of a dry summer (degC; stylised,
/// no source).
const COLD_SPRING_C: f32 = 2.5;
const DRY_SUMMER_C: f32 = 1.3;
/// Frequencies of cold springs and dry summers assumed to be contained in
/// the station normals (the defaults of `ClimateParams`).
const REF_COLD_SPRING_PROB: f32 = 0.15;
const REF_DRY_YEAR_PROB: f32 = 0.12;

/// Smooth bump: 1 in [a+r, b-r], 0 outside [a, b], linear ramps.
fn window(doy: f32, a: f32, b: f32, r: f32) -> f32 {
    ((doy - a) / r).clamp(0.0, 1.0) * ((b - doy) / r).clamp(0.0, 1.0)
}

impl Climate {
    pub fn new(params: ClimateParams, seed: u64) -> Self {
        let tables = TempTables::new(&params.tn_normals_c, &params.tx_normals_c, &params.tn_sd_c, &params.tx_sd_c);
        let table_mean = 0.5 * (tempgen::annual_mean(&params.tn_normals_c) + tempgen::annual_mean(&params.tx_normals_c));
        let shift_c = params.mean_temp_c - table_mean;
        let mut c = Climate { params, seed, chi: [0.0; 2], tables, shift_c, year: u32::MAX, anomaly: YearAnomaly::default() };
        c.set_year(0);
        c
    }

    pub fn params(&self) -> &ClimateParams {
        &self.params
    }

    fn set_year(&mut self, year: u32) {
        if self.year == year {
            return;
        }
        self.year = year;
        let s = self.seed;
        let y = year as u64;
        let p = &self.params;
        self.anomaly = YearAnomaly {
            temp_offset: p.annual_temp_sd * rng::normal(rng::hash(s, S_YEAR, y, 0)),
            cold_spring: rng::unit(rng::hash(s, S_YEAR, y, 1)) < p.cold_spring_prob,
            dry_summer: rng::unit(rng::hash(s, S_YEAR, y, 2)) < p.dry_year_prob,
            precip_mult: (1.0 + 0.15 * rng::normal(rng::hash(s, S_YEAR, y, 3))).clamp(0.6, 1.4),
        };
    }

    /// True if the given year was drawn as a naturally dry summer.
    pub fn is_dry_year(&mut self, year: u32) -> bool {
        let prev = self.year;
        self.set_year(year);
        let d = self.anomaly.dry_summer;
        self.set_year(prev);
        d
    }

    /// Weather of day `doy` (0..365) of `year`. Must be called once per day in
    /// order (the daily anomaly is autoregressive).
    pub fn day(&mut self, year: u32, doy: u32, forced_drought: bool) -> Weather {
        self.set_year(year);
        let p = &self.params;
        let a = self.anomaly;
        let d = doy as f32;
        let s = self.seed;

        // Precipitation: wetter autumn-winter (oceanic regime; Rennes has
        // ~73 mm/month in Oct-Dec against ~44 mm in Jul-Aug). Calibration:
        // peak in mid-December, +-18 % wet-day frequency and +-10 % amount.
        let summer = window(d, 125.0, 265.0, 25.0);
        let season = (TAU * (d - 350.0) / 365.0).dcos();
        let p_wet_clim = p.wet_day_prob * (1.0 + 0.18 * season);
        let mut p_wet = p_wet_clim;
        let mean_amount = p.annual_precip_mm / (365.0 * p.wet_day_prob) * (1.0 + 0.10 * season);
        let mut mult = a.precip_mult;
        if a.dry_summer {
            p_wet *= 1.0 - 0.6 * summer;
            mult *= 1.0 - 0.5 * summer;
        }
        let wet = rng::unit(rng::hash(s, S_RAIN, year as u64, doy as u64)) < p_wet;
        let mut precip = if wet {
            mean_amount * mult * rng::exponential(rng::hash(s, S_RAIN_AMT, year as u64, doy as u64))
        } else {
            0.0
        };

        // Temperature: WGEN-type Tmax/Tmin (see `crate::tempgen`).
        let t = &self.tables;
        let (tn_norm, tx_norm) = (tempgen::interpolate(&t.tn_knots, doy), tempgen::interpolate(&t.tx_knots, doy));
        // Wet/dry conditioning around the normals: a wet day has
        // WET_DTR_RATIO x the normal range, dry days the complement so that
        // the climatological wet-day frequency keeps the monthly means.
        let pw = p_wet_clim.clamp(0.0, 0.9);
        let half = 0.5 * (1.0 - tempgen::WET_DTR_RATIO) * (tx_norm - tn_norm);
        let off = if wet { -half } else { half * pw / (1.0 - pw) };
        // Year anomalies, minus their expected value in the station climate
        // (its normals already contain the average year). The reference
        // frequencies are fixed: raising `cold_spring_prob` or
        // `dry_year_prob` still makes springs colder or summers warmer on
        // average.
        let spring = window(d, 55.0, 150.0, 20.0);
        let mut anomaly = a.temp_offset + COLD_SPRING_C * REF_COLD_SPRING_PROB * spring - DRY_SUMMER_C * REF_DRY_YEAR_PROB * summer;
        if a.cold_spring {
            anomaly -= COLD_SPRING_C * spring;
        }
        if a.dry_summer {
            anomaly += DRY_SUMMER_C * summer;
        }
        let eps = [
            rng::normal(rng::hash(s, S_TEMP, year as u64, doy as u64)),
            rng::normal(rng::hash(s, S_TEMP, year as u64, 1000 + doy as u64)),
        ];
        self.chi = t.step(self.chi, eps);
        let k = p.temp_sd_scale;
        let base = self.shift_c + anomaly;
        let mut tmax = tx_norm + base + off + k * tempgen::interpolate(&t.tx_sd_knots, doy) * self.chi[0];
        let mut tmin = tn_norm + base - off + k * tempgen::interpolate(&t.tn_sd_knots, doy) * self.chi[1];
        if tmax - tmin < tempgen::MIN_DTR_C {
            let mid = 0.5 * (tmax + tmin);
            tmax = mid + 0.5 * tempgen::MIN_DTR_C;
            tmin = mid - 0.5 * tempgen::MIN_DTR_C;
        }
        let temp = 0.5 * (tmax + tmin);
        let dtr = tmax - tmin;

        // PET: Hargreaves-Samani with FAO-56 Ra (see module header). It
        // responds to the warmer, clearer (larger diurnal range) days of a
        // dry summer by itself.
        let photo = photoperiod_h(p.latitude_deg, doy);
        let ra = extraterrestrial_radiation_mj(p.latitude_deg, doy);
        let mut pet = hargreaves_pet_mm(temp, dtr, ra);
        if forced_drought {
            // Gameplay control, not weather: no rain and +40 % demand.
            precip = 0.0;
            pet *= 1.4;
        }
        Weather {
            temp_c: temp,
            tmin_c: tmin,
            tmax_c: tmax,
            precip_mm: precip,
            pet_mm: pet,
            photoperiod_h: photo,
        }
    }
}

/// Single-layer soil water bucket (mm of plant-extractable water).
#[derive(Clone, Debug)]
pub struct SoilWater {
    pub capacity_mm: f32,
    pub water_mm: f32,
}

impl SoilWater {
    pub fn new(capacity_mm: f32) -> Self {
        SoilWater { capacity_mm, water_mm: capacity_mm }
    }

    /// Relative extractable water 0..1.
    pub fn rew(&self) -> f32 {
        (self.water_mm / self.capacity_mm).clamp(0.0, 1.0)
    }

    /// Daily update. `demand_mm` is the potential canopy+soil evaporation.
    /// Actual ET declines linearly below REW 0.4: Granier, Breda, Biron &
    /// Villette 1999, "A lumped water balance model to evaluate duration and
    /// intensity of drought constraints in forest stands", Ecol Model
    /// 116:269-283 (BILJOU; transpiration reduced once relative extractable
    /// water < 0.4).
    /// Returns actual evapotranspiration (mm).
    pub fn step(&mut self, precip_mm: f32, demand_mm: f32) -> f32 {
        // Calibration: flat 10 % interception loss, a simplification (real
        // interception varies with canopy, season and rain intensity).
        self.water_mm += precip_mm * 0.9;
        let supply = (self.rew() / 0.4).min(1.0);
        let et = (demand_mm * supply).min(self.water_mm);
        self.water_mm = (self.water_mm - et).clamp(0.0, self.capacity_mm);
        et
    }

    /// Change capacity (e.g. as roots deepen) keeping relative water content.
    pub fn set_capacity(&mut self, capacity_mm: f32) {
        let rew = self.rew();
        self.capacity_mm = capacity_mm;
        self.water_mm = rew * capacity_mm;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn climate_is_plausible() {
        // Checked against the Rennes-St Jacques 1991-2020 normals (module
        // header): mean 12.4 degC, Jan 6.2, Jul 19.2, 691 mm on 115 days
        // >= 1 mm, Penman PET 856 mm (Jan 22 mm, Jul 137 mm; figures of an
        // earlier edition of the fiche, 836.9 mm for 2001-2020 in the 2025
        // edition). The PET range was 450-850 mm for the former ad hoc
        // index; it now encodes the station Penman total +-12 % (Hargreaves
        // is a different method).
        let mut c = Climate::new(ClimateParams::default(), 7);
        let (mut t, mut p, mut pet, mut wet1) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let (mut jan, mut jul, mut pet_jan, mut pet_jul, mut dtr_jan, mut dtr_jul) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let years = 30;
        for y in 0..years {
            for d in 0..365 {
                let w = c.day(y, d, false);
                t += w.temp_c as f64;
                p += w.precip_mm as f64;
                pet += w.pet_mm as f64;
                wet1 += (w.precip_mm >= 1.0) as u8 as f64;
                if d < 31 {
                    jan += w.temp_c as f64;
                    pet_jan += w.pet_mm as f64;
                    dtr_jan += (w.tmax_c - w.tmin_c) as f64;
                }
                if (181..212).contains(&d) {
                    jul += w.temp_c as f64;
                    pet_jul += w.pet_mm as f64;
                    dtr_jul += (w.tmax_c - w.tmin_c) as f64;
                }
            }
        }
        let n = (years * 365) as f64;
        let yf = years as f64;
        let t = t / n;
        let p = p / yf;
        let pet = pet / yf;
        let jan = jan / (31.0 * yf);
        let jul = jul / (31.0 * yf);
        let (pet_jan, pet_jul) = (pet_jan / yf, pet_jul / yf);
        let (dtr_jan, dtr_jul) = (dtr_jan / (31.0 * yf), dtr_jul / (31.0 * yf));
        let wet1 = wet1 / yf;
        println!("T {t:.2} jan {jan:.2} jul {jul:.2} P {p:.0} wet>=1 {wet1:.0} PET {pet:.0} jan {pet_jan:.1} jul {pet_jul:.1} dtr {dtr_jan:.1}/{dtr_jul:.1}");
        assert!((11.9..12.9).contains(&t), "mean T {t}");
        assert!((5.0..7.5).contains(&jan) && (18.0..20.5).contains(&jul), "jan {jan} jul {jul}");
        assert!((600.0..800.0).contains(&p), "precip {p}");
        assert!((95.0..140.0).contains(&wet1), "days >= 1 mm {wet1}");
        assert!((750.0..960.0).contains(&pet), "pet {pet}");
        assert!((8.0..35.0).contains(&pet_jan) && (115.0..165.0).contains(&pet_jul), "pet jan {pet_jan} jul {pet_jul}");
        assert!((5.0..7.0).contains(&dtr_jan) && (10.0..12.5).contains(&dtr_jul), "dtr {dtr_jan} {dtr_jul}");
        let ph_june = photoperiod_h(48.0, 171);
        let ph_dec = photoperiod_h(48.0, 354);
        assert!((15.8..16.5).contains(&ph_june), "{ph_june}");
        assert!((8.0..8.8).contains(&ph_dec), "{ph_dec}");
    }

    #[test]
    fn temperature_matches_rennes_monthly_normals_and_frost_days() {
        // Calibration checks (crate::tempgen): monthly Tn and Tx normals of
        // the Meteo-France fiche (1991-2020) and the yearly count of days
        // with Tn <= 0 degC (30.1), with the working tolerance of
        // docs/validation/physics.md (+-4 days). The tails left out of the
        // fit (Tn <= -5, Tx >= 30, records) are judged by the validation
        // bench, not here.
        let p = ClimateParams::default();
        let mut c = Climate::new(p.clone(), 7);
        let years = 300u32;
        let (mut tn, mut tx, mut n, mut frost) = ([0.0f64; 12], [0.0f64; 12], [0.0f64; 12], 0.0f64);
        for y in 0..years {
            for d in 0..365 {
                let w = c.day(y, d, false);
                let m = tempgen::month_of(d);
                tn[m] += w.tmin_c as f64;
                tx[m] += w.tmax_c as f64;
                n[m] += 1.0;
                frost += (w.tmin_c <= 0.0) as u8 as f64;
                assert!(w.tmax_c - w.tmin_c >= tempgen::MIN_DTR_C - 1e-4 && (w.temp_c - 0.5 * (w.tmax_c + w.tmin_c)).abs() < 1e-4);
            }
        }
        for m in 0..12 {
            let (a, b) = (tn[m] / n[m], tx[m] / n[m]);
            assert!((a - p.tn_normals_c[m] as f64).abs() < 0.3, "month {m}: Tn {a:.2} vs {}", p.tn_normals_c[m]);
            assert!((b - p.tx_normals_c[m] as f64).abs() < 0.3, "month {m}: Tx {b:.2} vs {}", p.tx_normals_c[m]);
        }
        let frost = frost / years as f64;
        assert!((26.1..34.1).contains(&frost), "frost days {frost:.1}");
        // A warmer site shifts every month uniformly.
        let warm = ClimateParams { mean_temp_c: p.mean_temp_c + 3.0, ..p.clone() };
        let (mut a, mut b) = (Climate::new(p, 3), Climate::new(warm, 3));
        for d in 0..365 {
            let (wa, wb) = (a.day(0, d, false), b.day(0, d, false));
            assert!((wb.temp_c - wa.temp_c - 3.0).abs() < 1e-3);
        }
    }

}
