# Quercus petraea (chêne sessile) — fiche de traits

Jeu de référence : `data/validation/quercus_petraea.ron` (40 entrées). Beaucoup de données sont communes avec *Q. robur* (Atlas, EUFORGEN) : repartir de `quercus_robur.ron` pour le reste.

## Port et architecture
- Contrairement au pédonculé, le sessile garde un fût principal dominant, houppier plus serré et ramifications décroissantes (Atlas). Fût plus droit, moins d'épicormiques que robur (jugement usuel ; non chiffré ici). Modèle architectural : non vérifié.

## Traits du génome (proposition)
| Trait | Valeur | Source / statut |
|---|---|---|
| height | 25-40 m (30 usuel) ; >40 m rare | Atlas, WP fr/de |
| dbh | ≤1 m usuel ; 3-4 m records | Atlas (commun robur) |
| longevity | >600, parfois 1000 ans | WP fr ; Atlas >1000 ; EUFORGEN 800+ |
| first fruiting | 40-100 ans (EUFORGEN) ; 60 ans (WP fr) ; 40-50 (WP pl) | divergence, voir .ron |
| rotation | ~160 ans (qualité, rings fins) | Atlas |
| budburst | doy 101 (11 avril), RENECOFOR ; pédonculé 100 | mesuré |
| flowering | mi-mai (doy ~135) en France | WP fr (estimé) |
| end yellowing | doy 299 | RENECOFOR |
| frost foliage / acorn | -3 °C / -6 °C | Atlas |
| shade | 2,73/5 | Niinemets (mesuré) |
| drought | 3,02/5 | Niinemets |
| waterlogging | 1,2/5 (moins que robur) | Niinemets |
| roots | pivot profond, plus développé que robur | Atlas |
| resprout | bon (têtards, taillis) | Atlas |
| wood | ~650 kg/m³ (0,39-0,93) ; E_L 13,3 GPa à 12 % | WP de/fr (non vérifié) |

## À coder / non trouvé
Marcescence de jeunes chênes (existe, non chiffrée ici). Données vert (MOR/MOE) non trouvées pour le sessile seul. Sensibilité thermique du débourrement la plus forte (-7,5 j/°C, Vitasse 2009 résumé) et avancée de -0,42 j/an à Fontainebleau : utile pour calibrer le modèle de débourrement GDD.
