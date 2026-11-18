# Validation de la physique générique et du climat de Rennes

Jeu de données : `data/validation/physics.ron` (92 entrées, `subject: "physics"`, syntaxe vérifiée par désérialisation avec la crate `ron` 0.8).
Consultation du 2026-11-17. Chaque entrée dit ce qui a réellement été lu (`read`) :

| Origine | Entrées | Lecture |
|---|---|---|
| Fiche climatologique Météo-France Rennes-St Jacques (PDF) | 36 | `full` (PDF entier, 2 pages) |
| Résumés d'articles (Europe PMC, OpenAlex, Semantic Scholar, PubMed) | 33 | `abstract` : le texte complet n'a PAS été lu |
| Wikipédia (en, fr, de) | 17 | `table` ou `via_wikipedia` |
| Secondaire (blog de Hemery, valeurs recopiées de `docs/sources.md`, prépublication, thèse) | 6 | `snippet` / `abstract` |

Ce qui n'a pas pu être lu : les textes complets de Moore & Maguire 2004/2005, de Jackson et al. 2019 (embargo PMC), de Hemery et al. 2005, de Granier et al. 1999 (pas de résumé), de Canadell 1996 (pas de résumé), de Peltola et al. 2000. Aucun coefficient connu seulement de mémoire n'a été écrit : si une valeur manque, elle figure plus bas dans « Non trouvé ».

Nouveaux observables utilisés (à ajouter à la liste de FORMAT.md) : `temp_mean_c`, `temp_max_mean_c`, `temp_min_mean_c`, `diurnal_range_c`, `frost_days`, `frost_days_tn_le_m5`, `frost_days_tn_le_m10`, `frost_days_tx_le_0`, `hot_days_tx_ge_25`, `hot_days_tx_ge_30`, `record_high_c`, `record_low_c`, `sunshine_h_yr`, `global_radiation_mj_m2_yr`, `etp_mm_yr`, `rain_mm_yr`, `rain_mm_month`, `rain_mm_day_record`, `rain_days_ge_1mm`, `rain_days_ge_5mm`, `rain_days_ge_10mm`, `snow_days`, `wind_mean_ms`, `gust_days_ge_16ms`, `gust_days_ge_28ms`, `gust_record_ms`, `f0_vs_d_over_h2` (relation, `value: None`), `f0_debranched_vs_intact`, `f0_winter_gain_pct`, `pruning_crown_mass_threshold_pct`, `anchorage_moment_gain_deep_roots_pct`, `tilt_at_critical_moment_deg`, `storm_damage_plot_fraction`, `storm_damaged_forest_fraction`, `windthrown_volume_m3`, `forest_destroyed_fraction`, `root_beta`, `root_fraction_top_10cm`, `root_fraction_top_30cm`, `root_fraction_top_40cm`, `rew_stress_threshold`, `budburst_rmse_d`, `crown_diameter_vs_dbh_linear`, `crown_to_stem_ratio`. Des entrées purement qualitatives (`value: None`, sans min ni max) servent de règle à vérifier, pas de cible numérique.

## 1. Climat de Rennes (station Rennes-St Jacques, 36 m, normales 1991-2020)

Source primaire lue en entier : fiche Météo-France 35281001 (édition du 06/06/2025).

- Températures : moyenne 12,4 degC (janv. 6,2, juil. 19,2, août 19,3), Tx 16,8, Tn 8,0. Amplitude Tx-Tn : 5,9 (janv.) à 11,1 degC (juil.), dérivée de deux normales.
- Jours de gel (Tn <= 0) : 30,1 par an (janv. 7,7, déc. 7,5, oct. 0,3); Tn <= -5 : 2,7; Tn <= -10 : 0,1; Tx <= 0 : 0,8; Tx >= 25 : 43,5; Tx >= 30 : 8,9.
- Records : minimum -14,7 degC (17-01-1985), maximum 40,5 degC (18-07-2022).
- Insolation 1761,5 h; rayonnement global 427 327 J/cm2 (4273 MJ/m2); ETP Penman 836,9 mm/an sur 2001-2020 seulement.
- Vent moyen 10 min à 10 m : 3,6 m/s (3,9 en janv.-févr., 3,2 en août-sept.). Jours avec rafale >= 16 m/s : 41,7; >= 28 m/s : 0,6. Rafale maximale 38 m/s le 15-10-1987; décembre : 35 m/s le 26-12-1999; records mensuels de 23 m/s (juillet) à 28,7 m/s (juin) en été, 24,8 m/s en août.
- Pluie (même fiche, page 1) : 691 mm, 114,6 jours >= 1 mm, 47,8 >= 5 mm, 18,6 >= 10 mm; mois le plus sec août 43,5 mm; plus humides oct.-déc. 73 mm; hauteur quotidienne maximale 70,4 mm (19-01-1995). Wikipédia donne les mêmes valeurs. Neige : 7,7 jours/an (Wikipédia en, période 1981-2010).

Distinction à faire dans tous les tests qui suivent : les comparaisons marquées « contrôle de calage » ne peuvent que confirmer ce sur quoi le moteur a été ajusté (normales mensuelles de température, comptages de vent Météo-France, amortissement de Jackson, Cd de houppier). Les comparaisons **indépendantes**, que le moteur n'a pas servi à ajuster, sont celles qui valident vraiment : queues Tn <= -5, Tn <= -10 et Tx >= 30 et records de température (depuis le générateur WGEN, les jours Tn <= 0, Tx >= 25 et Tx <= 0 et les normales mensuelles Tn/Tx servent au calage : ce sont des contrôles de calage), jours de pluie >= 5 et >= 10 mm, saisonnalité des records de rafale mensuels, record quotidien de pluie, seuil de 80 % d'élagage de Moore & Maguire, amortissement ébranché de 1 à 8 %, fractions de racines de Jackson et al. 1996, seuil REW 0,4.

Divergences conservées :
- Record de chaleur : Wikipédia en affiche 41,5 degC pour juin; la fiche lue donne 37,9 degC en juin et 40,5 degC en record absolu. Entrée Wikipédia marquée comme à ne pas utiliser en cible.
- ETP : le moteur annonce 856 mm/an (commentaire de `climate.rs`), la fiche actuelle 836,9 mm (2001-2020). De plus le moteur calcule Hargreaves-Samani, pas Penman : la comparaison est approximative par construction.

### Tests proposés (climat)

| Entrée | Sortie attendue | Tolérance (justification) |
|---|---|---|
| 1000 ans simulés, moyenne annuelle de T (contrôle de calage) | 12,4 degC | ± 0,3 degC (tolérance de travail (écart-type interannuel du générateur `annual_temp_sd` / sqrt(n) bien plus petit; la marge couvre le calage « à l'oeil ») |
| Moyenne de janvier et de juillet (contrôle de calage) | 6,2 et 19,2 degC | ± 0,5 degC (tolérance de travail) |
| Amplitude Tx-Tn de janvier et de juillet (contrôle de calage) | 5,9 et 11,1 degC | ± 0,7 degC (tolérance de travail) |
| Jours de gel par an (contrôle de calage depuis le générateur WGEN : les comptages mensuels calent les écarts-types de Tn) | 30,1 (mensuel 7,7 7,4 3,5 1,0 0,0 0,3 2,8 7,5) | ± 4 j/an (tolérance de travail, non publiée ; test `temperature_matches_rennes_monthly_normals_and_frost_days`) |
| Jours Tn <= -5 et Tn <= -10 (INDÉPENDANT, hors calage) | 2,7 et 0,1 | 1 à 4 et < 0,5 : **estimation de l'auteur de cette synthèse, pas une tolérance publiée** ; non utilisée par le banc (verdict « indicatif » sur la valeur publiée). Le générateur WGEN donne 4,3 et 0,27 jours : hors de cette estimation pour Tn <= -5 (voir sources.md, « Écarts connus ») |
| Pluie annuelle (contrôle de calage) | 691 mm | ± 10 % (tolérance de travail; variation interannuelle non publiée ici) |
| Jours de pluie >= 1, >= 5, >= 10 mm (>= 5 et >= 10 INDÉPENDANTS) | 114,6 ; 47,8 ; 18,6 | ± 6, ± 5, ± 3 j (tolérances de travail, non publiées) |
| Hauteur de pluie quotidienne maximale sur 80 ans simulés | atteindre 50 à 70 mm au moins une fois | record 70,4 mm (1995); test de la queue des averses, tolérance de travail |
| Vent moyen, jours >= 16 et >= 28 m/s (contrôle de calage) | 3,6 m/s; 41,7 j; 0,6 j | déjà vérifiés par `wind_climate_matches_rennes_statistics` (3,55; 41,5; 0,61); ajouter les records mensuels |
| Maximum annuel de rafale sur 45 ans simulés | atteindre 38 m/s au moins une fois sur ~45 ans | période de retour estimée à 34 ans par le moteur : test de plausibilité de la queue, pas de référence publiée de période de retour pour Rennes (voir « Non trouvé ») |
| Maximum de rafale par mois sur 44 ans simulés | juin à août : 23 à 28,7 m/s; octobre 38, décembre 35 | contrôle de la saisonnalité de la queue (le test actuel ne regarde que le maximum global); tolérance de travail ± 5 m/s : un maximum sur 44 ans est très variable |
| ETP annuelle (Hargreaves) | entre 750 et 900 mm | marges : 836,9 (Penman, 2001-2020) et 856 (cité par le moteur); Hargreaves diffère de Penman |

## 2. Mécanique et vent

### 2.1 Fréquence propre et amortissement

- Moore & Maguire 2004 (résumé) : f0 est « fortement et linéairement » liée à DBH/H2 sur 602 arbres de huit espèces de CONIFÈRES; les arbres ébranchés ont des fréquences nettement plus hautes, l'écart croissant avec DBH/H2; amortissement interne typiquement < 0,05, indépendant du diamètre. Pente non obtenue.
- Moore & Maguire 2005 (résumé, 9 douglas) : f0 linéaire en DBH/H2 avec ou sans houppier; il faut retirer au moins 80 % de la masse du houppier pour que f0 augmente de façon notable; amortissement 8 % à « presque critique » avec houppier, 1 à 8 % ébranché.
- Jackson et al. 2019 (résumé) : la poutre console prédit bien les conifères mais PAS les feuillus; la variance résiduelle des feuillus vient de l'architecture; les feuilles baissent f0 et le ratio d'amortissement. Valeurs chiffrées (8,6 ± 2,2 % l'été, 3,9 ± 1,3 % l'hiver, f0 +18 à 19 % en hiver) : recopiées de `docs/sources.md`, NON relues, à reconfirmer.
- Cadre critique : aucun jeu publié de feuillus lu ici ne donne f0 en fonction de D/H2 avec chiffres lus. Il faut donc des tolérances larges pour les feuillus.

Tests proposés :

1. Poutre console uniforme : `f1 = (1,875^2 / 2 pi) sqrt(EI / m L^4)`; déjà testée à 1,5 % près. Pas de nouvelle source.
2. Monotonie et linéarité : pour un arbre élancé, f0 doit croître avec DBH/H2 (régression de f0 sur DBH/H2 sur arbres de 10 à 150 ans : coefficient de corrélation > 0,8, pente positive). Tolérance qualitative; la pente publiée (conifères) n'est pas disponible, donc ne PAS comparer le coefficient aux conifères.
3. Élagage (INDÉPENDANT) : retirer 50 % de la masse foliaire/ligneuse du houppier doit changer f0 de moins de 10 %; retirer plus de 80 % doit la faire monter nettement (Moore & Maguire 2005). Les seuils 10 % et « nettement » sont à justifier par le seul chiffre de 80 % publié; test d'ordre de grandeur, pas de valeur précise.
4. Effet de saison : f0 d'hiver > f0 d'été; Jackson donne +18 à 19 % (non relu). Le moteur simule 1,21 à 1,71 (écart déjà signalé dans `sources.md`); tolérance de travail non publiée : 1,05 à 1,40, avec drapeau si > 1,40.
5. Amortissement en air calme (contrôle de calage : le moteur a été ajusté sur ces valeurs) : feuillé 8,6 ± 2,2 % (à reconfirmer); défeuillé 3,9 ± 1,3 %. Le moteur : 0,039 sans feuilles, 0,039 + 0,047 f avec feuilles = 0,086 pour un houppier entièrement feuillé. Tolérance : ± 2 écarts-types publiés (si les valeurs sont reconfirmées).
6. Amortissement ébranché (INDÉPENDANT) : doit rester entre 0,01 et 0,08 (Moore & Maguire 2005; douglas, 9 arbres, donc faible puissance) et < 0,05 pour l'amortissement interne seul (conifères).

### 2.2 Traînée, reconfiguration, vitesse critique

- Arbres modèles RIGIDES en soufflerie : Cd entre 0,8 et 1,2, bon accord avec les arbres naturels à faible vitesse (< 10 m/s) (Forestry 2008, résumé; auteurs non lus). Le moteur utilise Cd = 0,2 à 20 m/s pour le houppier feuillé reconfiguré : il faut l'exposant de Vogel pour comparer (non trouvé).
- Virot et al. 2016 : vitesse critique de rupture constante, ~42 m/s. Albrecht et al. 2016 (commentaire, résumé) : les données empiriques ne le soutiennent pas pour 50 % de ruptures. Divergence conservée.

Tests :

7. (contrôle de calage) Cd feuillé à 20 m/s dans [0,15 ; 0,22] (la valeur citée dans `sources.md` comme plage mesurée; source non relue ici); Cd à faible vitesse (< 10 m/s) dans [0,8 ; 1,2] comme borne haute de la reconfiguration (Cd(U) / Cd(U0) < 1 pour U > U0).
8. Vitesse critique de rupture du tronc pour un chêne mature en terrain dégagé : la seule référence numérique (42 m/s) est contestée. Ne pas imposer 42 m/s : borne de plausibilité large [25 ; 60] m/s (tolérance de travail, non publiée), à ne pas présenter comme une validation.

### 2.3 Ancrage, essais de traction, dégâts de tempête

- Nicoll (thèse 2006, auteur non confirmé) : à racines > 80 cm, moment critique 10 à 15 % plus grand que les arbres à racines superficielles (base ~2000 arbres, 12 conifères). Le moteur : `SHALLOW_ROOTING_FACTOR` = 153,2/178,1 : les racines profondes donnent 178,1/153,2 = +16 %, juste au-dessus de la fourchette de 10 à 15 %.
- Bergeron et al. 2009 (épicéa noir) : la masse de tige est le meilleur prédicteur du moment critique, la relation étant stable entre sites éloignés. Conifère, pas feuillu.
- Méthode SIM/Wessolly : divergence sur l'inclinaison au pied correspondant au moment critique : 2,5 deg (« courbe généralisée ») contre 1 deg (courbe réellement utilisée, selon une prépublication non évaluée). Aucune valeur feuillue n'a été lue.
- Tempêtes : Lothar en Suisse : 37 % des placettes touchées contre 17 % pour Vivian; le dégât augmente avec la hauteur du peuplement, la part de résineux, l'hydromorphie et la profondeur du sol; l'étude France/Allemagne/Suisse (969 sites) retrouve hauteur du peuplement, part de résineux, pH acide; Wikipédia (fr/en) : Rennes 126 km/h = 35 m/s en 1999; un quart de la forêt bretonne détruit en 1987; l'analyse de la Forestry Commission n'a pas confirmé que les feuilles résiduelles aient aggravé les dégâts de 1987.
- Aucun taux de dégâts par essence (chêne, hêtre, merisier) n'a été obtenu : les pages ne les donnent pas; les articles IFN de Colin et al. n'ont pas été trouvés.

Tests :

9. Moment critique de renversement d'un arbre vs masse de tige : doit croître avec la masse de tige (corrélation positive forte). Aucune pente à imposer (aucune valeur feuillue lue).
10. Gain d'ancrage racines profondes / superficielles : entre +10 % et +16 % (Nicoll, conifères; transposition aux feuillus à signaler).
11. Risque de dégâts : avec une rafale de 35 m/s (Lothar), un peuplement de chênes de 100 ans en bon état ne doit pas voir 100 % de ruptures; avec 38 m/s non plus; la proportion de dégâts doit augmenter avec la hauteur (étude suisse du WSL sur placettes). Test : fraction d'arbres cassés ou déracinés croissante avec H pour une rafale fixée.
12. Règle de Léonard (Phys Rev Lett 2011) : somme des sections des branches filles / section du parent proche de 1 aux noeuds de l'architecture simulée; la fourchette [0,8 ; 1,2] est une tolérance de travail non publiée, à ajuster à la dispersion observée sur les arbres réels.

### 2.4 Thigmomorphogenèse

- Jaffe 1973 (via Wikipédia en, non lu) : les plantes de serre sont plus hautes et plus élancées; la stimulation mécanique freine l'élongation.
- Hêtre (Ann For Sci 2022, résumé) : 40 perchis, moitié haubanés : la répartition de la croissance radiale est sous contrôle biomécanique; Foreco 2023 : le tronc est renforcé plutôt que l'ancrage au stade perchis. Pas de coefficient publié dans les résumés.

Test :

13. Deux arbres identiques, l'un avec acclimatation au vent, l'autre sans : l'arbre acclimaté doit avoir un diamètre de tronc plus grand à hauteur égale (H/D plus faible). Direction seulement; aucune amplitude à imposer.

## 3. Bois (propriétés mécaniques, Wikipédia de)

Table « Eigenschaften ausgewählter Holzarten » de de.wikipedia.org (note de bas de table non vue) : chêne 670 kg/m3, flexion 95 MPa, E 13 GPa; hêtre 690, 120, 14; frêne 690, 105, 13. Espèce de chêne non précisée, humidité (12 %) précisée seulement pour la densité du chêne. La page donne 10 à 22 % de variabilité naturelle.

Test :

14. MOR et MOE du moteur pour le chêne : MOR 95 MPa ± 22 %, MOE 13 GPa ± 22 %, à convertir entre état vert et sec avant comparaison (valeurs de manuel à 12 %).

## 4. Sol, eau, racines

- Jackson et al. 1996 (résumé) : profil Y = 1 - beta^d; beta = 0,970 pour les arbres (26 % des racines dans 0-10 cm, 60 % dans 0-30 cm); moyenne globale beta = 0,966 (30, 50, 75 % dans 10, 20, 40 cm); conifères tempérés beta = 0,976 (50 % dans 30 cm); rapports racine/pousse des forêts ~0,1-0,5.
- REW : la production primaire des forêts européennes chute quand REW < ~0,4 (Phil Trans R Soc B 2020, résumé, 10 stations de flux en 2018). Pas Granier 1999 (non lu).
- Pas de valeur lue pour la réserve utile des limons, le drainage ou les potentiels hydriques.

Tests :

15. (INDÉPENDANT) Distribution des racines fines d'un chêne de 100 ans : fraction dans les 10 premiers cm entre 0,15 et 0,40, dans les 30 premiers cm entre 0,45 et 0,75 (centrée sur 0,26 et 0,60, tolérance couvrant la dispersion entre biomes, valeur d'un seul biome non publiée ici).
16. Rapport racine/pousse d'un arbre de forêt dans [0,1 ; 0,5].
17. Stress hydrique (INDÉPENDANT) : à REW < 0,4, la fonction de stress de croissance doit être nettement sous 1; le seuil de 0,6 pour « proche de 1 » est une tolérance de travail, non publiée.

## 5. Écophysiologie

- Cannell & Smith 1983 (résumé, épicéa de Sitka) : temps thermique = jours-degrés > 5 degC depuis le 1er février; jours de froid = jours < 5 degC depuis le 1er novembre; débourrement prédit le 12 mai en moyenne (23 avril - 30 mai) à Eskdalemuir. Conifère; pas du chêne.
- Modèle publié dans Geosci Model Dev 2024 (auteurs non lus) : RMSE 5,7 ± 0,5 j pour la prédiction du débourrement de charme, chêne sessile, châtaignier.
- Hemery et al. 2005 (secondaire, via blog) : relation houppier-DBH presque linéaire de 20 à 50 cm; noyer 13,27 m à 0,60 m de DBH.

Tests :

18. Débourrement du chêne à Rennes : dans ± 6 j (le RMSE publié) de la valeur calibrée (RENECOFOR dans `sources.md`); toute tolérance plus serrée serait sur-ajustée.
19. (supprimé : l'ampleur de l'avance du débourrement par degré de réchauffement n'a pas été trouvée dans les sources lues; voir « Non trouvé ».)
20. Largeur du houppier d'un feuillu isolé : doit croître presque linéairement avec le DBH entre 20 et 50 cm (r2 > 0,8 publiée); chiffre à comparer à la table de Hemery (non lue).

## 6. Lois du moteur qui semblent fragiles ou non sourcées

1. ETP : 856 mm/an cité dans `climate.rs` contre 836,9 mm (Météo-France, 2001-2020); de plus, le calcul est Hargreaves-Samani, pas Penman.
2. Amortissement `damping_leafless` / `damping_leaves` : 0,039 et 0,047 fixés d'après quatre feuillus (Jackson 2019), même valeur pour toutes les espèces : estimation, non relue ici. L'amortissement aérodynamique en tempête est une estimation (`sources.md` : aucune mesure de tempête trouvée).
3. Rapport de fréquence hiver/été : moteur 1,21 à 1,71 contre 1,18 de Jackson : fragilité déjà signalée.
4. Fréquence propre : Rayleigh sur un arbre aux branches rigides et à la motte rigide : surestimation avouée; ni Moore & Maguire (conifères) ni les feuillus de Jackson (architecture) ne fixent une pente de référence.
5. `SHALLOW_ROOTING_FACTOR` : 153,2/178,1 : +16 %, à la limite haute de 10 à 15 % (conifères) : transposition aux feuillus non sourcée.
6. Vitesse critique : toute calibration sur la « constante » de 42 m/s de Virot et al. est contestée (Albrecht et al.). Les rafales de 1987 (38 m/s) et 1999 (35 m/s) à Rennes sont des repères plus solides.
7. Queue des rafales : Weibull + queue exponentielle d'échelle 3,0 m/s; période de retour de 38 m/s estimée à 34 ans (45 ans dans `sources.md`) : jamais confrontée à une période de retour publiée.
8. Cd du houppier (0,2 à 20 m/s) : le moteur utilise un exposant de Vogel; B, et le Cd de référence, n'ont pas de source lue ici; les soufflerie rigides donnent 0,8-1,2.
9. Fonction de stress en REW : seuil de 0,4 retrouvé dans une publication de 2020 mais l'origine Granier (BILJOU) n'a pas été relue.
10. Limites physiques du moteur déjà avouées : pas de sol gelé, pas de nappe saisonnière, branches rigides.

## 7. Non trouvé

- Pente et ordonnée de f0 = a D/H2 + b pour des feuillus (Moore & Maguire : conifères seulement; Jackson : feuillus, figures non lues).
- Exposant de Vogel B par espèce et Cd de houppier feuillu de Vollsinger/Rudnicki/Vogel (seul un résumé de simulation à Cd rigide et un résumé théorique ont été lus).
- Essais de traction sur feuillus (Peltola, Kamimura, Detter/Rust, Göcke) : pas de moment critique chiffré lu; seuls quelques résumés de conifères.
- Taux de dégâts par essence pour Lothar/Martin/Klaus (IFN, Colin et al., Bock et al. : texte non lu; Bock : seulement les prédicteurs).
- Périodes de retour des rafales en Bretagne (Météo-France, Infoclimat) : non trouvées ici.
- Réserve utile des limons, REW de Granier (texte), potentiels hydriques, drainage.
- Loi de Pressler, modèle du tuyau (Shinozaki), rapport transpiration/surface foliaire, WUE : rien lu.
- Sensibilité du débourrement du chêne au réchauffement hivernal (jours par degC) : non lue.
- Coefficients de Hemery et al. 2005 pour chêne et merisier (seulement le noyer, via blog).
- Paramètres de Cannell & Smith (seuils de forçage/froid) et modèle de Chuine pour le chêne.
- Facteurs de sécurité des branches et du tronc, contrainte des branches (Evans 2008) : aucun résultat.
- Wikipédia « Climat de Rennes » (fr) : page inexistante (404); le climat est dans « Rennes ».
