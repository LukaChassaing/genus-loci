# Jeux de référence pour la validation du moteur

Chaque fichier `data/validation/<sujet>.ron` liste des valeurs publiées, pour les confronter
aux sorties du moteur (`crates/dendro-core`). Le banc de validation lit ces fichiers et
compare chaque observable à ce que produit la simulation.

```ron
(
    subject: "quercus_robur",          // espèce (nom du génome) ou "physics" pour le générique
    entries: [
        (
            id: "oak_height_100y_good_site",      // unique dans le fichier, snake_case
            quantity: "height_m",                  // observable, voir la liste ci-dessous
            conditions: (age_years: Some(100.0), site: Some("open-grown, fertile lowland, W Europe"), extra: None),
            value: Some(30.0),                     // valeur centrale publiée (None si seule une plage existe)
            min: Some(25.0),                       // plage publiée ou dispersion (écart-type, quantiles…)
            max: Some(35.0),
            unit: "m",
            spread_kind: "range",                  // "range" | "sd" | "ci95" | "quantiles" | "single" | "max_record"
            source: "White J. 1998. Estimating the age of large and veteran trees in Britain. Forestry Commission Information Note 12.",
            url: Some("https://…"),
            accessed: "2026-11-17",
            source_kind: "primary",                // "primary" (article, flore, inventaire) | "review" | "handbook" | "wikipedia" | "secondary"
            read: "full",                          // "full" | "abstract" | "table" | "via_wikipedia" | "snippet"
            language: "en",                        // langue de la source consultée
            quote: Some("…courte citation < 15 mots…"),
            note: Some("tout ce qui conditionne la comparaison"),
        ),
    ],
)
```

Règles :

- Aucune valeur inventée, aucune référence fabriquée. Si un chiffre vient de Wikipédia, `source_kind: "wikipedia"`
  et on suit autant que possible la note vers la source primaire (une entrée par source réellement consultée).
- Quand deux sources divergent, on garde les deux entrées : la divergence est une information.
- Les conditions (âge, station, peuplement serré ou arbre isolé, climat) sont indispensables :
  une hauteur sans âge ni station ne valide rien.
- Observables reconnus (étendre au besoin, en le signalant) : `height_m`, `dbh_cm`, `crown_width_m`,
  `crown_ratio`, `ring_width_mm`, `root_depth_m`, `root_spread_m`, `root_shoot_ratio`, `budburst_doy`,
  `flowering_doy`, `leaf_fall_doy`, `fruit_maturity_doy`, `lai`, `lma_g_m2`, `transpiration_mm_yr`,
  `transpiration_l_day`, `wue`, `green_density_kg_m3`, `dry_density_kg_m3`, `mor_mpa`, `moe_gpa`,
  `aboveground_biomass_t`, `natural_frequency_hz`, `damping_ratio`, `critical_wind_ms`, `drag_coefficient`,
  `anchorage_moment_knm`, `longevity_years`, `age_first_flowering_years`, `shade_tolerance`,
  `drought_tolerance`, `waterlogging_tolerance`, `pipe_area_ratio`, `branch_stress_mpa`,
  `wound_closure_mm_yr`, `epicormic_response`, `mycorrhiza_type`.

## Observables ajoutés par les jeux de référence (2026-11-17)

Ajoutés par les recherches documentaires ; ils sont reconnus par le banc (`crates/dendro-validate`), qui les relie
ou les déclare non modélisés dans `docs/validation/report.md`.

- Climat et station (`physics.ron`) : `temp_mean_c`, `temp_max_mean_c`, `temp_min_mean_c`, `diurnal_range_c`,
  `frost_days`, `frost_days_tn_le_m5`, `frost_days_tn_le_m10`, `frost_days_tx_le_0`, `hot_days_tx_ge_25`,
  `hot_days_tx_ge_30`, `record_high_c`, `record_low_c`, `sunshine_h_yr`, `global_radiation_mj_m2_yr`, `etp_mm_yr`,
  `rain_mm_yr`, `rain_mm_month`, `rain_mm_day_record`, `rain_days_ge_1mm`, `rain_days_ge_5mm`, `rain_days_ge_10mm`,
  `snow_days`, `wind_mean_ms`, `gust_days_ge_16ms`, `gust_days_ge_28ms`, `gust_record_ms`.
- Mécanique, tempêtes, thigmomorphogenèse (`physics.ron`) : `f0_vs_d_over_h2`, `f0_debranched_vs_intact`,
  `f0_winter_gain_pct`, `pruning_crown_mass_threshold_pct`, `anchorage_moment_gain_deep_roots_pct`,
  `tilt_at_critical_moment_deg`, `failure_mode`, `leonardo_rule`, `storm_damage_plot_fraction`,
  `storm_damaged_forest_fraction`, `storm_damage_predictors`, `windthrown_volume_m3`, `forest_destroyed_fraction`,
  `leaf_on_damage_effect`, `thigmomorphogenesis_effect`, `radial_growth_mechanical_control`,
  `stem_vs_anchorage_strengthening`, `bending_dose_response`.
- Racines, eau, phénologie, houppier (`physics.ron`) : `root_beta`, `root_fraction_top_10cm`, `root_fraction_top_30cm`,
  `root_fraction_top_40cm`, `rew_stress_threshold`, `budburst_model_form`, `budburst_rmse_d`,
  `crown_diameter_vs_dbh_linear`, `crown_to_stem_ratio`.
- Chêne (`quercus_robur.ron`) : `frost_kill_new_foliage_c`, `frost_kill_acorn_c`, `shoot_flushes_per_year`,
  `mast_interval_years`, `acorn_mass_g`, `leaf_thickness_um`, `basal_area_m2_ha`, `stem_density_per_ha`, `girth_cm`,
  `transpiration_mm_day`, `transpiration_l_day_per_m2_canopy`, `yield_class_m3_ha_yr`, `rotation_years`,
  `storm_damage_fraction`, `dormant_bud_longevity_years`.
- Merisier (`prunus_avium.ron`) : `dbh_increment_mm_yr`, `slenderness_hd`, `mai_m3_ha_yr`, `heart_rot_onset_years`,
  `heart_rot_fraction`, `flowering_duration_days`, `chilling_hours`, `compressive_strength_mpa`,
  `tensile_strength_mpa`, `shrinkage_tangential_pct`, `sapwood_width_cm`, `sapwood_diameter_fraction`,
  `sapwood_rings`, `bark_thickness_mm`, `bark_area_fraction`, `stem_volume_m3`, `stand_volume_m3_ha`,
  `min_soil_depth_cm`, `sucker_distance_m`, `root_suckering_relative`, `root_architecture`, `soil_ph_range`,
  `frost_tolerance`, `max_altitude_m`, `pruning_wound_rot_fraction`, `max_occluded_branch_diameter_mm_4y`,
  `pruning_max_branch_diameter_mm`, `pruning_season`, `dead_branch_persistence`, `gummosis`, `stump_sprouting`,
  `shoot_length_cm`, `ring_width_trend`.

## Conventions constatées et vérifiées par le banc

- `spread_kind: "sd"` : `min` et `max` valent valeur ± 1 écart-type (le banc en tire la tolérance ± 2 sd). Quand la
  source donne une erreur-type (Niinemets & Valladares, Cole & Sheldon), la note le dit : le banc applique quand même
  la règle `sd` et l'affiche.
- `max_record` : le record est dans `value`, dans `max` ou, pour une classe « plus de… », dans `min`.
- Entrées d'absence : id commençant par `absence_`, `value`, `min`, `max` à `None` ; ignorées par les verdicts.
- Catégories qualitatives : `value: None`, la catégorie dans `conditions.extra` ou dans `note`.
- Ids : le banc accepte `[A-Za-z0-9_]` (trois ids `cienciala_agb_D30/D40/D50` portent une majuscule ; ils n'ont pas
  été renommés pour ne pas toucher aux fichiers de référence).
- Unités : la liste fermée des unités en usage est dans `crates/dendro-validate/src/data.rs` (`KNOWN_UNITS`) ; une
  unité nouvelle doit y être ajoutée, ce qui attrape les fautes de frappe (test `reference_files_load_and_are_consistent`).

Vérification : `cargo test -p dendro-validate` ; rapport complet :
`cargo run --release -p dendro-validate -- --seeds 5 --out docs/validation/report.md`.
