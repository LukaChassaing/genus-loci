# Abies alba (sapin pectiné) : fiche de traits pour le génome

Rédigée le 2026-11-18 ; entrées chiffrées dans `data/validation/abies_alba.ron` (ids entre crochets). Statuts : mesuré (source lue), Wikipédia, estimé (connaissance générale non vérifiée ici), calibrer.

## Port et architecture
- Conifère persistant, tige monopodiale très droite, cylindrique (« vollholzig », Utschig 2004), branches en étages à contrôle apical fort ; cime conique chez le jeune, qui devient un dôme arrondi (« nid de cigogne ») chez le vieux (Atlas : « conical shape gradually changing to a rounded dome »). Modèle de Rauh pour le gabarit général (**estimé**) ; certains *Abies* suivent le modèle de Massart (branches plagiotropes à rythme régulier), non vérifié ici.
- Croissance **très lente dans la jeunesse** (survit des décennies sous couvert), puis rapide à la libération, accroissement qui culmine tard et reste élevé (Utschig). Le sapin a plus de réactivité à la libération que toute autre essence européenne (arbre supprimé > 200 ans, cernes < 0,5 mm, repart à ~1 mm) [utschig_ring].
- Gourmands (« Wasserreiser ») sur plus de 50 % des sapins en peuplement dense [utschig_epicormic] : cime qui se reconstitue.
- Bourgeons non résineux (Atlas) ; cônes dressés qui se désagrègent sur l'arbre (ne tombent pas entiers).

## Feuillage
| Trait | Valeur | Source | Statut |
|---|---|---|---|
| Aiguille | 1,8-3,0 cm (EN), 1,5-3 (FR), jusqu'à 3 cm × 3 mm (DE) ; deux bandes stomatiques blanches dessous | [wen_needle_length] [wfr_needle_length] | Wikipédia |
| Persistance | **6-8 ans** (Atlas, WP fr) contre **8-12 ans** (WP de) : divergence conservée | [atlas_needle_longevity] [wde_needle_longevity] | mesuré / Wikipédia |
| LMA, LAI | non trouvés | n/d | manquant |
Les aiguilles d'ombre sont plates et pectinées sur deux rangs ; très tolérant à l'ombre (extrêmement, WP de) [wde_shade].

## Phénologie et reproduction
- Floraison avril-mai (Atlas, WP fr, pl), avril-juin (WP de) [atlas_flowering]. Débourrement **précoce**, donc sensible aux gelées tardives (WP fr ; Atlas : gelées de printemps, dessiccation) [atlas_frost].
- Première floraison : 30-40 ans (Atlas) ; 25-35 ans en plein découvert et 60-70 ans en peuplement (WP de) [wde_first_flowering_open] [wde_first_flowering_stand]. Cônes 9-20 cm, dressés, mûrs la première année (**estimé**, non lu).
- Aucune date de débourrement chiffrée (jour de l'année) trouvée.

## Dimensions et croissance
- Hauteur adulte 40-50 m (WP en), 30-50 m jusqu'à 65 (WP de), plus de 60 m (Atlas) ; DBH jusqu'à 1,5-2 m, record 3,8 m [wde_height] [wen_height] [atlas_height] [wde_dbh].
- Comparaison à l'épicéa sur même station : le sapin est 4-5 m plus bas au même âge, 30 m à 100 ans quand l'épicéa fait ~34 m (Bavière du Sud) ; stock 600-700 m3/ha à 100 ans contre ~1000 pour l'épicéa mais accroissement supérieur à partir de 50 ans [utschig_h100] [utschig_gap] [utschig_vol] [utschig_incr].
- Production : 16 m3/ha/an en Allemagne, jusqu'à 26 sur bonnes stations [wde_mai]. Âge : > 500 ans (WP de), 500-600 ans (Atlas), 220 ans dans le peuplement de Garmisch étudié [atlas_longevity] [wde_longevity] [utschig_age].
- Pas de données pour l'arbre isolé.

## Racines
**Pivot profond**, d'où une bonne tenue au vent : environ 1,50 m en Allemagne et près de 3 m dans les parties chaudes de l'Autriche [wde_taproot], « tiefreichende Pfahlwurzel » donnant une essence très stable contre le chablis [utschig_taproot]. Besoin de sols profonds, frais, non compacts ni hydromorphes [atlas_waterlogging]. Mycorhizes : ectomycorhizes (**estimé**, non lu ici).

## Bois
Densité à 12 % : 415 kg/m3, MOR sec 66,1 MPa, MOE sec 8,28 GPa (Wood Database) [wdb_density] [wdb_mor] [wdb_moe] ; densité anhydre moyenne 410 kg/m3 (WP de) [wde_dry_density]. Pas de valeurs vertes trouvées (estimer comme l'épicéa : ~800 kg/m3 vert, à vérifier).

## Tolérances
Ombre : extrême (le sapin et l'if sont les plus tolérants d'Europe, WP de) ; humidité : exige 700-1800 mm de pluie et des étés frais (14-19 °C) [atlas_precip] [atlas_summer_temp] : **à Rennes (~700 mm, étés chauds) le sapin est en marge de sa niche**, stress hydrique attendu ; gel : plantules très sensibles ; très sensible au feu et au SO2 [atlas_frost]. Fortement abrouti par le gibier (Atlas).

## Mécanique et tempêtes
Essence considérée comme peu vulnérable au chablis grâce au pivot [utschig_taproot] ; aucune donnée de Lothar 1999 ou Klaus 2009 propre au sapin, ni de coefficient d'ancrage ForestGALES (le sapin n'est pas parmi les espèces paramétrées), ni de fréquence propre, trouvée dans cette recherche. Armillaria et Heterobasidion provoquent des pourritures de pied et des chablis (Atlas).

## Taille
Pas de rejet de souche ; pas de rejet après étêtage (**estimé**), mais gourmands sur le fût [utschig_epicormic].

## Moteur
Persistant, cohorte d'aiguilles de 6-12 ans ; étages de branches ; forte réactivité à la libération (modèle de croissance qui accepte un long état supprimé) ; pivot profond à garder dans le modèle racinaire ; bourgeons non résineux.

## Non trouvé
Hauteurs d'arbre isolé par âge ; table de production Hausser en chiffres ; date de débourrement ; LMA/LAI ; densité et MOR verts ; dégâts de tempête par essence en France.
