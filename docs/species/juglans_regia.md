# Noyer commun — Juglans regia : fiche de traits pour le génome

Jeu de référence : `data/validation/juglans_regia.ron` (49 entrées). Les noyers fruitiers sont greffés (Franquette, Lara…) ; le noyer à bois est de semis. Légende : **mesuré** (id) ou **estimé**.

## Franc ou greffé
| Cas | Taille | Fruits | Longévité |
|---|---|---|---|
| Franc / noyer de semis | 25-35 m, DBH jusqu'à 2 m [atlas_*] ; 25-30 m [wal_height] ; 15-25 m en isolé [de_wiki_height] | maturité sexuelle 15-20 ans (Wallonie), 10-20 ans (de), 7-8 ans (PlantVillage, résumé de recherche) | 100-200 [atlas_longevity], 150-160 [fr/de wiki], 200-300 [wal_longevity] : divergence |
| Verger greffé | ~15 m de haut et de large en Californie [en_wiki_orchard_size] | 5-6 ans, pleine production à 25 ans, jusqu'à 70 ans [fr_wiki_grafted_*] | — |

## Traits du génome du merisier
- **Architecture** : tronc court, houppier très large par rapport au tronc (« larger than any other broadleaf »), feuilles composées de 5-9 folioles de 20-45 cm [atlas] : **mesuré**. Fin de croissance en hauteur à 60-80 ans [fr_wiki_height_stop]. Hallé-Oldeman : non trouvé. Floraison en chatons mâles pendants sur rameaux de l'année précédente, fleurs femelles terminales ; moelle cloisonnée [en wiki]. `apical_dominance`, angles : **estimé**.
- **Feuille** : composée imparipennée (rachis jusqu'à 40-45 cm), `lobes` à 0 pour le moteur actuel (folioles) ; marcescence 0 : **mesuré** (taille) / **estimé** (autres).
- **Phénologie** : débourrement tardif : « mi-mai à mi-octobre » de période de foliation en Wallonie (jours 135-288, approximés) [wal_leaf_on_*] ; Franquette 5 mai, G-139 23 avril (1984-1990), avance de 7 et 3 j ensuite [ishs_*] : **mesuré** ; floraison avril-juin [de_wiki_flowering, fr_wiki_flowering] ; stratification 16-24 semaines à 3 °C [wal_stratification]. Besoin en froid : non trouvé.
- **Gel** : jeunes pousses et inflorescences sensibles aux gelées tardives (S à TS), moins que le noyer noir ; gelée précoce très sensible pour les jeunes (-7 voire -2 °C) [wal_early_frost] ; adultes -30 °C [atlas_frost_old_trees] ; fiche CNPF : très tardif donc peu de gel de printemps (non extrait chiffré). Proposé : `flower_frost_c` ≈ -2 (**estimé**).
- **Bois** : 640 kg/m³ à 12 % (basale 520), MOR 111,5 MPa, MOE 10,81 GPa [wdb_walnut_*] ; 550-700 selon fr_wiki_density ; semi-poreux : **mesuré (compilation)**.
- **Racines** : pivot fort dès le jeune âge, qui disparaît avec l'âge ; 3-5 m, phrase ambiguë à relire [wal_root_depth] ; très sensible à l'anoxie et au tassement [wal_*]. Le pivot est sectionné à la transplantation.
- **Tolérances** : très héliophile, intolérant à la concurrence latérale ; sol riche pH 6-7,5, profond, frais ; ≥ 700 mm, optimum 1000-1200 mm [wal_rain] ; sécheresse peu sensible si sol profond.
- **Reproduction** : noix 4-5 cm, ≤ 18 g [atlas_nut_size] ; fructification tous les 1-2 ans [wal_fruiting_freq] ; rendement verger roumain 11-15 ans : 1,7 t/ha Franquette, soit ~12 kg/arbre (dérivé) [botu_*]. Fruit à 5 % de fleurs nouées non applicable.
- **Longévité** : 150-300 selon source, choisir 150-200 comme zone commune.
- **Symbiose** : non trouvée (Atlas muet) ; arbuscule supposé (**estimé**).
- **Taille et blessures** : le noyer « n'apprécie pas la taille » : ne tailler que le bois mort en fin d'automne, sinon forte saignée de sève [fr_wiki_pruning] (affirmation de Wikipedia) ; rejette de souche, surtout jeune [wal_stump]. Le moteur a `sap_bleeding` : monter haut. Compartimentation : non mesurée.
- **Juglone** : produite surtout dans racines et brou ; effets allélopathiques très contradictoires en nature (WSU, noyer noir) [wsu_juglone] : particularité à ne pas modéliser pour l'instant.
- **Maladies** : anthracnose, bactériose, armillaire, chancres (Wallonie) ; non modélisées.

## Moteur
Feuilles composées, floraison monoïque hétérodichogame, saignée de sève, pivot disparaissant avec l'âge, juglone, greffe sur noyer noir/hybride, gel de débourrement dépendant du cultivar.

## Non trouvé
Courbes hauteur/DBH-âge d'un noyer isolé en Bretagne, mycorhizes, besoin en froid, tailles du houppier mesurées, sexe-timing quantitatif.
