# Poirier — Pyrus communis (et P. pyraster) : fiche de traits pour le génome

Jeu de référence : `data/validation/pyrus_communis.ron` (41 entrées). Légende : **mesuré** (id) ou **estimé**.

## Franc ou greffé
| Cas | Taille | Fruits | Longévité | Verdict |
|---|---|---|---|---|
| *P. pyraster* sauvage (franc) | jusqu'à 22 m, fût net 10 m, DBH 45-80 cm à 80-150 ans, max 130 cm [euforgen_pyraster_*] | non trouvé | 80-150 ans, max 250 | **Cible directe** |
| Poirier franc de semis (fr) | jusqu'à 20 m [fr_wiki_franc_height] ; adulte de verger 10-15 m | — | jusqu'à 200 ans [fr_wiki_longevity] | **Cible** pour un franc de plein vent |
| Poirier à poiré, haute tige (Angleterre) | « très haut », racines profondes | jusqu'à 10 ans ou plus [perry_wiki_first_fruit] | pleinement productif 250 ans (page Wikipedia, sans source primaire) | **Cible** (poiré breton/normand proche) |
| Sur cognassier C / A | 2,5-3 / 3-4,5 m (RHS) ; 2,4-3 m (pépiniériste) | 4 ans | — | Verger, hors périmètre |

Aucune source bretonne lue pour le poiré (Domfront, Normandie, mentionné par Wikipedia en seulement).

## Traits du génome du merisier
- **Architecture** : forme élancée, houppier ascendant et étroit en bonne station [euforgen_pyraster_height]; plus vigoureux que le pommier (pépiniériste) ; fructification sur dards/lambourdes, comme le pommier : **estimé** (aucune source lue sur les types de pousses du poirier). Épines sur les formes sauvages [fr_wiki_wild_fruit, de_wiki]. Hallé-Oldeman : non trouvé.
- **Feuille** : ovale à elliptique, 2-10 cm, pétiole long : **mesuré** (Wikipedia fr).
- **Phénologie** : floraison avril-mai [fr_wiki_flowering] ; fleurit avant le pommier (Bernwode, qualitatif). Besoin en froid : non trouvé. Gel (Bartlett, 30 min) : 10 % -9,4 °C (écailles qui s'écartent), -4,4 °C (jeune grappe), -2,2 °C (pleine floraison) ; 90 % : -17,8, -9,4, -4,4 °C [psu_pear_*] : secondaire, WSU EB0419 : proposé `flower_frost_c` = -2,2.
- **Bois** : 690 kg/m³ à 12 % (basale 520), MOR 83,3 MPa, MOE 7,80 GPa [wdb_pear_*] ; densité plus faible que le pommier ; diffus-poreux, **mesuré (compilation)**.
- **Racines** : pivot pour le sauvage, tolère les sols très secs [euforgen_pyraster_taproot] ; **estimé** pour la profondeur.
- **Tolérances** : lumière exigeante ; tous sols sauf très acides, optimum calcaire frais [euforgen] ; sécheresse bonne (pivot).
- **Reproduction** : fruits sauvages 1,5-3 cm, pierreux ; première fructification du franc : non trouvée (supposée > 10 ans, 7-10 estimé) ; alternance : non quantifiée ici.
- **Longévité** : `decline_onset_years` 80-100, `typical_lifespan_years` 150-200 : **mesuré** (divergences 100-150 sauvage vs 200).
- **Symbiose** : non trouvée ; arbuscule supposé (**estimé**).
- **Taille** : peu réactif ; feu bactérien (hors périmètre) : non traité.

## Moteur
Greffe (cognassier), fructification sur lambourdes, épines, poires lourdes qui cassent les branches, fruits au sol, poiré de grands arbres (hauteurs > 20 m), dimorphisme franc/greffé.

## Non trouvé
Âge de première fructification du franc, besoin en froid, rendement par arbre de haute tige, profondeur racinaire, mycorhizes, phénologie bretonne.
