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
