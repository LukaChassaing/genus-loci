# Ajouter une espèce à Dendrologia

Ce guide s'adresse à qui écrit un nouveau génome **sans toucher au code**. Une espèce = **un fichier** `data/species/<nom_latin>.ron`. Rien d'autre à modifier : le fichier est découvert à la compilation (`crates/dendro-core/build.rs`), embarqué dans le binaire (déterminisme, export Godot), placé dans le menu « Espèce » sous sa catégorie, lisible dans les sauvegardes (par son nom latin), parcouru par l'exemple `state_hash` et par le banc de validation dès que `data/validation/<nom_latin>.ron` existe.

Les deux pilotes sont les modèles à copier :

| Votre espèce | Partir de | Pourquoi |
|---|---|---|
| Feuillu de forêt caduc | `quercus_robur.ron` (longévif, décurrent, poreux) ou `prunus_avium.ron` (pionnier, excurrent, diffus) | traits les plus commentés |
| Fruitier (franc de pied) | `malus_domestica.ron` | fruits charnus, alternance, rameaux courts |
| Résineux persistant | `picea_abies.ron` | aiguilles, cohortes, verticilles, cônes, masting |
| Résineux caduc (*Larix*) | `picea_abies.ron` avec `deciduous: true` | aiguilles mais chute annuelle complète |

## 1. Règles de crédibilité (non négociables)

- **Chaque valeur a un statut en commentaire** : `measured` (valeur publiée lue, avec l'identifiant de l'entrée entre crochets, ex. `[wfr_needle_longevity]`), `proxy` (autre espèce, dites laquelle), `estimated` (connaissance générale, aucune source lue), `calibrated` (réglée pour que l'arbre simulé corresponde aux références). N'inventez **ni chiffre ni référence**. Si la fiche `docs/species/<id>.md` dit « non trouvé », écrivez `estimated` et signalez-le.
- Les références chiffrées vivent dans `data/validation/<id>.ron` (format : `data/validation/FORMAT.md`) ; la fiche de traits dans `docs/species/<id>.md`. Le génome cite les identifiants de ces entrées plutôt que de recopier les sources.
- Ajoutez à `docs/sources.md` toute source nouvelle utilisée par le génome, et tout écart notable dans « Écarts connus ».
- **Pas de calage sur ses propres résultats** : les bandes des tests viennent des références ; une convention du banc (par exemple « 0,7 × la hauteur de peuplement ») doit être écrite comme telle.

## 2. Le fichier

- Nom : binôme latin en minuscules, espaces → `_` : `Fagus sylvatica` → `data/species/fagus_sylvatica.ron`. Le test `species_registry` vérifie que le nom du fichier correspond au champ `species`.
- En-tête : `species` (binôme latin), `common_name` (anglais), `common_name_fr` (libellé du menu), `category` : `Broadleaf` (feuillu), `FruitTree` (fruitier) ou `Conifer` (résineux). L'ordre du menu est catégorie puis nom français ; **les indices d'espèce changent quand on ajoute un fichier** : on ne stocke jamais un indice, toujours l'id ou le nom latin.
- Syntaxe RON : les énumérations s'écrivent sans guillemets (`leaf_type: Needle`, `mycorrhiza: Ecto`, `fruit_kind: ConePendent`), les couleurs comme des triplets `(r, g, b)` de 0 à 1, `Some(0.55)` / `None` pour les options. Un champ absent prend sa valeur par défaut (indiquée ci-dessous) ; préférez écrire chaque champ explicitement.

## 3. Les traits, un par un

Unités et sens. « Pilotes » donne les valeurs des quatre génomes existants (chêne / merisier / pommier / épicéa) comme ordre de grandeur, **pas** comme plage publiée : la plage d'une espèce vient de ses références.

### `architecture` (forme de l'arbre ; `growth.rs`, modèle de bourgeons de Palubicki et al. 2009)

| Champ | Unité | Effet | Pilotes |
|---|---|---|---|
| `apical_dominance_young`, `apical_dominance_mature` | 0-1 (0,5 neutre) | part de la vigueur laissée à l'axe principal (tige), jeune puis adulte. > 0,6 : tige unique (excurrent) ; < 0,5 : la tige se dissout en charpentières (décurrent) | 0,75→0,42 / 0,70→0,51 / 0,62→0,42 / 0,80→0,72 |
| `juvenile_phase_years`, `maturity_age_years` | ans | la valeur jeune tient jusqu'au premier âge, puis passe en douceur à la valeur adulte au second | |
| `apical_dominance_branches` | 0-1 | idem sur les branches, constant | 0,5-0,62 |
| `lateral_competition` | exposant (1 = proportionnel) | > 1 : quelques branches bien éclairées dominent (charpentières) | 1,4-2,0 |
| `vigour_per_light` | métamères par unité de lumière | vigueur globale : **premier levier de la hauteur** | 6-7,5 |
| `bud_break_threshold` | métamères | seuil de débourrement d'un bourgeon | 1,0 |
| `internode_length_m`, `max_shoot_length_m` | m | longueur d'un métamère ; plafond de la pousse annuelle (la flèche n'ira jamais au-delà ; comparez à la pousse annuelle maximale publiée) | 0,03-0,05 ; 0,55-0,85 |
| `lateral_buds_per_shoot`, `lateral_buds_per_metamer` | nombre ; par métamère | bourgeons latéraux par pousse (le plafond = nombre de branches d'un verticille chez un résineux) | 5 |
| `bud_lifetime_years` | ans | survie d'un bourgeon dormant | 2-4 |
| `whorled` | booléen (défaut `false`) | latérales réparties régulièrement autour de la pousse (verticilles annuels : *Pinus*, *Picea*, *Abies*) au lieu de la phyllotaxie | épicéa `true` |
| `lateral_planarity` | 0-1 (défaut 0) | latérales des branches ramenées dans le plan de la branche (rameaux plagiotropes en éventail : *Picea*, *Abies*, *Fagus*, *Carpinus*) | épicéa 0,55 |
| `branching_angle_deg`, `phyllotaxis_deg` | degrés | angle d'insertion ; divergence (144 = 2/5, 137,5 = spirale) | 42-72 ; 137,5-144 |
| `max_order` | ordre | ordre de ramification maximal (laisser grand sauf espèce peu ramifiée) | 8-120 |
| `gravitropism`, `lateral_gravitropism` | poids | redressement de la tige ; correction des branches vers leur angle de consigne | 0,45-1,0 ; 0,3-0,4 |
| `limb_setpoint_low_deg`, `limb_setpoint_high_deg`, `twig_setpoint_deg` | degrés depuis la verticale | angle visé par les charpentières basses / hautes, et les rameaux (> 90 : retombant) | épicéa 100 / 60 / 95 |
| `crown_spread_ratio` | largeur / hauteur | au-delà de ~60 % de ce rayon les pointes se relèvent : **levier de la largeur du houppier** | 0,45 (cône) à 1,7 (pommier) |
| `phototropism`, `sinuosity_deg` | poids ; degrés | attirance vers la lumière ; zigzag aléatoire | |
| `leader_loss_probability`, `leader_loss_angle_deg` | /an ; degrés | perte du bourgeon terminal relayé par un latéral (coudes du chêne, fourches) | 0,02-0,15 |
| `sag_*`, `straightening_*` | voir `genome.rs` | affaissement stylisé des branches lourdes ; redressement autotrope | copier un pilote |
| `shedding_threshold`, `shedding_mean_threshold`, `shade_survival_years` | lumière 0-1 ; ans | mort des branches à l'ombre (élagage naturel) ; bas = tolérant | 0,04 (épicéa) à 0,10 (pommier) |
| `clear_bole_fraction`, `clear_bole_max_m`, `clear_bole_limb_ratio` | fraction de hauteur ; m ; ratio | élagage stylisé du bas de la tige | un résineux isolé garde ses branches basses : petites valeurs |
| `retrenchment_*`, `epicormic_rate` | âge, probabilités | descente de cime des vieux arbres, réitération | 0 = jamais |
| `hydraulic_path_max_m`, `hydraulic_exponent` | m ; exposant | distance collet-bourgeon où l'allongement s'arrête (Ryan & Yoder 1997) : **plafond de hauteur** ; l'exposant règle la forme de la courbe hauteur-âge | 14 (pommier) à 40 (épicéa) |
| `hydraulic_foliage_exponent` | exposant | baisse d'efficacité du feuillage avec la distance (pionniers : élevé) | 0,65-1,5 |

### `leaf` (feuillage)

| Champ | Unité | Effet |
|---|---|---|
| `length_cm`, `width_cm`, `lobes`, `serration` | cm ; nombre ; 0-1 | forme dessinée (texture) ; pour une aiguille, `length_cm` = longueur de l'aiguille |
| `leaf_type` | `Broad` (défaut) / `Needle` / `Scale` | texture : limbes ou rameaux d'aiguilles (`Scale` est dessiné comme des aiguilles très courtes) |
| `deciduous` | booléen | `false` = persistant : exige `leaf_longevity_years` > 1 |
| `leaf_longevity_years` | ans (défaut 1) | **persistants** : durée de vie moyenne d'une cohorte d'aiguilles. Chaque pousse annuelle garde son feuillage ~L ans, la plus vieille cohorte tombe en automne. Agit sur la lumière, le bois (Pressler), le poids et la traînée, le rendu ; en hiver il reste (L-1)/L du feuillage (photosynthèse hivernale quand T > 4 °C). Épicéa 4-7, sapin ~8-10, pins 2-4, if 4-8 (à sourcer pour chaque espèce) |
| `lma_g_m2`, `crown_lai` | g/m² ; m²/m² (défauts 75 et 4) | masse surfacique et indice foliaire du houppier : masse des feuilles (mécanique), carbone (vue des flux). **Même base de surface pour les deux** (projetée ou demi-surface totale : une aiguille a 2,2 à 4 fois plus de surface totale que projetée) |
| `marcescence`, `marcescence_max_age_years` | 0-1 ; ans | feuilles mortes gardées l'hiver par les jeunes arbres (chêne, hêtre, charme) |
| `leaves_per_shoot`, `cluster_size_leaves`, `foliage_density`, `tip_bunching`, `petiole_ratio`, `leaf_droop` | rendu | carte = `cluster_size_leaves` longueurs de feuille ; densité des cartes. Pour les aiguilles : `tip_bunching` ≥ 0,5 = aiguilles en touffes au bout des pousses (pins, mélèze), `leaf_droop` = angle des aiguilles (0 vers l'avant, 1 à angle droit) |
| `spur_foliage`, `spur_max_radius_mm`, `spur_min_light` | rendu | rameaux courts (dards, bourses) sur le bois de 2 ans et plus |
| `colour_spread` | 0-1 | étalement du début de la coloration entre bouquets |

### `pigments` (couleurs, réglées à l'œil, voir `phenology::leaf_colour`)

`chlorophyll`, `carotenoids` (jaune), `anthocyanins` (rouge), `tannins` (brun), `anthocyanin_persistence`, `anthocyanin_color`. Statut toujours `calibrated` (aucune mesure de spectre).

### `phenology` (Cannell & Smith 1983 pour le débourrement, Delpierre et al. 2009 pour la sénescence)

| Champ | Unité | Effet |
|---|---|---|
| `gdd_budburst`, `chill_forcing_gdd`, `chilling_requirement_days` | °C·jour au-dessus de `gdd_base_c` depuis le 1er février ; jours ≤ 5 °C depuis le 1er novembre | date de débourrement `T = a + b exp(-3C/req)`. **Levier : `gdd_budburst`** (épicéa, mesuré : 330 → 420 °C·j a retardé le débourrement moyen de 15 à 20 jours à Rennes) |
| `leaf_expansion_gdd` | °C·jour | durée du débourrement à la feuille adulte (et de l'allongement des pousses) |
| `senescence_photoperiod_h`, `senescence_temp_base_c`, `senescence_cdd`, `leaf_fall_days` | h ; °C ; °C·jour ; jours | date et durée de la coloration et de la chute ; chez un persistant, ces mêmes traits datent la chute de la plus vieille cohorte et la fin du cerne |

### `wood`

| Champ | Unité | Effet |
|---|---|---|
| `density_kg_m3` | kg/m³ (densité basale) | charge du modèle d'affaissement ; lu par le banc |
| `elasticity_gpa` | GPa | affaissement |
| `ring_porous`, `earlywood_share`, `ring_contrast` | — | cernes (zone poreuse, part de bois initial, contraste dessiné ; résineux : non poreux, contraste net) |
| `growth_coefficient` | m³ de bois par m² de feuillage pondéré et par an | **levier du diamètre** (DBH) : à caler sur le DBH-âge publié |
| `twig_radius_mm`, `max_ring_width_mm` | mm | rayon d'une pousse ; plafond physiologique du cerne |
| `pipe_exponent`, `limb_draw_boost`, `bark_irregularity`, `collar_flare`, `root_flare`, `root_flare_height_radii`, `buttress_lobes` | rendu | dessin seulement (le DBH est pris sur la tige simulée) |
| `adaptive_stress_mpa`, `adaptive_min_diameter_cm`, `adaptive_max_budget_share` | MPa ; cm ; part | croissance radiale sous contrainte (11,1 MPa = moyenne inter-espèces d'Evans et al. 2008) : copier sauf source |
| `snag_years_per_m` | ans par m de rayon | persistance du bois mort (bois durable : grand) |

### `bark` (rendu) : `color`, `young_color`, `old_darkening`, `lenticels`, `fissures`, `gloss`.

### `roots` (`roots.rs`, en partie stylisé)

`taproot_dominance`, `taproot_decline_years` (pivot), `max_depth_m` (comparer aux profondeurs publiées), `lateral_spread` (× rayon du houppier), `lateral_count`, `lateral_depth_m`, `sinker_tendency`, `branching_probability`, `max_order`, `annual_extension_m`, `max_penetration_mpa`, `waterlogging_tolerance` (0-1 : entrée dans le sol saturé ; < 0,3 déclenche au banc la règle « croissance sur argile à gley »), `hypoxia_sensitivity`, `hypoxia_air_porosity`, `drought_deepening`, `hydrotropism`, `root_shoot_ratio`, `root_shoot_stress_gain`, `fine_root_per_m`, `rapid_taper`.

### `tolerances`

`frost_c` (°C qui tue les jeunes feuilles ; **pas** l'endurcissement hivernal du bois, non modélisé), `drought` (0-1), `shade` (0-1 adulte), `shade_juvenile` (`Some(x)` si le jeune est plus tolérant), `shade_juvenile_years`, `shade_mature_years`.

### `reproduction`

| Champ | Unité | Effet |
|---|---|---|
| `first_flowering_age_years` | ans (± 1 an par arbre) | fin de la phase juvénile |
| `flower_display`, `fruit_display` | 0-1 | 0 = non dessiné (chatons du chêne) |
| `bloom_gdd`, `bloom_duration_gdd` | °C·jour | date et durée de floraison (même pénalité de froid que le débourrement) |
| `min_wood_age_years`, `min_light`, `max_wood_radius_mm` | ans ; 0-1 ; mm | où sont les fleurs (rameaux courts, haut du houppier pour les cônes) |
| `flower_density`, `flower_cluster_cm`, `fruit_cluster_cm` | rendu | taille d'une carte de fleurs / de fruits (cône de 13 cm, pomme 6,5 cm) |
| `flower_color`, `fruit_color_unripe`, `fruit_color_ripe` | RGB | |
| `fruit_set` | 0-1 | part des bouquets qui nouent |
| `flower_frost_c` | °C | gel des fleurs ouvertes (le banc lit ce trait pour les entrées « blossom frost ») |
| `fruit_ripening_days`, `fruit_persistence_days` | jours | maturation après la chute des pétales ; durée sur l'arbre (le cycle annuel efface tout le 31 décembre) |
| `fruit_kind` | `Fleshy` (défaut) / `Nut` / `Samara` / `ConePendent` / `ConeErect` / `Aril` | texture du fruit (cône, samare ; `Nut` et `Aril` sont encore dessinés comme des fruits ronds) ; un `Fleshy` de 5 cm et plus est dessiné comme un gros fruit unique (pomme, poire) |
| `fruit_mass_g` | g | informatif (lu par le banc), sans coût carboné |
| `crop_interval_years`, `off_year_crop` | ans (défaut 1) ; 0-1 (défaut 0,15) | **masting / alternance** : année de pleine récolte avec une fréquence ~1/intervalle, jamais deux de suite ; les autres années fleurissent à `off_year_crop`. Pommier 2 et 0,5 (alternance 2:1), épicéa 3,5 et 0,1 |

### `longevity`, `symbiosis`, `pruning`, `mechanics`

- `longevity` :
  - `maintenance_per_m` (1/m, `#[serde(default)]`, défaut 0 = pas de bilan carboné, l'arbre ne meurt jamais de vieillesse) : coût d'entretien du bois vivant qui alimente le feuillage, par mètre de distance collet-feuille, rapporté à la production de ce feuillage (`vitality.rs`). Le rapport coût / production `ρ = maintenance_per_m × Σ(feuille × distance) / Σ(feuille × lumière × efficacité hydraulique)` monte avec la taille et la portée hydraulique ; tant qu'il reste sous ~0,7 l'arbre est intact (réserves pleines) ; au-delà les réserves baissent, la croissance ralentit, les branches au bilan négatif meurent (descente de cime) et la probabilité annuelle de mort monte. **La longévité en émerge** : il n'y a pas d'âge de mort. **Toujours `calibrated`** (aucune valeur mesurée n'existe) : réglez-le avec `cargo run -p dendro-core --release --example ageing -- --species <id> --cost <x> --seed <n> --quiet --stop` sur les graines 1-5 pour que l'âge médian de mort du tronc tombe dans la longévité publiée de `data/validation/<id>.ron`. Ordre de grandeur : `0,85 / (D/F à l'âge visé)`, la colonne D/F de l'exemple (lancé avec un coût minuscule, 1e-5) ; la réponse est raide et non monotone (de 0,0021 à 0,0026 l'épicéa passe de > 350 ans à ~45 ans) : petits pas, 5 graines. Espèce longévive (> 400 ans) : coût assez bas pour que ρ reste sous ~0,75 dans les 400 ans du banc (seules des séries censurées sont vérifiables).
  - `decline_onset_years`, `decline_years`, `decline_min_vigour` : ancien déclin de vigueur **programmé par l'âge** ; **ignoré** dès que `maintenance_per_m` > 0 (gardé pour les génomes sans bilan carboné). N'en écrivez plus.
- `symbiosis` : `mycorrhiza` = `None`, `Arbuscular` ou `Ecto` (Pinaceae, Fagaceae, Betulaceae dont *Corylus*, *Tilia* : ecto ; Rosaceae, *Acer*, *Fraxinus*, *Juglans*, *Taxus*, Cupressaceae : arbusculaire en général, Wang & Qiu 2006 ; à vérifier espèce par espèce, le banc compare avec le texte de l'entrée de référence), puis `carbon_cost`, `phosphorus_share`, `nitrogen_share`, `water_share`, `hyphal_reach_m` (calibrations, vue des flux seulement).
- `pruning` (`pruning.rs`) : `epicormic_potential` (banque de bourgeons latents 0-1), `sprout_vigour`, `stump_sprouting` (rejets de souche 0-1 : 0 pour la plupart des résineux), `root_suckering`, `compensatory_growth`, `woundwood_ratio`, `compartmentalization` (CODIT 0-1), `decay_infection_rate`, `decay_axial_m_per_year`, `decay_radial_mm_per_year`, `pathogen_dormant_risk`, `pathogen_summer_risk`, `pathogen_dieback_years`, `reserve_in_branches`, `root_balance_rate`, `sap_bleeding`, `typical_lifespan_years` (**le banc en tire la durée simulée et l'âge « adulte »**, voir § 5 ; il sert aussi à l'estimation d'« espérance de vie » du panneau de taille ; ce n'est **pas** un âge de mort, la mort émerge de `maintenance_per_m`). `stump_sprouting` et `root_suckering` décident aussi si la souche rejette après la mort du tronc (probabilité = le plus grand des deux).
- `mechanics` (lecture seule, pas d'effet sur la croissance) : `green_density_kg_m3`, `green_mor_mpa`, `green_moe_gpa` (bois **vert** ; une valeur à 12 % d'humidité n'est pas une valeur verte : dites `proxy` ou `estimated`), `crown_drag_coefficient`, `vogel_exponent`, `anchorage_nm_per_kg` (Creg de ForestGALES ; 178,1 = épicéa de Sitka, drainé, enracinement profond).

## 4. Caler l'espèce (dans cet ordre)

1. **Le fichier se charge et pousse** : `cargo test -p dendro-core --release --test species_registry` (messages explicites : nom de fichier, champs vides, persistant sans longévité, plante qui ne pousse pas).
2. **Phénologie** (les dates ne dépendent pas de la taille) : `cargo run -p dendro-core --release --example grow -- --species <id> --years 40 --pheno` (jour de débourrement, de floraison, de fin de chute, 0-based) ; réglez `gdd_budburst`, `bloom_gdd`, `senescence_*`.
3. **Hauteur** : `--example grow -- --species <id> --years 100 --seed 1` (puis graines 2-5). Leviers : `max_shoot_length_m` (pousse maximale), `vigour_per_light`, `hydraulic_path_max_m` (plafond), `hydraulic_exponent` (forme de la courbe).
4. **Diamètre** : `growth_coefficient`. Le DBH varie moins que proportionnellement (pommier, mesuré : coefficient × 0,86 → DBH à 80 ans × 0,92) : procédez par petits pas et vérifiez sur 5 graines.
5. **Forme** : `--example species_probe -- <id> --ages 20,60,100` (largeur/hauteur, fût, charpentières, feuillage de janvier, rafale critique et point faible). Leviers : dominance apicale, consignes d'angle, `crown_spread_ratio`, `whorled`, `lateral_planarity`.
6. **Reproduction** : `--example crop_years -- <id> <graine> <ans>` (années de récolte) ; `--example fruit_mesh -- <id> <graine> <ans> <jour>` (fruits dessinés, hauteur).
7. **Banc** : `cargo run --release -p dendro-validate -- --species <id> --out /tmp/<id>.md` (quelques dizaines de secondes ; `--years <id>=N` pour changer la durée ; `--check` vérifie tous les fichiers de références, même sans génome). Ne régénérez `docs/validation/report.md` qu'avec la commande complète, sans `--species`.
8. **Image** : recompilez la liaison (`cargo build -p dendro-godot` : **les génomes y sont embarqués à la compilation**), puis `DENDRO_SHOT=/chemin/absolu.png DENDRO_SPECIES=<id> DENDRO_YEARS=60 DENDRO_DOY=200 DENDRO_FLUX=0 DENDRO_RINGS=0 ~/.local/bin/godot --path godot` (été) et `DENDRO_DOY=20` (hiver), `DENDRO_CAM=leaves` (gros plan), `DENDRO_CAM=flowers` (fleurs, fruits). Regardez vous-même les images.
9. **Tests** : ajoutez vos tests d'espèce sur le modèle de `crates/dendro-core/tests/pilots.rs` (médiane de 5 graines fixées d'avance, bandes tirées des références, conventions écrites).

## 5. Pièges connus

- **DBH et fourche basse.** Le DBH est lu sur la chaîne des pousses terminales à 1,3 m. Une dominance apicale faible dans la jeunesse fait fourcher la tige sous 1,3 m et le DBH tombe sur un rejet mince (pommier essayé à 0,56 : DBH ÷ 4). Gardez une jeunesse excurrente (`apical_dominance_young` ≥ 0,6, `juvenile_phase_years` ≥ 8) et rendez l'adulte décurrent par `apical_dominance_mature`.
- **Le banc simule `1,4 × typical_lifespan_years`** (arrondi à 50 ans, entre 100 et 400) et prend l'âge « adulte » à `0,6 × typical_lifespan_years` (arrondi à 10 ans). Une série s'arrête à la mort du tronc (vitality.rs) ; la longévité jugée est l'âge médian de mort sur les graines (censurée si plus de la moitié vit encore à la fin de la série). Une espèce longévive (> 400 ans) n'est donc jugée que par censure, et son DBH maximal peut dépasser les records s'il est atteint avant la mort réelle.
- **Mort et tests** : un test qui lit un arbre à un âge proche ou au-delà de la longévité lit peut-être un tronc mort (cernes arrêtés, DBH figé, puis 0 après l'effondrement). Vérifiez `Simulation::trunk_alive()` ou la longueur de `rings()`.
- **Entrées de verger greffé.** Le moteur simule un arbre **franc de pied** (semis), jamais greffé ni taillé en forme. Les entrées de référence dont `conditions.extra` commence par `VERGER greffé` sont classées non comparables par le banc ; seules `FRANC/plein vent` (et `MIXTE`, avec prudence) servent au calage.
- **Résineux et mécanique** : la traînée du houppier est répartie par cellule de la surface frontale (plus selon le feuillage porté), et le bois nu d'un houppier caduc est masqué et plafonné par l'enveloppe feuillée. Sur un cône monopodial (épicéa, mélèze), la tige haute reste trop mince pour la silhouette qu'elle porte : la flèche casse encore vers 15-17 m/s en feuilles à 60 ans. Les rafales critiques **en feuilles** des résineux **ne sont pas valides** (voir `docs/sources.md`, « Écarts connus »). Ne réglez pas le génome pour le masquer. Pour les feuillus caducs, le test `leafless_crown_withstands_more_wind_than_crown_in_leaf` s'applique sans exception.
- **Persistants et productivité** : le feuillage d'hiver photosynthétise dès 4 °C ; la normalisation de la productivité annuelle a été calée sur une saison caduque : un persistant a une vigueur relative un peu plus haute, absorbée par le calage de `vigour_per_light` et `growth_coefficient`.
- **Aiguilles et budget de rendu** : un persistant porte L cohortes, donc ~L fois plus de cartes de feuillage. Le budget (380 000 triangles) éclaircit les cartes au-delà ; gardez `foliage_density` modéré (épicéa 3) et vérifiez le banc de performance (`DENDRO_BENCH=8 DENDRO_SPECIES=<id> DENDRO_YEARS=150 DENDRO_VSYNC=0`).
- **Fruits de 2 ans et cônes persistants** : le cycle de reproduction est annuel et tout disparaît le 31 décembre ; les cônes de pin (2 saisons), les cônes ouverts gardés des années ne sont pas représentables.
- **Planche d'herbier** : seules celles du chêne et du merisier sont dessinées (code GDScript à la main). Les autres espèces ont une fiche générique (noms, catégorie) et l'entrée « Planche de l'espèce » du menu pause est grisée.
- **Graines et masting** : le tirage des années de récolte utilise son propre flux aléatoire (`S_CROP`) : changer `crop_interval_years` ne change pas la croissance.

## 6. Ce qui est impossible sans toucher au code

- Greffe, porte-greffe nanifiant, formes fruitières taillées, éclaircissage, charge des fruits sur les branches, rendement en kg.
- Cônes sur deux saisons (*Pinus*), cônes persistants (sérotinie), cônes dressés qui se désarticulent sur l'arbre (*Abies* : seul le dessin dressé existe), arille rouge de l'if et noix/glands dessinés spécifiquement (dessinés comme des fruits ronds).
- Feuilles composées (*Fraxinus*, *Juglans* : une texture de folioles serait nécessaire ; en attendant `lobes: 0` et une feuille longue), rameaux courts de mélèze et de cèdre en rosettes (approximés par `tip_bunching`).
- Pousses polycycliques (pousse de la Saint-Jean du chêne), pousses intercalaires entre verticilles, bourgeons proventifs.
- Endurcissement au froid hivernal, dégâts de gel du bois, sècheresse hivernale des persistants (gel du sol).
- Port arbustif multicaule (noisetier en cépée : un seul tronc est simulé ; le recépage par la taille fonctionne).
- Résine, scolytes, maladies autres que les pathogènes de plaie génériques.
- Planches d'herbier des nouvelles espèces.
