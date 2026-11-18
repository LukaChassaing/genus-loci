# Pinus pinaster (pin maritime) : fiche de traits pour le génome

2026-11-18 ; entrées dans `data/validation/pinus_pinaster.ron`. Statuts : mesuré, Wikipédia, estimé, calibrer. Rennes est à la limite nord de l'aire de plantation (Landes, Gascogne, Bretagne côtière).

## Port et architecture
- Persistant, héliophile strict (« cannot tolerate shade », Atlas) [atlas_shade]. Cime ovoïde à conique chez le jeune, **irrégulière et ouverte chez l'adulte**, branches garnies à leurs extrémités (Atlas). Modèle de Rauh (**estimé**).
- **Pousse polycyclique** : l'axe principal peut faire un ou plusieurs flush(es) par saison (WP de) [wde_polycyclic] ; il en résulte plusieurs verticilles par an (non dénombrés ici) et des « faux-verticilles ». Une seule source (WP de) : à confirmer avant de coder.
- Pivot fort et latérales faibles : arbres penchés, fûts en sabre sur sol mince (Forest Research) [atlas_root] [fruk_shade].
- Élagage artificiel recommandé en Landes (3 m sur 600 tiges/ha à 40 cm de circonférence, puis 5,5 m sur 400 tiges/ha à 55 cm, WP fr).

## Feuillage
Aiguilles par paires, **10-25 cm**, les plus longues des pins européens, 2-3 ans (Atlas) ou 2-4 ans (WP de) [atlas_needle_length] [atlas_needle_longevity] [wde_needle_longevity]. Résine abondante (canaux résinifères nombreux, 2-9 dans l'aiguille, WP de). LMA, LAI : non trouvés.

## Phénologie et reproduction
Floraison avril-mai (WP fr), pollen « pluie de soufre » surtout en avril, fécondation en juin de la 2e année (WP de) [wde_pollen] [wfr_flowering]. Cônes 8-22 cm, mûrs 2 ans après la pollinisation, s'ouvrent l'été même ou jusqu'à 10 ans plus tard : **sérotine en zone d'incendie** (Atlas) [atlas_cone_cycle]. Production de cônes dès 6-8 ans après semis (WP fr) [wfr_first_cones].

## Dimensions et production
20-30 m (Atlas, 40 exceptionnellement), 15-40 m (DE), 20-35 m (EN) ; DBH ≤ 1 m (DE), 1,2 m (EN) [atlas_height] [wde_height] [wen_height] ; longévité 300 (DE) à 500 ans (WP fr) : peu fiable. **Production landaise** : 4,8 m3/ha/an en 1960, 15 en 2015 (WP fr), massif moyen 10 (fiche Fibois) ; hauteur dominante potentielle à 40 ans 25-30 m ; rotation 40-50 ans [wfr_mai_1960] [wfr_mai_2015] [fibois_hdom40] [fibois_prod] [wfr_rotation].

## Racines
Pivot profond avec secondaires développées (Atlas) ; « d'abord plongeant puis traçant » (WP fr) [atlas_root] [wfr_root]. Pas de profondeur chiffrée. Ectomycorhizes **inférées** (bolets, lactaires) [atlas_mycorrhiza].

## Bois
12 % : 500 kg/m3, MOR 73,0 MPa, MOE 8,54 GPa (Wood Database) ; r12 0,45-0,79 g/cm3 selon WP de [wdb_density] [wdb_mor] [wdb_moe] [wde_density]. Résine : gemmage historique (10-12 livres de colophane par an et par arbre, WP fr, source de 1836).

## Tolérances
Climat : > 600 mm de pluie (survit jusqu'à 400 mm), **gel : ne supporte pas moins de -18 °C** (Forest Research) [atlas_precip] [fruk_frost] ; sol siliceux et sableux, intolérant au calcaire, à la tourbe et au gley ; tolère les embruns salés. Pyrophyte : l'écorce épaisse et les cônes sérotines (Atlas).

## Mécanique et tempêtes
Klaus 2009 et Martin 1999 ont abattu ensemble environ 75 millions de m3 de bois en France (toutes essences, ~10 ans de récolte)[wfr_klaus] (aucune répartition par essence dans la source lue). Pas de coefficient ForestGALES (le pin maritime ne figure pas dans la liste des espèces du manuel ForestGALES 2.5) ; Gardiner et al. ont modélisé le risque de chablis du pin maritime (article repéré, non lu). Fréquence propre : non trouvée.

## Taille
Ne rejette pas de souche ; élagage des branches vertes jusqu'à une hauteur limitée pour éviter la perte de croissance (**estimé**). Plaies résineuses.

## Moteur
Pousse polycyclique (plusieurs flushes), aiguilles longues et groupées en paires, cônes sérotines (peu utile visuellement), résine, héliophilie stricte, pivot profond, sol sableux et nappe.

## Non trouvé
Hauteur par âge sur station landaise (tables IGN/CNPF), LMA, LAI, profondeur du pivot, nombre de verticilles par an, dégâts de Klaus par essence.
