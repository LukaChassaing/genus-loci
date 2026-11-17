# Jeu de référence pour *Quercus robur* (chêne pédonculé)

Fichier de données : `data/validation/quercus_robur.ron` (132 entrées, format de `data/validation/FORMAT.md`).
Recherche documentaire du 2026-11-17, menée **sans exécuter le moteur** et sans reprendre les sources déjà citées par
`docs/sources.md` comme autorité (White 1998, Granier 1999, Lucot et Bruckert 1992 ne figurent qu'en entrées
marquées et limitées). Les ordres de grandeur du génome sont comparés ensuite, en fin de document.

## Ce qui est dans le fichier

| Domaine | Entrées | Remarque |
|---|---|---|
| Hauteur, DBH, houppier, peuplement, circonférence | 31 | peu de valeurs « arbre isolé » ; les séries de peuplement viennent de Cienciala 2008 (scénario simulé) et de Tomczak 2024 (mesuré, Pologne) |
| Longévité et arbres remarquables | 13 | 12 viennent de Wikipédia ; chaque langue donne un âge différent pour le même arbre |
| Cernes | 8 | 6 calculées par nous sur la série brute ITRDB GERM008 (forêt, Allemagne) |
| Biomasse aérienne | 4 | équations de Cienciala 2008 (valeurs dérivées signalées) |
| Racines | 4 | pauvre : une valeur polonaise non référencée, Lucot et Bruckert (résumé), Atlas qualitatif |
| Phénologie (débourrement, floraison, fruits, chute, gel) | 22 | **meilleure couverture** : 4 ans x 20 peuplements (Angleterre du Sud), Wytham, BIFoR |
| Reproduction (âge de fructification, glandées) | 11 | intervalles de glandée contradictoires (2 à 9 ans) |
| Feuilles, LAI, LMA, eau | 7 | LMA et LAI mesurés ; transpiration journalière, pas de total annuel |
| Tolérances (ombre, sécheresse, hydromorphie), rendement, révolution | 7 | Niinemets et Valladares 2006 (échelle 0 à 5) |
| Bois (densités, MOR, MOE) | 14 | aucune valeur « bois vert » pour *Q. robur* |
| Mécanique (tempêtes, vent critique, fréquence propre) | 4 | dégâts de Lothar (groupe hêtre/chêne), 42 m/s de Virot ; aucune fréquence propre ni amortissement |
| Épicormiques, cicatrisation, mycorhizes | 7 | qualitatif surtout |

Origine : 54 primaires, 56 Wikipédia, 9 « review » (Atlas européen), 7 « handbook » (Forest Research, Read 2000, convention IPCC),
6 secondaires. Langues Wikipédia consultées : fr, en, de, nl, pl, it, es, sv, cs (les neuf). Statut de lecture :
`full` ou `table` quand j'ai lu le texte (PDF converti en texte ou fichier de données) ; `snippet` quand la valeur ne m'est
parvenue que par un résumé de moteur de recherche ou de WebFetch (Cole et Sheldon, Lanvin et Reuling, Martiník, Kerr
et Harmer, Dujesiefken, Virot, Milović, pages Forest Research) ; ces entrées n'ont pas de citation (`quote: None`) sauf
quand le texte a été vu mot pour mot.

Entrées d'absence (`value`, `min`, `max` à `None`, id commençant par `absence_`) : à ignorer par le banc, elles
consignent seulement ce qui n'a pas été trouvé. Plusieurs entrées qualitatives (`epicormic_response`, `mycorrhiza_type`,
`shade_tolerance` de Forest Research) ont aussi `value: None` : l'information est dans `note`.

### Quantités ajoutées hors liste de FORMAT.md

`frost_kill_new_foliage_c`, `frost_kill_acorn_c`, `shoot_flushes_per_year`, `mast_interval_years`, `acorn_mass_g`,
`leaf_thickness_um`, `basal_area_m2_ha`, `stem_density_per_ha`, `girth_cm`, `transpiration_mm_day`,
`transpiration_l_day_per_m2_canopy`, `yield_class_m3_ha_yr`, `rotation_years`, `storm_damage_fraction`,
`dormant_bud_longevity_years`. Les dates (jour de l'année) tirées de « fin avril », « avril à mai » sont des conversions
de ma part, signalées dans `note`.

## Valeurs clés

| Observable | Valeur | Conditions | Source |
|---|---|---|---|
| Hauteur | 11,1 / 19,3 / 26,0 m à 25 / 50 / 100 ans | peuplement géré, indice de station 3, scénario simulé (SILVISIM), Tchéquie | Cienciala et al. 2008, tab. 2 |
| Hauteur, DBH à ~130-144 ans | 31 m, 63 cm (station forestière) ; 26 m, 55 cm (ancienne terre agricole) | Pologne de l'Ouest, peuplement | Tomczak et al. 2024 |
| Hauteur usuelle | 30 m (40 m exceptionnel) | *robur* + *petraea* | Atlas européen 2016 |
| Cernes | 2,35 mm (rangs 0-20) à 1,18 mm (80-100) ; moyenne 1,63 mm | forêt de Schwalenberger Wald (D), 1830-1972, 10 séries | ITRDB GERM008, calcul propre |
| Cernes (carottes) | 2,27 mm en moyenne | peuplement polonais, 130-144 ans | Tomczak 2024 |
| Biomasse aérienne | 0,54 t (moyenne observée, D 26 cm, H 21 m) ; 1,09 t à D = 40 cm (dérivée) | 51 arbres tchèques, D 6-59 cm | Cienciala 2008 |
| Débourrement (stade 4) | jour 106 (2011), 123 (2012), 129 (2013), 119 (2014) ; extrêmes 97 à 142 | 20 peuplements d'Angleterre du Sud | Eaton et al. 2015, jeu FC |
| Débourrement | jour 122 (2013), 102 (2014) | Wytham Woods, arbres individuels (via résumé) | Cole et Sheldon 2017 |
| Première feuille | jours 88 à 117 (2017-2021) | BIFoR FACE, Staffordshire | Quick et al. (préprint) |
| Automne (stade 4) | jour 311 à 328 selon l'année | Angleterre du Sud, « couleur ou chute » | Eaton 2015 |
| Premier arbre nu | jour 330 (26 nov. 2019), 341 (7 déc. 2021) | BIFoR | Quick et al. |
| Gel | -3 °C tue le feuillage jeune ; -6 °C soutenu tue les glands | Europe | Atlas 2016 |
| LMA | 76,6 g/m² (feuilles de soleil) ; 51,9 (ombre) | calculé depuis SLA 130,6 et 192,7 cm²/g, forêt alluviale italienne | Catoni et al. 2015 |
| LAI du chêne | 1,9 à 2,8 | peuplement **dépérissant**, Belgique | Vincke et al. 2005 |
| Eau | 3,1 L/jour par mm de rayon, soit 850 à 1440 L/jour (rayon 274-465 mm) ; ~5 L/jour/m² de houppier en juillet | chênes de ~25 m, Angleterre centrale, jour seulement | Quick et al. (préprint) |
| Ombre / sécheresse / hydromorphie | 2,45 / 2,95 / 1,89 sur 0-5 (*petraea* : 2,73 / 3,02 / 1,2) | Europe | Niinemets et Valladares 2006 |
| Densité sèche | 691 kg/m³ à ~9 % d'humidité ; 650 (de), 720 (en), 660 à 780 (pl) selon l'état | divers | Tomczak 2024 ; Wikipédia |
| MOR, MOE | 97 MPa, 8,5 GPa (petits échantillons sains, ~9 %) ; 49 MPa, 11,9 GPa (bois de structure français) | **ni vert ni comparable** | Tomczak 2024 ; Lanvin et Reuling 2012 (via résumé) |
| Dégâts de Lothar | 5 % des arbres du groupe hêtre/chêne, 15 % de l'épicéa | Bade-Wurtemberg, inventaire fédéral | Schmidt et al. (NW-FVA) |
| Vitesse critique | ~42 m/s, indépendante de l'espèce | données de tempêtes ; contestée | Virot et al. 2016 (via résumé) |
| Fructification | 40-50 ans en arbre isolé, 60-80 en peuplement ; 60 ans (fr) | | pl, es, sv ; Martiník 2014 (via résumé) |
| Longévité | « 300-400 ans » typique (nl), « 500 facilement » (fr), 500-1000 (de), jusqu'à 1400 (de) | | Wikipédia |
| Production | classe de production 2 à 8, moyenne ~4 ; révolution 130-160 ans | Grande-Bretagne | Forest Research |

## Synthèse par domaine

**Croissance.** Les sources primaires sur la hauteur par âge sont rares. Les tables de production citées par les pages
(Jüttner 1955, Schober 1995) ne sont pas disponibles en ligne : les valeurs de la table de Jüttner n'ont pas été lues, ni la table
néerlandaise d'Oosterbaan (1988). La seule trajectoire hauteur-âge indépendante en peuplement est le scénario de Cienciala
(26 m à 100 ans). Elle est cohérente avec les cibles *calibrées* du génome (23-26 m à 100 ans, 19-22 m à 60 ans, 9-12 m à 25 ans,
cibles que `docs/sources.md` reconnaît comme non sourcées). À 130-144 ans, un peuplement polonais mesuré atteint 31 m (bonne
station) ou 26 m. Les diamètres de peuplement (63 cm à ~140 ans) sont inférieurs à ceux d'arbres isolés de White (~100 cm à 150 ans) : la différence isolé/peuplement
est donc une information à conserver, pas une divergence.

**Cernes.** Dans la forêt allemande étudiée les cernes décroissent de 2,35 mm (premiers 20 rangs) à ~1,2 mm (rangs 60-100), puis remontent
(1,63 mm sur rangs 100+, effet de survie et d'éclaircies à interpréter avec prudence). Le moteur, calibré sur des arbres
isolés, produit 3 à 5 mm : correct pour l'isolé, mais trop large si le jeu est comparé à un peuplement serré.
Le calcul compte les rangs depuis le premier cerne mesuré (le décalage de moelle est inconnu) et a retiré les marqueurs de fin de série (999).

**Phénologie.** Meilleur jeu : 20 peuplements de chêne pédonculé en Angleterre du Sud sur 4 ans. Le débourrement va du jour 97 au jour 142
(7 avril au 22 mai) ; la moyenne annuelle varie de 106 à 129 (16 avril à 9 mai) selon la météo, un écart de 23 jours entre 2011 et 2013.
Wytham Woods donne les jours 122 (2013) et 102 (2014). Le débourrement individuel varie de plus de 3 semaines dans une population (via résumé).
Pour l'automne, l'étape 4 survient au jour 295-335 (« couleur ou chute » non distinguées par la fiche du jeu) ; au BIFoR, la première teinte
est entre le 6 septembre et le 1er octobre, le premier arbre nu entre le 26 novembre et le 7 décembre.
Aucun jeu breton ni français (RENECOFOR, PEP725, Observatoire des saisons) n'a pu être ouvert : PEP725 demande une inscription.

**Feuilles et eau.** LMA 77 g/m² en plein soleil et 52 g/m² à l'ombre (valeurs calculées). La seule mesure de LAI trouvée concerne un
peuplement en dépérissement, donc une borne basse. Aucun total annuel de transpiration spécifique à *Q. robur* n'a été trouvé : Granier et al. 1999
(300-460 mm/an, déjà dans `docs/sources.md`) n'est pas confronté ici. Le préprint du BIFoR donne une consommation proportionnelle au rayon du tronc.

**Bois.** Les densités vont de 650 à 780 kg/m³ à l'état sec à l'air ; 691 à 9 % d'humidité (mesure) ; 1080 vert (Wikipédia polonaise).
Aucune table « bois vert » de *Q. robur* (Lavers 1983, Tropix, Wagenführ) n'a été ouverte, donc le proxy du chêne blanc américain du génome (57 MPa, 8,6 GPa) reste non vérifié.

**Mécanique.** Aucune fréquence propre ni aucun amortissement de *Q. robur* ; Jackson et al. 2019 ne contient pas de chêne identifié. Les dégâts de Lothar
disponibles concernent le groupe hêtre/chêne (5 %) contre l'épicéa (15 %). Pas de données Klaus 2009 ni Martin 1999 spécifiques au chêne.

**Tolérances.** Niinemets et Valladares classent *Q. robur* plus tolérant à l'engorgement (1,89) que *Q. petraea* (1,2), à peu près égal pour la sécheresse (2,95 contre 3,02)
et modérément intolérant à l'ombre (2,45). Forest Research : intolérant à l'ombre, tolère l'engorgement tant que le sol ne sèche pas en été.

**Épicormiques, trognes, cicatrisation.** Selon le guide de Read (2000, texte lu), le chêne forme beaucoup de bourgeons dormants (longévité de l'ordre de 100 ans), mais sa
réponse à la taille d'un arbre ancien est moyenne (**), très bonne pour créer un têtard jeune (***), mauvaise à possible pour initier un têtard sur un arbre mature. La
phrase « le chêne est généralement mauvais » porte sur la croissance **adventive**, pas sur les épicormiques issus de bourgeons dormants ; je ne l'ai pas généralisée.
Aucun taux de fermeture de plaie en mm/an pour *Q. robur* n'a été trouvé.

**Mycorhizes.** Ectomycorhizes : 21 taxons dans un peuplement serbe (via résumé), *Lactarius quietus* dominant. Aucune page Wikipédia n'en parle.

## Divergences notables entre sources

- **Âge du Kongeegen** (Danemark) : environ 1200 ans (it), 1400 (nl, de), 1500-2000 (sv). Même arbre, trois Wikipédias.
- **Hauteur de l'Ivenack** : 35,5 m (de) contre 32,2 m (nl). Âge 500-800 ans (nl) contre 700-800 ans (en).
- **Hauteur typique** : 20 m « jusqu'à » (en) contre 25-35 m (fr), 30 m (Atlas), 25-40 m (it, de 20 à plus de 40). Maximum : 38 m, 40 m, 45 m, 50 m, 57 m (cs, marqué « très atypique »).
- **Intervalle des glandées** : tous les 2 ans (en, Crawley 2005), 2-4 ans (Future Trees Trust), 4-6 ans (pl), 6-9 ans pour la grosse glandée (Croatie). Spécificité régionale probable.
- **Âge de première fructification** : 40-50 ans en isolé (pl, es, sv, Martiník) contre 60 ans (fr) ; en peuplement 60-80 ans.
- **Longévité typique** : 300-400 ans (nl) contre « facilement 500 ans » (fr) et « souvent plus de 500 » (en). Les âges des arbres à légende sont souvent surestimés (cs, nl : Doornenburg annoncé 1000 ans, estimé 400-500).
- **Densité du bois** : 0,65 (de), 720 (en), 0,74 Tropix (pl) contre 600 pour « chêne » dans un tableau Carbofor/Tropix vu par recherche. L'état d'humidité n'est presque jamais précisé.
- **MOR et MOE** selon le type d'essai : 49 MPa / 11,9 GPa (bois de structure, défauts) contre 97 MPa / 8,5 GPa (petits échantillons sains) ; non comparables entre eux ni au « vert ».

## Ce que chaque Wikipédia apporte de spécifique

- **de (Stieleiche)** : peu de chiffres, mais la gamme de densité (0,39-0,93) et les références forestières (Schütt, Enzyklopädie der Laubbäume). Pas de données sur les racines, le bourgeonnement ou les tempêtes.
- **fr** : longévité (Camus), première fructification vers 60 ans, durabilité classe 3 (CIRAD), dimensions de feuilles ; texte presque sans chiffres de station ni Lothar.
- **en** : cycle de glandée (Crawley 2005), valeurs d'Ellenberg 2022, densité ~720 kg/m³, arbres remarquables et circonférences.
- **nl** : le plus riche en dimensions d'arbres isolés (cime jusqu'à 25 m, hauteurs en isolé/forêt), plage de cernes 0,3-15 mm, critique des âges légendaires, branches datées par les cernes.
- **pl** : de loin la plus riche (croissance de jeunesse, racines à 5 m, âge de fructification, densités à plusieurs états d'humidité, deux pousses par an, glandées tous les 4-6 ans). Seuls les 100 000 premiers des 141 000 caractères de la page ont été lus ; les références ne sont que des ancres de lien, pas des fiches.
- **it** : densité 0,75, altitude 800-1000 m, plusieurs vieux arbres italiens. Peu référencée.
- **es** : peu de chiffres (40 m, 1000 ans, première floraison 40-50 ans).
- **sv** : hauteur et circonférence à 50 ans en Suède (20-25 m, 1 m), arbres remarquables (Kvilleken, Prins Eugens ek).
- **cs** : record de hauteur (57 m, abattu 1905), circonférences et âges d'une liste d'arbres tchèques, pouvoir calorifique.

## Points où le génome actuel semble s'écarter de la littérature

Sans exécuter le moteur ; je compare les valeurs des commentaires et du RON. Classement par importance estimée. « Déjà reconnu » =
déjà signalé dans `docs/sources.md`, « Écarts connus ».

**Nouveaux (non signalés dans `docs/sources.md`)**

1. **Pousses de la Saint-Jean.** Le RON dit « première pousse seulement, pas de lammas » (`max_shoot_length_m` 0,5). Wikipédia polonaise : deux poussées typiques en Pologne (mai et Saint-Jean), 4 à 5 possibles. Contradiction directe ; référence de la page (n° 25, « GoD ») non ouverte.
2. **Première floraison à 35 ans** (`first_flowering_age_years`) contre 40 à 50 ans en isolé (pl, es, sv, Martiník) et 60 en peuplement (fr, pl). Légèrement trop précoce, surtout par rapport au peuplement.
3. **Début de la décroissance du pivot : 12 ans** (`taproot_decline_years`) contre un système oblique qui n'apparaît qu'à 30-50 ans (pl, un seul site forestier polonais). Le texte du RON dit « quelques décennies » ; la valeur de 12 ans est plus courte que ce commentaire.
4. **Longévité « typique » de 600 ans** (`typical_lifespan_years`) contre 300-400 ans en cas non perturbé (nl) et plus de 500 (fr, en) ; 500-1000 (de). Acceptable pour les arbres isolés très vieux, trop élevé comme « typique ».
5. **Largeur du houppier.** `crown_spread_ratio` 1,15 donne environ 34 m de large pour 30 m de haut ; nl : « jusqu'à 25 m, parfois 35-40 m ». À vérifier : `docs/sources.md` signale seulement le rapport L/H testé (0,91 à 100 ans).
6. **Tolérance à l'ombre 0,3 contre 2,45/5** (Niinemets) : sur une échelle 0-1 naïve, 0,49. La correspondance entre échelles n'est pas établie ; la sécheresse (0,45 contre 2,95/5 = 0,59) est un peu basse. L'hydromorphie (0,4 contre 1,89/5 = 0,38) est cohérente, et la hiérarchie *robur* > *petraea* aussi.
7. **Potentiel épicormique 0,9, rejets de souche 0,85.** Read 2000 : beaucoup de bourgeons dormants (cohérent) mais pollardage d'un arbre mature « à peine possible », tolérance moyenne à la taille des vieux arbres, et viabilité des bourgeons dormants ~100 ans. Le RON cite « libère abondamment » (Meier 2012, Fontaine 2001) ; la tension est qualitative.
8. **Débourrement « mi-avril » à Rennes.** Angleterre du Sud : moyenne annuelle du jour 106 à 129 (16 avril au 9 mai), Atlas : fin avril à début mai. Si le moteur produit mi-avril, c'est à l'extrémité précoce de la plage (la Bretagne est plus douce que l'Angleterre, donc plausible mais à vérifier par la sortie du banc). La chute des feuilles (nus vers le 29 novembre, jour 333) est cohérente avec BIFoR (330-341).
9. **Cernes** de 3-5 mm de 10 à 90 ans : bon pour l'isolé (White), mais 2 à 3 fois plus larges que les peuplements mesurés (1,2-2,4 mm ; 2,27 mm en moyenne en Pologne). À signaler si un jeu « peuplement » est un jour simulé.

**Déjà reconnus dans `docs/sources.md`**

10. Courbes de hauteur non sourcées : ici cohérentes avec Cienciala (11 m à 25 ans, 19 m à 50, 26 m à 100). Il manque toujours des données en arbre isolé.
11. Propriétés du bois vert tirées du chêne blanc américain (57 MPa, 8,6 GPa, 1026 kg/m³) : le chêne pédonculé vert n'a pas pu être confronté. La densité verte de la Wikipédia polonaise (1080) est proche (+5 %).
12. Coefficient d'ancrage de l'épicéa de Sitka, traînée et amortissement de feuillus non-chêne : aucune donnée *Q. robur* trouvée non plus (pas de fréquence propre ni d'amortissement).
13. Interception et dessèchement estival : pas de confrontation possible (aucun total annuel de transpiration de *Q. robur* trouvé ; Vincke 2005 est un peuplement dépérissant).

Cohérent avec la littérature : gel foliaire -3 °C (même source), hydromorphie *robur* > *petraea*, hauteur et DBH d'arbres isolés (White, non confronté ici), appartenance ectomycorhizienne, couleur de l'écorce et porosité annulaire (pas de donnée contraire).

## Ce qui n'a pas été trouvé

- Fréquence propre, amortissement, coefficient d'ancrage, traînée de *Q. robur* ; dégâts de Klaus 2009 et Martin 1999 par essence ; vitesse de rupture par espèce.
- Table de production de Jüttner (1955), tables de l'ONF et de l'IFN, tables du Forestry Commission pour le chêne (le bulletin FC 48 mentionne des modèles de chêne sans les tabuler), table d'Oosterbaan (1988).
- Séries de phénologie en France/Bretagne (RENECOFOR, PEP725, Observatoire des saisons) : jeux non ouverts, seulement l'Angleterre.
- Total annuel de transpiration, rapport racines/pousses, LAI de chêne sain, tables de MOR/MOE vert (Lavers, Tropix, Wagenführ, Wood Handbook pour *Q. robur*).
- Taux de fermeture des plaies chez *Q. robur* en mm/an ; croissance des rejets épicormiques ; âge et dimensions des trognes (Read 2000 vu pour l'épicormique seulement).
- Jones 1959 (Biological Flora, *J. Ecol.* 47) : article JSTOR, non ouvert ; White 1998 non repris ici car déjà utilisé pour la calibration.
