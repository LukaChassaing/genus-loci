# Fagus sylvatica (hêtre commun) — fiche de traits

Jeu de référence : `data/validation/fagus_sylvatica.ron` (53 entrées). Statuts : **mesuré** = valeur publiée directement ; **estimé** = dérivé ou jugement ; **non trouvé**.

## Port et architecture
- Grand arbre monopodial, fût long et mince en futaie (dégagé jusqu'à 15-20 m), houppier étroit ; isolé : fût court, houppier large pouvant couvrir ~600 m² (WP fr) / 20-30 m de diamètre (WP de). Modèle de Hallé-Oldeman : « Troll » est la désignation courante dans la littérature, **non vérifiée dans cette recherche** (estimé).
- Hauteur maximale atteinte vers 100-150 ans en solitaire (WP de) ; croît vite très tard (Atlas).

## Traits du génome (proposition)
| Trait | Valeur proposée | Source / statut |
|---|---|---|
| height_max (peuplement) | 30-40 m, record 45-50 m | Atlas (mesuré, sans âge) ; WP fr 45,5 m |
| longevity / typical_lifespan | 150-300 ans ; record daté 546 ans (Kalkalpen), >500 (Cervara) | Atlas, WP de/en (mesuré) |
| first_flowering_age_years | 40-50 isolé ; 60-80 en peuplement dense | WP fr, Atlas 40-50 (mesuré) |
| mast_interval | 5-8 ans | Atlas, EUFORGEN (mesuré) |
| juvenile growth | 40-70 cm/an à 30-50 ans | WP fr/de (mesuré, source primaire non remontée) |
| budburst | doy 112 (dominants), 107 (sous-étage), RENECOFOR 1997-2013, France | mesuré (primaire) |
| end of yellowing | doy 294 / 298 | RENECOFOR (mesuré) |
| marcescence | oui, surtout jeunes sujets et bas de houppier | WP fr (qualitatif) |
| shade tolerance | 4,56/5 (Niinemets) ; « plus tolérante des feuillues » | mesuré |
| drought tolerance | 2,4/5 (±0,43) ; racines superficielles | mesuré |
| waterlogging | 1,02/5 : intolérant | mesuré |
| frost | gel tardif des jeunes et des fleurs | Atlas, Forest Research |
| wood density | 700-720 kg/m³ sec ; MOR 110 MPa, MOE 14,3 GPa (12 %) | Wood Database (secondaire) ; vert : **non trouvé** |
| roots | système « en cœur », superficiel à intermédiaire, variable selon le sol | WP fr, Atlas |
| mycorrhiza | ectomycorhizien, indispensable | WP fr (type non détaillé dans la page) |
| stump sprouting | très faible (quasi nul en plaine) | WP fr |
| decay | cœur rouge dès ~100 ans ; Ganoderma, Meripilus | Atlas, EUFORGEN |

## Particularités à modéliser
- Marcescence partielle ; débourrement piloté par la photopériode (WP fr), plus tardif que le chêne de ~11 jours (RENECOFOR) ; sensibilité thermique du débourrement faible (-2,1 j/°C, Vitasse 2009 via résumé).
- Écorce fine : sensible aux coups de soleil sur fût exposé après éclaircie.
- Rejette très mal : un moteur de rejets de souche ne doit pas en produire pour cette essence.
- Hêtre de Verzy (tortillard) : forme tordue, hors périmètre.

## Risques moteur / non trouvé
Hauteur-âge en peuplement (tables Schober) et largeur de houppier en fonction du DBH : non trouvées. Pas de valeur vert pour MOR/MOE. Sécheresse 2018-20 : dépérissement, non modélisé.
