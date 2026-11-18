# Pommier — Malus domestica (et M. sylvestris) : fiche de traits pour le génome

Jeu de référence : `data/validation/malus_domestica.ron` (83 entrées). Légende : **mesuré** = valeur publiée lue (id d'entrée entre crochets) ; **estimé** = choix de l'agent génome, sans source. Les entrées ont un préfixe `FRANC/plein vent` ou `VERGER greffé <porte-greffe>` dans `conditions.extra`.

## Franc ou greffé : ce qui convient à un arbre de semis
| Cas | Taille | Entrée en fruits | Longévité | Verdict |
|---|---|---|---|---|
| Sauvage *M. sylvestris* (franc) | jusqu'à 10 m, DBH 23-45 cm [euforgen_sylvestris_*] | non trouvé | 80-100 ans | **Cible directe** d'un arbre de semis |
| Haute tige sur semis ou M25 (cidre, Streuobst) | 8-15 m couronne libre [de_wiki_freestanding_height] ; DBH > 75 cm cas extrême | 8-10 ans sur semis [de_wiki_seedling_rootstock_first_fruit] ; ~10 ans sur M25 [cider_uk_standard_first_fruit] | 80-100 ans ; pic de production 20-60 ans [arche_noah_*] ; « jusqu'à 30 ans » selon Wikipedia fr (divergence) | **Cible** pour l'arbre franc de plein vent (cidre breton) |
| Verger basse tige M9 | 1,8-2,4 m [rhs_m9_height] ; 40-45 % du semis [ars_size_m9] | 2-3 ans | remplacé vers 15 ans [fr_wiki_m9_replace] | **Hors périmètre** (propre au verger) |
| MM106 | 3-4 m ; 70-75 % du semis | 3-4 ans | — | Verger |
| MM111 / M25 | 4-4,5 m / ≥ 4,5 m ; MM111 80-85 % | 4-5 / 5-6 ans | M25 : « un siècle » (guide) | Intermédiaire, plutôt haute tige |

Le Paradis (clone nain, fruit à 3-4 ans) n'est pas un témoin du franc. Pas de donnée bretonne par arbre trouvée (rendement, âge) ; les chiffres viennent de Wallonie (300-500 kg/arbre adulte [vieux_vergers_yield]), d'Autriche (200 kg en moyenne [arche_noah_yield_mean]) et du Royaume-Uni.

## Traits du génome du merisier, un par un
- **Architecture** : acrotonie/dominance apicale, angle d'insertion, phyllotaxie 2/5 (Rosaceae, 144°) : **estimé**. Port : houppier étalé, souvent buissonnant chez le sauvage [euforgen_sylvestris_height]. Le modèle de Hallé-Oldeman n'a pas été trouvé dans une source lue : ne pas l'affirmer. Typologie de Lespinasse (types I à IV, du fût à fructification latérale/axillaire au type à fructification terminale et houppier globuleux), connue par des résumés seulement ; le type I alterne, le type IV est régulier.
- **Dards, lambourdes, bourses** : le fruit naît sur un rameau court (dard) qui devient une bourse après fructification ; la bourse porte des « bourses-rameaux » l'année suivante (résumés lus). Le génome du merisier a déjà `spur_foliage` : un système de fructification sur rameaux courts est à ajouter (voir « moteur »).
- **Feuille** : 5-12 cm × 3-6 cm, pétiole 1-3,5 (it : 2-5) cm, caduque, aucun marcescence : **mesuré** (Wikipedia it/en, sans source primaire).
- **Phénologie** : besoin en froid 1000-1600 h sous 7 °C (CFIA via Wikipedia fr), ou 40-70 portions (modèle dynamique), cultivars de 218 à 1423 h : **mesuré (secondaire, divergent)** [fr_wiki_chill_*]. Floraison d'après les groupes de pollinisation anglais : 1-3 mai (précoces) à 24-28 mai [en_wiki_pollination_group_*] ; mai en Europe centrale [de_wiki_bloom_may]. La floraison avance de 7,8 j (Angers) à 15,4 j (Gembloux) depuis ~1988 [legave_*]. Aucune date bretonne. `gdd_budburst`, `bloom_gdd`, `fruit_ripening_days` : **estimé**.
- **Gel des fleurs** (`flower_frost_c`) : bourgeons « Red Delicious » 10 % tués : -9,4 °C (pointe argentée), -2,8 °C (jeune grappe), -2,2 °C (pleine floraison) ; 90 % : -16,7, -6,1, -3,9 °C [psu_apple_*, 30 min, source secondaire adaptée de WSU EB0419]. Bois dormant -40 °C (en/fr), -25 °C (it) [divergence]. Proposé : `flower_frost_c` = -2,2 ; `frost_c` du génome ≈ -25 à -40 (au choix selon que le moteur teste le bois).
- **Bois** : 830 kg/m³ à 12 % (basale 610), MOR 88,3 MPa, MOE 8,76 GPa [wdb_apple_*] ; diffus-poreux : **mesuré (compilation)**. `adaptive_stress_mpa`, `growth_coefficient` : **estimé** (aucune contrainte mesurée).
- **Écorce** : lenticelles, écorce fissurée en vieillissant : **estimé**.
- **Racines** : superficielles/étalées d'après Wikipedia it (« superficiale ») ; aucune profondeur chiffrée [absence_root_depth_apple]. Le franc de semis a un pivot (supposé, non sourcé) ; les porte-greffes de verger sont superficiels. `max_depth_m` : **estimé**.
- **Tolérances** : lumière très exigeante [euforgen_light_demand] ; ombre ≈ 0,25 comme le merisier (**estimé**) ; sécheresse moyenne (**estimé**) ; asphyxie racinaire : MM106 mal adapté aux sols mal drainés (USDA) pour le greffé uniquement.
- **Reproduction** : `first_flowering_age_years` franc ≈ 8-10 (mesuré pour franc greffé sur semis) ; `flower_density`, `fruit_set` (5 % de fleurs nouées suffisent à une récolte pleine, Wikipedia de), masse du fruit (Ø 2,5-12 cm, cidre plus petit) : le poids moyen n'est pas sourcé. Alternance : rapport 2:1 entre années fortes et faibles dans le cidre britannique [cider_uk_biennial_ratio] — nécessite un compteur de réserves.
- **Longévité** : `decline_onset_years` ~ 60-80, `typical_lifespan_years` 80-100 (franc) : **mesuré**.
- **Symbiose** : arbuscule (**estimé** : aucune source lue [absence_mycorrhiza_apple]).
- **Taille / blessures** : pommier réagit fortement : rejets verticaux (gourmands) après taille sévère ; compartimentation moyenne ; **estimé**, rien de mesuré lu.
- **Mécanique** : densité verte non sourcée ; coefficient de traînée : **estimé**.

## Ce que le moteur risque de ne pas savoir faire
Greffe et porte-greffes nains (taille 40 % du semis), fructification sur bourses/dards avec alternance bisannuelle, éclaircissage des fruits, gel par stade (tableau 10 %/90 %), chute physiologique, formes taillées (palmette, fuseau), charge en fruits qui plie les branches, hybridation sauvage/cultivar. Les fruits au sol et les gourmands de taille sont à considérer.

## Non trouvé
Profondeur racinaire, mycorhizes, rendement par arbre de pommiers à cidre bretons, floraison/maturité en Bretagne (PEP725 non téléchargé), heures de froid pour le franc, DBH-âge d'un franc de semis.
