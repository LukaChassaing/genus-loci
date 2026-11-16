//! Procedural temperate-oceanic climate and a soil water bucket.
//!
//! Stylised weather generator for western Europe, anchored on the Meteo-France
//! 1991-2020 normals of Rennes-St Jacques (48.07 degN, station 35281001,
//! "Fiche climatologique, statistiques 1991-2020"): annual mean 12.4 degC,
//! January 6.2 degC, July/August 19.2-19.3 degC, mean daily range (Tx - Tn)
//! from 5.9 degC in January to 11.1 degC in July, 691 mm of precipitation
//! (115 days >= 1 mm), wetter autumn-winter, Penman PET 856 mm/yr. Daily
//! temperature is a seasonal cosine plus an AR(1) anomaly; each year draws
//! seeded anomalies (warm/cold year, cold spring, dry summer).
//!
//! Calibration: the seasonal cosines are fitted by eye to those monthly
//! normals (not a statistical fit). The AR(1) daily anomaly (rho 0.72, sd
//! 2.6 degC), the smaller diurnal range on wet days and the anomaly
//! probabilities are stylised weather-generator settings (no source); the
//! unit test checks the generated normals against the station values.
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
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// Climate parameters (defaults: temperate oceanic, 48 degN).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClimateParams {
    pub latitude_deg: f32,
    /// Annual mean temperature (degC).
    pub mean_temp_c: f32,
    /// Half the difference between warmest and coldest month means (degC).
    pub temp_amplitude_c: f32,
    /// Day of year (0-based) of the temperature minimum.
    pub coldest_doy: f32,
    /// Standard deviation of day-to-day anomaly (degC).
    pub daily_temp_sd: f32,
    /// Lag-1 autocorrelation of daily anomalies.
    pub daily_temp_autocorr: f32,
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
    /// Annual mean of the daily temperature range Tmax - Tmin (degC).
    #[serde(default = "dtr_mean_default")]
    pub dtr_mean_c: f32,
    /// Half the seasonal swing of the daily range (smallest around Jan 1,
    /// largest around Jul 1; degC).
    #[serde(default = "dtr_amplitude_default")]
    pub dtr_amplitude_c: f32,
}

fn dtr_mean_default() -> f32 {
    8.8
}

fn dtr_amplitude_default() -> f32 {
    2.6
}

impl Default for ClimateParams {
    /// Rennes-St Jacques 1991-2020 (see the module header).
    fn default() -> Self {
        ClimateParams {
            latitude_deg: 48.07,
            mean_temp_c: 12.4,
            temp_amplitude_c: 6.6,
            coldest_doy: 20.0,
            daily_temp_sd: 2.6,
            daily_temp_autocorr: 0.72,
            annual_temp_sd: 0.6,
            // Calibration: 760 gives the station's ~690 mm once dry years
            // and the generator's other terms are applied (tested below).
            annual_precip_mm: 760.0,
            wet_day_prob: 0.42,
            dry_year_prob: 0.12,
            cold_spring_prob: 0.15,
            dtr_mean_c: dtr_mean_default(),
            dtr_amplitude_c: dtr_amplitude_default(),
        }
    }
}

/// One day of weather.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Weather {
    /// Daily mean air temperature (degC).
    pub temp_c: f32,
    /// Daily minimum / maximum air temperature (degC): mean -/+ half the
    /// generated diurnal range.
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
    ar: f32,
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

/// Smooth bump: 1 in [a+r, b-r], 0 outside [a, b], linear ramps.
fn window(doy: f32, a: f32, b: f32, r: f32) -> f32 {
    ((doy - a) / r).clamp(0.0, 1.0) * ((b - doy) / r).clamp(0.0, 1.0)
}

impl Climate {
    pub fn new(params: ClimateParams, seed: u64) -> Self {
        let mut c = Climate { params, seed, ar: 0.0, year: u32::MAX, anomaly: YearAnomaly::default() };
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

        // Temperature.
        let seasonal = p.mean_temp_c - p.temp_amplitude_c * (TAU * (d - p.coldest_doy) / 365.0).dcos();
        let rho = p.daily_temp_autocorr;
        let eps = rng::normal(rng::hash(s, S_TEMP, year as u64, doy as u64));
        self.ar = rho * self.ar + (1.0 - rho * rho).sqrt() * p.daily_temp_sd * eps;
        let mut temp = seasonal + a.temp_offset + self.ar;
        if a.cold_spring {
            temp -= 2.5 * window(d, 55.0, 150.0, 20.0);
        }
        let summer = window(d, 125.0, 265.0, 25.0);
        if a.dry_summer {
            temp += 1.3 * summer;
        }

        // Precipitation: wetter autumn-winter (oceanic regime; Rennes has
        // ~73 mm/month in Oct-Dec against ~44 mm in Jul-Aug). Calibration:
        // peak in mid-December, +-18 % wet-day frequency and +-10 % amount.
        let season = (TAU * (d - 350.0) / 365.0).dcos();
        let mut p_wet = p.wet_day_prob * (1.0 + 0.18 * season);
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

        // Diurnal temperature range: seasonal cosine fitted to the Rennes
        // normals (5.9 degC in January, 11.1 degC in July). Cloudy wet days
        // have a smaller range than clear dry days. Calibration: wet days
        // get 0.75 x the seasonal range and dry days are raised so that the
        // seasonal mean is kept (no source for the 0.75).
        let dtr_season = p.dtr_mean_c - p.dtr_amplitude_c * (TAU * d / 365.0).dcos();
        let pw = p_wet.clamp(0.0, 0.9);
        let dtr = if wet { 0.75 * dtr_season } else { dtr_season * (1.0 - 0.75 * pw) / (1.0 - pw) };
        let dtr = dtr.max(1.0);

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
            tmin_c: temp - 0.5 * dtr,
            tmax_c: temp + 0.5 * dtr,
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
        // >= 1 mm, Penman PET 856 mm (Jan 22 mm, Jul 137 mm). The PET range
        // was 450-850 mm for the former ad hoc index; it now encodes the
        // station Penman total +-12 % (Hargreaves is a different method).
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
}
