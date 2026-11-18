# Carpinus betulus (charme commun) — fiche de traits

Jeu de référence : `data/validation/carpinus_betulus.ron` (40 entrées).

## Port et architecture
- Arbre petit à moyen, 20-25 m (Atlas), parfois >30 m en forêt ancienne ; fût cannelé et souvent tordu (« muscled », Forest Research), houppier irrégulier ovoïde puis arrondi. Modèle architectural : non vérifié.
- Souvent en sous-étage sous chêne ou hêtre ; en peuplement fermé fût de 8-10 m (WP de).

## Traits du génome (proposition)
| Trait | Valeur | Source / statut |
|---|---|---|
| height | 20-25 m, >30 m rare ; record polonais 34,2 m | Atlas, WP pl (mesuré) |
| longevity | ~150 ans (de) ; <100 (fr) ; ~200 (pépiniériste) | divergence, estimée |
| growth | ~10 m en 20 ans, lent en futaie | WP fr (secondaire) |
| marcescence | oui : feuilles gardées tout l'hiver, tombent au débourrement | Atlas (mesuré) |
| budburst | doy 90 (31 mars), sous-étage RENECOFOR | mesuré ; le plus précoce des feuillus étudiés |
| end yellowing | doy 292 | RENECOFOR |
| flowering | mars-avril, anémophile | Atlas |
| shade | 3,97/5 ; fortement tolérant, un peu moins que le hêtre | Niinemets, Atlas |
| drought | 2,66/5 | Niinemets |
| waterlogging | 1,65/5 ; mais alternative au hêtre sur sols engorgés (Forest Research) | divergence |
| frost | très rustique, jusqu'à -30 °C | Atlas, WP de |
| wood | ~735-820 kg/m³ sec ; MOR 110 MPa, MOE 12,1 GPa (12 %) | Wood Database ; WP de 820 |
| roots | superficiel (35 cm en sol humide) → chablis ; tronc fluté | WP de (mesuré qualitatif) |
| mycorrhiza | ectomycorhizien (~25 basidiomycètes) | WP de |
| resprout | rejets de souche et drageons ; têtard ; taillis 20-35 ans | Atlas, Forest Research |
| first flowering | non trouvé | — |

## Particularités / moteur
- Marcescence totale chez les adultes : à traiter via `marcescence` et `leaf_fall` en fin de dormance.
- Rejet vigoureux : doit alimenter la réitération (rejets sur recépage), proche du tilleul.
- Non trouvé : hauteur-âge, houppier-DBH, MOR/MOE vert, âge de première fructification.
