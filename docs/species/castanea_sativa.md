# Castanea sativa (châtaignier) — fiche de traits

Jeu de référence : `data/validation/castanea_sativa.ron` (42 entrées).

## Port et architecture
- Grand arbre (30-35 m, 40 m max.), tronc épais, écorce profondément fissurée en spirale. En culture (vergers de fruitiers greffés) ≤18 m. Modèle architectural : non vérifié.
- Plante monoïque : les chatons mâles se développent d'abord, les chatons androgynes au sommet des pousses.

## Traits du génome (proposition)
| Trait | Valeur | Source / statut |
|---|---|---|
| height | 30-35 m ; 40 m max ; 20-35 m (en) | Atlas, EUFORGEN |
| dbh/girth | 150 cm DBH (EUFORGEN) ; tour jusqu'à 12 m (Atlas) | mesuré |
| longevity | 500-600 ans (WP), jusqu'à 1000 en culture ; en Europe centrale rarement >200 | divergence |
| first flowering | 20-30 ans ; fructification de semis 25-35 ans | WP de/en |
| budburst | avril à juin (large), tardif | EUFORGEN |
| flowering | fin juin-juillet (nord) ; mai-juin (Méditerranée) ; début ~1 j/40 m d'altitude | Atlas, EUFORGEN, Conedera 2007 |
| nut fall | septembre-octobre | Atlas |
| shade | 3,15/5 ; régénère en demi-ombre mais exige la lumière dès la perche | Niinemets, Atlas |
| drought | 3,46/5 (mais Atlas : « forte sensibilité aux sécheresses estivales ») | divergence |
| waterlogging | 1,32/5 | Niinemets |
| frost | sensible aux gels tardifs ; tolère -15 °C | Atlas, WP en |
| climate | T moyenne 8-15 °C ; >6 mois à >10 °C ; pluie min 600-800 mm | Atlas |
| soil | acide à neutre, pas de calcaire, pas de compactage | Atlas, WP en |
| wood | ~590-630 kg/m³ sec ; MOR 71 MPa ; MOE 8,6 GPa (12 %) ; tanins ~13 % | Wood Database, WP |
| resprout | rejette très vigoureusement : taillis 15-30 ans (40 en France) | Atlas, WP de |

## Particularités à modéliser
- Rejet de souche très vigoureux = ressource centrale (≈80 % de la surface en taillis).
- Maladie de l'encre (*Phytophthora cinnamomi*, *P. cambivora*) et chancre (*Cryphonectria parasitica*) : mortalité racinaire/de branche ; pas dans le moteur.
- Roulure (ring-shake) : fente circulaire du bois, défaut de qualité ; non modélisable.
- Fruits : greffe, autostérilité en culture.

## Non trouvé
Hauteur-âge/DBH-âge en taillis ou futaie, houppier, enracinement chiffré (profondeur), dates de débourrement précises en France/Bretagne. Châtaignier présent en Bretagne (toponymes ; WP fr) mais pas de donnée de croissance locale.
