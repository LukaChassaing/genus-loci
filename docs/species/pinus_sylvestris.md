# Pinus sylvestris (pin sylvestre) : fiche de traits pour le génome

2026-11-18 ; entrées dans `data/validation/pinus_sylvestris.ron`. Statuts : mesuré, Wikipédia, estimé, calibrer.

## Port et architecture
- Persistant, pionnier héliophile. Modèle de Rauh (verticilles annuels), **estimé** (connaissance générale). Pousse annuelle monocyclique en général (un seul verticille de branches par an), écorce rouge-orangé en haut de la tige, écaillée et fissurée en bas (Atlas).
- Forme très variable selon la provenance et la station ; cime conique chez le jeune, **élargie et plate à l'âge adulte** quand le pin est isolé ; élagage naturel net en peuplement dense (les branches basses meurent par manque de lumière) : **estimé**, pas de chiffre.

## Feuillage
| Trait | Valeur | Source | Statut |
|---|---|---|---|
| Aiguilles | par paires, 5-7 cm (Atlas), 4-7 (fr, de), 2,5-5 (en) | [atlas_needle_length] [wfr_needle_length] [wen_needle_length] | mesuré / Wikipédia |
| Persistance | 2-6 ans (Atlas) ; 3 ans courant, 2 en zone polluée (de) ; 2-4 ans, jusqu'à 9 en subarctique (en) | [atlas_needle_longevity] [wde_needle_longevity] [wen_needle_longevity] | divergences conservées |
| LMA/LAI | non trouvés | | manquant |

## Phénologie et reproduction
Floraison avril-mai (WP de) [wde_flowering]. **Cycle des cônes de deux ans** : fécondation au 2e été, maturité en novembre de la 2e année, graines libérées au printemps 22-24 mois après la pollinisation [wen_cone_cycle] [wde_cone_maturity]. Première floraison dès 10-15 ans (WP de) [wde_first_flowering], très précoce. Cônes 3-8 cm. Besoin de froid hivernal [atlas_chilling].

## Dimensions
Hauteur moyenne 23-27 m, plus de 40 m exceptionnellement, record 46,6 m (Estonie), 48 m (DE) [atlas_height] [wen_height_record] [wde_height] ; DBH ~1 m ; longévité 150-300 ans, 400+ (Atlas), 600 (DE), >750 (Laponie) [atlas_longevity] [wde_longevity]. Tables de production britanniques (Yield Class 12-14) : DBH moyen de 20-23 cm à 40 ans, 27-31 à 50 ans, 33-38 à 60 ans [fc34_sp_yc14_dbh_40 à _60, fc34_sp_yc12_dbh_*]. Rotation 80-140 ans [wde_rotation]. Accroissement des vieux peuplements bavarois 7-8 m3/ha/an [klemmt_increment]. Pas de hauteur par âge ni arbre isolé trouvés.

## Racines
Mesures finlandaises sur arbres excavés : profondeur maximale **2,91 m**, extension 9,5 m, 80 % de la biomasse de grosses racines dans les 30 premiers cm [kall_depth] [kall_spread] [kall_crb30]. Pivot décrit jusqu'à ~6 m (WP de) [wde_root]. Mycorhizes ectomycorhiziennes (**estimé**, non sourcé ici).

## Bois
12 % : 550 kg/m3, MOR 83,3 MPa, MOE 10,08 GPa [wdb_density] [wdb_mor] [wdb_moe] ; densité « sèche » 470 kg/m3 (EN) [wen_dry_density]. Pas de valeurs vertes. Résine : canaux résinifères, exsudation en cas de blessure ; gemmage possible (Atlas).

## Tolérances
Héliophile (« sunny to partially shaded », Landolt L=4 selon WP de) [atlas_shade] ; très tolérant à la sécheresse et au gel, pauvres sols [atlas_drought] ; sensible aux embruns salés et à la pollution. Insecte et champignons : Diprion, Hylobius, Dothistroma, Ips (Atlas).

## Mécanique et tempêtes
Finlande 2001-2002 : le pin fait 78,8 % des arbres endommagés (essence dominante, **pas un classement de sensibilité**) ; 44-58 % déracinés, le reste cassé [zub_share] [zub_uproot]. ForestGALES paramètre le pin sylvestre ; aucune valeur de coefficient d'ancrage ni de fréquence propre extraite. Lothar/Klaus par essence : non trouvé.

## Taille
Ne rejette pas de souche ni de la partie sans aiguilles de la branche (**estimé**) ; une branche sans aiguilles ne repart pas.

## Moteur
Persistant (cohorte de 3 ans en général), aiguilles par paires sur brachyblastes, pousse monocyclique, cônes bisannuels, floraison précoce, résine.

## Non trouvé
Table hauteur-âge (Wiedemann 1943 non numérisée), arbre isolé, LMA/LAI, débourrement chiffré, MOR vert.
