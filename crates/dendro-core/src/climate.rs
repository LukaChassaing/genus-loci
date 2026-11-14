//! Procedural temperate-oceanic climate and a soil water bucket.
//!
//! Stylised weather generator for western Europe (~48 degN, e.g. Rennes /
//! Nantes): annual mean ~12 degC, January ~5 degC, July ~19.5 degC,
//! ~750 mm precipitation on ~165 wet days, wetter winters. Daily temperature
//! is a seasonal cosine plus an AR(1) anomaly; each year draws seeded
//! anomalies (warm/cold year, cold spring, dry summer).

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
}

impl Default for ClimateParams {
    fn default() -> Self {
        ClimateParams {
            latitude_deg: 48.0,
            mean_temp_c: 12.0,
            temp_amplitude_c: 7.2,
            coldest_doy: 15.0,
            daily_temp_sd: 2.6,
            daily_temp_autocorr: 0.72,
            annual_temp_sd: 0.6,
            annual_precip_mm: 750.0,
            wet_day_prob: 0.45,
            dry_year_prob: 0.12,
            cold_spring_prob: 0.15,
        }
    }
}

/// One day of weather.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Weather {
    /// Daily mean air temperature (degC).
    pub temp_c: f32,
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
pub fn photoperiod_h(latitude_deg: f32, doy: u32) -> f32 {
    let decl = 23.44_f32.to_radians() * (TAU * (284.0 + doy as f32 + 1.0) / 365.0).sin();
    let lat = latitude_deg.to_radians();
    let c = ((-0.833_f32).to_radians().sin() - lat.sin() * decl.sin()) / (lat.cos() * decl.cos());
    let h = c.clamp(-1.0, 1.0).acos();
    24.0 * h / std::f32::consts::PI
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
        let seasonal = p.mean_temp_c - p.temp_amplitude_c * (TAU * (d - p.coldest_doy) / 365.0).cos();
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

        // Precipitation: wetter winters (oceanic regime).
        let season = (TAU * (d - 15.0) / 365.0).cos();
        let mut p_wet = p.wet_day_prob * (1.0 + 0.22 * season);
        let mean_amount = p.annual_precip_mm / (365.0 * p.wet_day_prob) * (1.0 + 0.12 * season);
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

        // PET: simple temperature x day-length index (Thornthwaite-like),
        // ~3.5 mm/d in July, <1 mm/d in winter, ~650 mm/yr.
        let photo = photoperiod_h(p.latitude_deg, doy);
        let mut pet = (0.25 + 0.135 * temp.max(0.0) * photo / 12.0).max(0.1);
        if a.dry_summer {
            pet *= 1.0 + 0.15 * summer;
        }
        if forced_drought {
            precip = 0.0;
            pet *= 1.4;
        }
        Weather { temp_c: temp, precip_mm: precip, pet_mm: pet, photoperiod_h: photo }
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
    /// Actual ET declines linearly below REW 0.4 (Granier et al. 1999).
    /// Returns actual evapotranspiration (mm).
    pub fn step(&mut self, precip_mm: f32, demand_mm: f32) -> f32 {
        self.water_mm += precip_mm * 0.9; // ~10% interception loss
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
        let mut c = Climate::new(ClimateParams::default(), 7);
        let (mut t, mut p, mut pet) = (0.0f64, 0.0f64, 0.0f64);
        let mut jan = 0.0;
        let mut jul = 0.0;
        let years = 30;
        for y in 0..years {
            for d in 0..365 {
                let w = c.day(y, d, false);
                t += w.temp_c as f64;
                p += w.precip_mm as f64;
                pet += w.pet_mm as f64;
                if d < 31 {
                    jan += w.temp_c as f64;
                }
                if (181..212).contains(&d) {
                    jul += w.temp_c as f64;
                }
            }
        }
        let n = (years * 365) as f64;
        let t = t / n;
        let p = p / years as f64;
        let pet = pet / years as f64;
        let jan = jan / (31.0 * years as f64);
        let jul = jul / (31.0 * years as f64);
        assert!((10.5..13.5).contains(&t), "mean T {t}");
        assert!((550.0..950.0).contains(&p), "precip {p}");
        assert!((450.0..850.0).contains(&pet), "pet {pet}");
        assert!(jan < 7.5 && jul > 17.0, "jan {jan} jul {jul}");
        let ph_june = photoperiod_h(48.0, 171);
        let ph_dec = photoperiod_h(48.0, 354);
        assert!((15.8..16.5).contains(&ph_june), "{ph_june}");
        assert!((8.0..8.8).contains(&ph_dec), "{ph_dec}");
    }
}
