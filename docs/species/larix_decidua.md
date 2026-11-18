# Larix decidua (mélèze d'Europe) : fiche de traits pour le génome

2026-11-18 ; entrées dans `data/validation/larix_decidua.ron`. Seul conifère **caduc** d'Europe : c'est le seul résineux de cette série compatible avec le cycle feuille caduque du moteur, mais avec des aiguilles en touffes.

## Port et architecture
Tronc monopodial, droit ou courbé à la base sur pente (Atlas), cime conique claire, branches étagées retombantes aux rameaux pendants ; jeunes plants très flexibles, non abîmés par les avalanches (Atlas). Deux types de rameaux : **longs rameaux** (feuilles isolées, croissance de l'année) et **brachyblastes** (« rosettes » de 20-40 aiguilles) qui se maintiennent plusieurs années. Modèle de Rauh (**estimé**). Forte croissance en jeunesse (« fort potentiel de croissance », WP fr).

## Feuillage
| Trait | Valeur | Source |
|---|---|---|
| Aiguilles | 1,5-4 cm (Atlas), 2-4 (EN), 1-3 (DE) ; tendres, vert clair ; en touffes de 20-40 | [atlas_needle_length] [wen_needle_length] [wde_needle_length] |
| Caducité | jaunissent à l'automne puis tombent ; **aucune date chiffrée** de débourrement ou chute trouvée | [atlas_deciduous] |
| LMA, LAI | non trouvés (feuillage clair, LAI faible attendu : **estimé**) | |

## Reproduction
Cônes mâles soufrés 5-10 mm, femelles roses ; cônes mûrs de ~2,5 cm persistant jusqu'à 10 ans (Atlas) ; maturité 4-6 mois après la pollination (EN) [atlas_cone_length] [wen_cone_cycle]. Floraison mars-juillet selon l'altitude (WP fr) [wfr_cones_period]. **Première floraison** : 15-20 ans en plein découvert, 30-40 en peuplement (WP de) [wde_first_flowering_open] [wde_first_flowering_stand].

## Dimensions et longévité
45 m, rarement > 50 (Atlas), 30-40 (FR), jusqu'à 54 (DE) ; DBH 1-1,5 m (jusqu'à 2) ; durée 600-800 ans (Atlas), 600 (DE), ~1000 confirmés (EN), plus de 1000 en altitude [atlas_height] [atlas_dbh] [atlas_longevity] [wde_longevity] [wen_longevity]. Rotation 100-140 ans [wde_rotation]. Tables britanniques : DBH moyen YC 12 : 25 cm à 35 ans, 29 à 40, 32 à 45 ; YC 10 : 25 à 40, 28 à 45, 31 à 50 [fc34_el_yc12_dbh_35 etc.]. **Mélèze du Japon/hybride ou d'Europe, la table ne distingue que l'européen ici.**

## Racines
Système en cœur (WP de) [wde_root]. Pas de profondeur chiffrée. Écorce épaisse (feu, chute de pierres plus favorable que l'épicéa, Atlas). Ectomycorhizes (**estimé**).

## Bois
12 % : 575 kg/m3, MOR 90,0 MPa, MOE 11,80 GPa [wdb_density] [wdb_mor] [wdb_moe] ; bois le plus lourd et dur des résineux européens (WP de), très durable.

## Tolérances
Héliophile strict [atlas_shade] ; froid -30 °C en dormance [atlas_frost] ; pas d'engorgement [atlas_waterlogging] ; sols drainés. En plaine ou fortes pluies, plus de maladies fongiques (chancre) ; défoliations par la tordeuse du mélèze (Atlas). À Rennes, hors niche altitudinale (continentale).

## Mécanique et tempêtes
Essais de traction sur mélèze hybride (Larix europaea × japonica), 22 arbres : moment de rupture 6-24 kNm à 16-28° [andreozzi_larch] [andreozzi_larch_tilt]. ForestGALES paramètre le mélèze d'Europe ; les modèles ont surestimé les dégâts sur les peuplements de 50-100 ans [hale_larch]. Lothar/Klaus et fréquence propre : non trouvés.

## Taille
Pas de rejet de souche (**estimé**), mais il peut émettre des rameaux courts sur le vieux bois ; non sourcé.

## Moteur
Caduc avec brachyblastes en touffes (feuilles groupées à l'extrémité) ; aiguilles molles ; cônes persistants longtemps.

## Non trouvé
Dates de débourrement et de chute, hauteur par âge, LMA/LAI, profondeur racinaire, fréquence propre, dégâts de tempête par essence.
