# Jeu de référence : *Prunus avium* (merisier, cerisier des oiseaux)

Fichier de données : `data/validation/prunus_avium.ron` (169 entrées, format de `data/validation/FORMAT.md`).
Date de consultation de toutes les sources : 2026-11-17. Le travail est une confrontation documentaire
indépendante du génome (`data/species/prunus_avium.ron`) : le moteur n'a pas été exécuté.

Portée : l'arbre forestier (merisier, *P. avium* subsp. *avium*). Les cultivars de cerisier doux greffés ne servent
que lorsque la donnée n'existe pas ailleurs, et sont alors signalés (entrées `wpfr_fruit_cultivars`, `wpes_chill`,
`heatpulse_transp`).

## 1. Ce qui a été réellement lu

| Source | Lue comment | Entrées |
|---|---|---|
| Pryor 1988, Forestry Commission Bulletin 75 (étude de >40 peuplements britanniques) | PDF complet (OCR), tables 1-3, 6a, 6b, 7, texte | 52 |
| Pretzsch, Nickel & Dietz 2010, LWF Wissen 65 (essais de Bavière, TU München) | PDF complet | 17 |
| Albrecht 2010, LWF Wissen 65 (sylviculture, Franconie) | PDF complet | 13 |
| Aas 2010 (Steckbrief), Blaschke & Nannig 2010, LWF Wissen 65 | PDF complet | 6 (Aas 5, Blaschke 1) |
| Jeske & Grosser 2010, LWF Wissen 65 (bois ; valeurs DIN 68364) | PDF, tables 1 et 2 | 6 |
| CIRAD TROPIX 7, fiche Merisier | PDF, tableau de propriétés | 6 |
| Welk, de Rigo & Caudullo 2016, Atlas européen des essences forestières | PDF complet (2 pages) | 14 |
| Russell 2003, EUFORGEN Technical Guidelines | traduction italienne officielle, PDF complet | 8 |
| Stojecová & Kupka 2009, J. For. Sci. | PDF complet | 8 |
| Nocetti et al. 2012, BioResources | PDF, texte et corrélations (pas les moyennes) | 2 |
| Forest Research (UK) fiche espèce (extraction automatique, `snippet`, 2) ; Tharaga et al. 2023, Frontiers in Horticulture (table 4 lue, `table`, 1) | | 3 |
| Wikipédia fr, en, de, nl, pl, it, es, sv, cs | extraction automatique de page (`read: via_wikipedia`) | 34 |

Parts du fichier (169 entrées) : 72 `primary`, 38 `review`, 14 `handbook`, 11 `secondary` (chiffres cités de seconde main par un
document lu : Thill 1980, Erlbeck 1998, Gordon & Rowe 1982, etc.), 34 `wikipedia`. Par domaine : croissance 67, phénologie et
âge de floraison 21, bois et écorce 18, longévité et pourriture 17, tolérances et station 17, cicatrisation/élagage/gomme 13,
racines et drageons 10, cernes 3, mycorhizes 2, eau 1.

Non obtenu malgré des essais : monographie Future Trees Trust (Leslie 2024 : 404), texte intégral de Schindler et al. 2023
(Ann. For. Sci., accès bloqué), Spiecker 1994 / Spiecker et al. 2009, tables de Röös 1994 (cités seulement), fiche
CNPF/ministère « Prunus avium conseils d'utilisation » (non lue), Wagenführ *Holzatlas*, Farmer 1972, PEP725 (aucun jeu de
dates propre à *P. avium* sauvage), article DWD (cité par le génome, non relu ici).

## 2. Synthèse par domaine

### 2.1 Croissance en hauteur, diamètre, houppier

Les sources divergent d'abord par les conditions (peuplement dense, arbre libre, station).

| Observable | Valeur | Conditions | Source |
|---|---|---|---|
| Hauteur dominante | 12.5 / 21.0 / 24.1 / 24.7 m à 20 / 40 / 60 / 80 ans | Grande-Bretagne, station « bonne », peuplement | Pryor, table 6a |
| Hauteur dominante | 11.5 / 17.5 / 20.2 / 20.6 m à 25 / 40 / 60 / 75 ans | station « pauvre » | Pryor, table 6b |
| Hauteur dominante, croissance libre | 9.2 / 16.3 / 20.5 / 21.7 m à 15 / 40 / 60 / 70 ans | arbres isolés ; modèle préliminaire (92 arbres) | Pryor, table 7 |
| DBH, croissance libre | 14 / 30 / 44 / 50 cm à 15 / 40 / 60 / 70 ans | idem | Pryor, table 7 |
| DBH moyen du peuplement (station bonne) | 22 / 34 / 49 cm à 40 / 60 / 80 ans | peuplement dense, éclaircie légère | Pryor, table 6a |
| Hauteur dominante à 15 ans | 14.1-15.2 m (lœss frais) à ~8 m (station sèche, à 33 ans : 7.1-9.5 m) | Bavière | Pretzsch et al. |
| Hauteur plafond sous forte mise en lumière | ~25 m | meilleures stations bavaroises | Pretzsch et al. |
| DBH cible | 50-60 cm en 60-80 ans (houppier solitaire) | Franconie, sylviculture « Lichtwuchs » | Albrecht |
| Maturité | 60-80 ans, 20-25 m, 50-70 cm | Europe | EUFORGEN |
| Hauteur / diamètre extrêmes | 15-32 m ; jusqu'à 35 m ; DBH 90-120 cm (max > 120) | Europe | Atlas, EUFORGEN |
| Peuplement mixte réel | 59 ans : H 22.4 m, DBH 25.8 cm (tous arbres), houppier 5.1 m (dominants), accroissement 2 mm/an | Bohême, 350 m | Stojecová & Kupka |
| Largeur de houppier | Dk = 1.19 + 0.158 DBH (R2 0.88) : 5.1 m à DBH 25 cm, 9.1 m à 50 cm | Grande-Bretagne | Pryor, fig. 1 |
| Volume de tige | 0.67 m3 (DBH 30, grume 12 m) ; 1.33 m3 (DBH 40, 14 m) | Grande-Bretagne | Pryor, table 2 |
| Production | MAI max 9.1 (bonne) / 7.9 (pauvre) m3/ha/an ; 10.3-12.7 à 15 ans sur lœss bavarois | | Pryor ; Pretzsch |

Points saillants :

- La croissance dépend très fortement de la station (eau utile, bases) : à 15 ans, 8 m contre 15 m de hauteur dominante
  (Pretzsch). Pryor trouve en revanche un diamètre peu dépendant de la fertilité mais fort dépendant de la densité.
- La croissance en hauteur s'arrête presque après 50-60 ans (Pryor), et après la jeunesse sous faible concurrence
  (Pretzsch : plafond ~25 m). Culminer entre 15 et 25 ans (Albrecht).
- Les sources de peuplement (Stojecová : 25.8 cm à 59 ans) et d'arbre libre (Pryor : 44 cm à 60 ans) donnent des DBH très
  différents : le moteur doit être comparé à la condition correspondante, pas à une seule valeur.
- Largeur de houppier : la régression de Pryor donne un houppier d'environ 0.4 x la hauteur aux diamètres que le génome
  produit (voir § 4). Stojecová confirme la valeur de ~5 m pour un DBH de ~25 cm.
- Pousses annuelles de 25 à 100 cm selon la station ; > 1 m dès la 3e année chez les plants vigoureux (Albrecht).

### 2.2 Cernes

Cernes moyens de 4-5 mm visés sur arbres d'élite (Albrecht), cernes étroits (borne basse 1.5 mm, borne haute illisible à
l'OCR) souhaités pour le placage (Pryor). Plantations clonales italiennes de 20 ans : déclin net de la largeur de
cerne à partir de l'âge cambial 4-5 ans ; densité de cerne indépendante de la largeur sauf cernes < 3 mm (Nocetti).
Aucun jeu de largeur de cerne par âge n'a été trouvé en accès libre ; on peut seulement dériver des moyennes (rayon/âge) des
tables de Pryor : 4.7 mm/an jusqu'à 15 ans en croissance libre, 3.8 mm/an jusqu'à 40 ans.

### 2.3 Racines, drageons

- Profondeur : ~3 m sur sols faciles à pénétrer, couches superficielles sur sol peu profond (Erlbeck et al. 1998, cité par
  Stojecová & Kupka, seconde main). Pryor : « superficially rooting » mais croissance faible dès que la roche mère se
  trouve à moins de 40 cm ; jamais de merisier sur sol gleyifié dans les 50 cm supérieurs.
- Architecture : « Herzwurzelsystem » (racine en coeur) étalé, peu profond, peu de racines fines actives, nombreuses
  racines latérales moyennes (Albrecht ; Wikipédia de ; Atlas « comparatively shallow-rooting »).
- Drageons : jusqu'à 10 m du parent (Pryor), jusqu'à 15 m (Albrecht, citant Schmid 2006 et Wilhelm 1993). Divergence
  qualitative : « renowned for its capacity to produce sucker shoots » (Pryor), mais drageons décevants après abattage
  (Pryor) et « moins de rejets de racines que les cerisiers acide et à grappes » (Aas).
- Rapport racines/pousses, étalement latéral chiffré, masse de racines fines : non trouvés.

### 2.4 Phénologie

Peu de chiffres pour l'arbre forestier en Europe de l'Ouest. Floraison avril-début mai, juste avant le débourrement
(Aas ; Wikipédias fr, de, nl, cs, pl), « fin mars à mai » (Atlas), durée d'~1 semaine par arbre (Atlas). Fruits du merisier
sauvage mûrs en juillet (Aas ; cs), juin-juillet (de), principalement juin (it), mai à fin juillet (pl, formes mêlées). Les
variétés greffées mûrissent de fin mai à juillet (fr). Gel tardif sur la floraison (Albrecht, EUFORGEN). Débourrement et chute
des feuilles chiffrés (jour de l'année) : rien d'exploitable trouvé ; les recherches Nature's Calendar / PEP725 n'ont donné
que des tendances (floraison du cerisier avancée de ~0.06 jour/an, résumé de recherche non vérifié) et un essai
commun européen 2026 (écart de ~8 jours entre provenances au débourrement, résumé seulement, sans jour de l'année).

### 2.5 Feuilles, eau, biomasse

- Dimensions des feuilles : 7-15 x 3-8 cm, pétiole 2-5 cm (Aas ; Atlas ; Wikipédias). Ni LAI ni LMA de merisier forestier
  trouvés ; seul un LAI-cible de verger de cerisier doux (4-5) apparaît dans une recherche, non retenu dans le RON.
- Eau : un seul jeu exploitable, pour un verger de cerisier doux sud-africain, cv. Bing (Tharaga et al. 2023, table 4 : 1-7.6 l/jour
  par arbre selon le stade, flux de sève sur 2 arbres) ; ce n'est pas le merisier forestier. Une étude de flux de sève sur plantation de merisier (Molina et al. 2019, DOI 10.1016/j.agwat.2019.03.019) existe
  mais n'a pas été lue.
- Biomasse : Schindler et al. 2023 (Ann. For. Sci. 80:28, 70 arbres en agroforesterie, équations en DBH avec R2adj >= 0.97)
  existe mais seul le résumé a été vu : aucune valeur reportée. À lire en priorité si on veut valider la biomasse aérienne.
- Volume et surface terrière de peuplement : tables de Pryor (310 m3/ha, G 28 m2/ha, 310 tiges/ha à 60 ans, station bonne).

### 2.6 Bois

| Observable | Valeur | Source (état) |
|---|---|---|
| Densité à 12 % | 0.60 (CIRAD) ; 600 kg/m3 (Pryor, d'après Farmer 1972) ; 0.57 [0.49-0.67] (DIN 68364 via Jeske) | sec, pas basale, pas verte |
| MOR | 95 MPa (CIRAD) ; 85-110 (DIN) | à 12 % (DIN : humidité non précisée dans l'article) |
| MOE | 10.2 GPa (CIRAD) ; 10-11 GPa (DIN) | à 12 % |
| Compression axiale | 50 (CIRAD) ; 45-55 MPa (DIN) | |
| Traction axiale | 98 MPa (DIN) | |
| Retrait tangentiel / radial | 8.4 % / 5.1 % (CIRAD) | |
| Aubier | 2-6 cm (CIRAD) ; 2.5-5 cm (Jeske) ; < 20 % du diamètre (Pryor) ; ~10 cernes (Nocetti) | |
| Bois | semi-poreux : vaisseaux de 60-80 µm presque en cercle continu au bois initial, limite de cerne nette | Jeske |
| Écorce | double épaisseur = 0.367 + 0.0353 d (cm) ; 5.5 mm à DBH 20, 9 mm à DBH 40 | Pryor |

**Valeurs vertes de *P. avium* : aucun équivalent publié trouvé** (ni densité verte, ni MOR/MOE verts, ni humidité du bois
vert). Le génome emprunte à *P. serotina* (Wood Handbook FPL-GTR-190, tableau 5-3a) : vert, SG 0.47, MOR 55 MPa, MOE 9.0 GPa ;
à 12 % : SG 0.50, MOR 85, MOE 10.3 (valeurs de ce tableau lues par résumé de recherche, non vérifiées sur le PDF). Les valeurs
à 12 % de *P. avium* (MOR 85-110, MOE 10-11 GPa) sont du même ordre : le proxy est plausible, sans qu'on puisse le montrer pour le
bois vert, qui est ce que le moteur utilise. Pryor précise que le merisier est « slightly weaker than beech ».
Nocetti : la densité n'est pas liée au taux de croissance (corrélation nulle avec la largeur de cerne), et la densité
moyenne des jeunes cernes (7-8 ans) prédit celle d'un arbre de 19 ans (r > 0.9).

### 2.7 Mécanique et tempête

Aucune valeur de P. avium sur la fréquence propre, l'amortissement, le coefficient de traînée, le moment d'ancrage ou la
vitesse critique n'a été trouvée. Indices qualitatifs : risque de chablis « relativement élevé » dû au système racinaire plat
et à la pourriture racinaire précoce (Albrecht) ; arbres inclinés et chablis fréquents sur sol peu profond même en site
modérément exposé, croissance médiocre sur sites exposés (Pryor) ; 18 merisiers sur 34 avec pourriture dans un
peuplement chablis du Buckinghamshire (Pryor) ; peuplements purs de plus de 0.5 ha déconseillés (Albrecht). Pour comparaison,
Canham et al. (2001) classe *P. serotina* parmi les essences les plus sujettes au chablis à DBH 40 cm (résultat de recherche,
non lu).

### 2.8 Longévité, rotation, pourriture

| Source | Valeur |
|---|---|
| Atlas | 100-150 ans (« short-lived »), puis « 100-120 ans au maximum » (contradiction interne) |
| EUFORGEN | 70-100 ans |
| Aas | environ 100 ans (maximum) |
| Wikipédia fr / it / pl / de | ~100 / ~100 / 80-100 / 80-90 typique, **300 maximum** |
| Stojecová | sénescence attendue à 80-90 ans |
| Albrecht | pourriture racinaire dès 50 ans, du tronc régulière dès ~80 ans |
| Pretzsch | pourriture centrale à partir de ~70 ans |
| Pryor | rotation de 65-75 ans (55-60 en croissance libre) ; 15-20 % des tiges vues par les acheteurs avec pourriture ; Belgique : 39 % de > 300 arbres (Thill 1980, seconde main) |
| Forest Research | rotation <= ~60 ans |

### 2.9 Tolérances

Jeune plant très tolérant à l'ombre (aucune baisse de croissance en hauteur sous un chêne de G = 18 m2/ha, 2 saisons ; Pryor),
mais essence pionnière de plus en plus héliophile : Albrecht donne ~10 ans comme âge limite de tolérance (source d'âge que le
génome indique ne pas avoir). Adulte : « light demanding » (Atlas, EUFORGEN). Sécheresse : sensible (Atlas, Pretzsch :
7-9.5 m à 33 ans sur station extrêmement sèche) mais supporte la sécheresse sur sols riches en bases (Albrecht). Engorgement :
non tolérant (Pryor, Atlas, EUFORGEN). pH 5.5-8.5 (EUFORGEN) ; pH 4-7 observé (Pryor). Altitude : rare au-dessus de 300 m en
Grande-Bretagne, 1100-1700 m dans les Alpes, 1900 m dans le sud-est de la France.

### 2.10 Cicatrisation, élagage, gomme, rejets

- Pretzsch (tomographie sur essais bavarois) : plaie recouverte en 2 ans -> aucune pourriture ; en 3 ans -> 10-40 % ; en 4 ans
  -> 30-90 %. Diamètre maximal de la branche dont la plaie se referme en 4 ans : 10 mm (arbres lents, stations pauvres), 30 mm (moyen),
  70 mm (arbres rapides, bonnes stations). La vitesse de recouvrement suit l'accroissement diamétral. (Cette métrique n'est pas
  `wound_closure_mm_yr` : observable ajouté.)
- Saison d'élagage : juin/juillet (fruits mûrs) ou fin d'hiver (Pretzsch) ; juin-août (Pryor, canker) ; mai à mi-juillet (plomb).
  Branches < 3 cm (Pryor, Albrecht, Forest Research).
- Gourmands : peu fréquents sauf après forte éclaircie d'un peuplement trop dense (Pryor) ; « immer » après mise en lumière
  brutale (Albrecht).
- Branches mortes : persistantes (Pryor ; Albrecht : « Totasterhalter »).
- Gomme : chancre bactérien *Pseudomonas syringae* (Pryor, Albrecht), gomme ambrée des blessures (Atlas).
- Rejets de souche : divergence : bonne capacité (Thill, Beck via Pryor ; Atlas) contre souches et rejets rares sur le terrain
  (Pryor) et « does not coppice well » (Forest Research).

### 2.11 Mycorhizes

Arbusculaires (endomycorhizes) : Wikipédia de (Düll & Kutzelnigg 2011) et Blaschke & Nannig 2010. Aucun taux de colonisation
forestier trouvé (les études de colonisation trouvées portent sur des plants micropropagés ou des vergers).

## 3. Apport de chaque Wikipédia

| Langue | Page | Apport chiffré |
|---|---|---|
| fr | Merisier | H 15-25 m, D 0.60 m, ~100 ans (Rameau, *Flore forestière française*) ; floraison avril-mai ; fruits des variétés fin mai-juillet |
| en | *Prunus avium* | H 5-25 m, D jusqu'à 1.5 m (Turner & Kuhlmann 2014, Amérique du Nord-Ouest) ; feuilles 7-14 x 4-7 cm ; peu de physiologie |
| de | Vogel-Kirsche | la plus riche : H 15-20 (30) m, D > 60 cm, 80-90 ans typique et **300 ans max**, floraison **20-25 ans**, mycorhize VA, drageons « meterweit » (Düll & Kutzelnigg 2011) |
| nl | Zoete kers | H ~20 m, floraison avril-mai ; sol mésotrophe |
| pl | Wiśnia ptasia | H 20-25 m en peuplement dense, 80-100 ans, plus gros arbre du pays (circonférence 356 cm, H 22 m) |
| it | *Prunus avium* | H 15-32 m, ~100 ans, fruits principalement juin ; datation Bronze (Lavagnone) |
| es | *Prunus avium* | H jusqu'à 30 m ; froid hivernal 500-1300 h (variétés, Sozzi 2008) |
| sv | Sötkörsbär | H 15-30 m seulement |
| cs | Třešeň ptačí | H jusqu'à 35 m (Slavík 1992), fruits en juillet |

Les Wikipédias n'apportent presque rien en physiologie, bois, racines, cernes ; ce sont les articles de synthèse allemands,
britanniques et européens qui portent l'information. Divergences utiles : 35 m max (cs) contre 25 m (fr), 300 ans (de) contre
100-150 (Atlas), floraison à 20-25 ans (de) contre 4 ans (EUFORGEN).

## 4. Points où le génome actuel semble s'écarter de la littérature

Les cinq écarts les plus importants soupçonnés : (1) incohérence d'étiquette de densité du bois (560 « basique » contre 570-600 à 12 %) et absence de données vertes pour *P. avium* ; (2) DBH en croissance libre au-dessus de la table 7 de Pryor, qui était la cible du génome ; (3) persistance des branches mortes et fût net naturel ; (4) architecture et profondeur racinaires ; (5) attribution erronée de la longévité 100-150 ans à Russell 2003 (qui donne 70-100 ans).

Ce sont des pistes à vérifier (les valeurs du génome sont des calibrations assumées ; le moteur n'a pas été exécuté).

1. **DBH en croissance libre.** Le génome, par ses propres commentaires, donne 37-39 cm à 40 ans, 52-54 cm à 60 ans et 76-78 cm
   à 100 ans. Pryor (table 7, croissance libre, 92 arbres, modèle préliminaire) : 30 cm à 40 ans, 44 cm à 60 ans, 50 cm à 70 ans.
   Albrecht (houppier solitaire) : 50-60 cm en 60-80 ans, ce qui est cohérent avec le génome à 60 ans ; Pretzsch : objectif 40-60 cm à
   70 ans largement dépassé sur stations riches. Le génome cite « 60 cm en 50-60 ans, Evans 1984 via FTT » : non vérifié
   (monographie introuvable), et nettement plus rapide que toutes les sources lues. Nuance : sources.md prend la table 7 de Pryor comme borne basse d'un arbre réellement isolé, ce qui est une hypothèse du projet (Pryor présente un modèle préliminaire, sans arbre vraiment solitaire).
2. **Houppier trop large relativement à la régression de Pryor.** Pour DBH 38 cm, Dk = 7.2 m, soit ~0.4 x une hauteur de 18 m ; le
   génome vise un rapport largeur/hauteur de 0.45-0.6 à 40 ans (0.7 en vieillesse). Sources sur grands houppiers : Albrecht >= 10 m
   comme objectif de sylviculture, pas une mesure.
3. **Persistance des branches mortes et fût net naturel.** La mort des branches à l'ombre est cohérente : Pryor dit qu'elles
   meurent vite à l'ombre (le génome : `shade_survival_years` 2 ; ne pas toucher à ce paramètre). L'écart porte sur ce qui suit :
   Pryor (« retained for many years after their death ») et Albrecht (« Totasterhalter ») disent que les branches mortes restent
   en place ; le génome les fait pourrir et tomber en quelques années à quelques décennies (`snag_years_per_m: 150`) et forme un
   fût net naturel (`clear_bole_max_m` 4.5 m, 20 % de la hauteur). Comparaison publiée la plus proche pour `clear_bole_*` : Albrecht
   obtient un fût sans branche verte sur ~25 % de la hauteur finale (6-8 m) à 17-22 ans, mais en peuplement dense et avec élagage à
   sec, jamais par chute naturelle des branches mortes.
4. **Profondeur racinaire.** Génome : `max_depth_m` 1.8 m, système « traçant » avec pivot court et peu de pivotants. Sources :
   racine en coeur (Herzwurzel) étalée, mais profondeur ~3 m sur sols faciles (Erlbeck 1998, seconde main) ; exigence d'un sol
   profond (Pryor) ; système plat à fort risque de chablis (Albrecht). Une cote de 3 m s'appuie sur une seule source de seconde main.
5. **Maturité du fruit (dans la plage, bas de fourchette).** Génome : ~60 jours après la floraison. Aas donne, pour la même
   région, floraison avril à début mai et maturité en juillet, soit environ 55-100 jours : 60 jours est dans la plage, côté court.
   Les maturités de juin viennent des variétés greffées.
6. **Âge de première floraison.** Génome 5.5 ans. Sources : ~4 ans (EUFORGEN, conditions optimales), < 10 ans (Pryor, de seconde main),
   10-15 ans (Atlas), 20-25 ans (Wikipédia de). Dans la fourchette basse ; à justifier face aux 10-15 ans de l'Atlas.
7. **Longévité : mauvaise attribution à Russell 2003.** Le commentaire du génome écrit « few live beyond ~100-150 y, Russell 2003 ».
   Russell (EUFORGEN, traduction italienne lue) dit que le merisier vit normalement 70-100 ans ; la plage 100-150 ans vient de
   l'Atlas (Welk et al. 2016, qui dit aussi 100-120 ans au maximum) et de la monographie Future Trees Trust (non retrouvée ici). Les
   paramètres (`typical_lifespan_years` 110, déclin dès 85 ans) sont dans la plage de l'Atlas mais au-dessus de celle d'EUFORGEN,
   et la pourriture racinaire (dès 50 ans) et du tronc (70-80 ans, Pretzsch, Albrecht) arrive plus tôt que le déclin du génome.
   Un maximum de 300 ans est donné par la Wikipédia de (sans source forestière à l'appui).
8. **Bois : emprunts à *P. serotina* et incohérence de densité.** Aucun équivalent vert publié pour *P. avium* n'a été trouvé (ni
   densité, ni MOR/MOE, ni humidité vert). Valeurs publiées à 12 % pour *P. avium* : densité 570-600 kg/m3, MOR 85-110 MPa, MOE
   10-11 GPa. Elles sont du même ordre que celles de *P. serotina* à 12 % (MOR 85, MOE 10.3 GPa, lues par résumé de recherche), donc
   le proxy est plausible **pour le MOR et le MOE seulement**. Pour la densité, le génome se contredit : `wood.density_kg_m3` = 560,
   commenté « basic density ~0.5-0.6 » ; or une densité basique (masse anhydre / volume vert) est toujours inférieure à la densité à
   12 %, ce qui impliquerait ~630 kg/m3 ou plus à 12 %, au-dessus de toutes les valeurs publiées de *P. avium* (570-600). Le bloc
   `mechanics` utilise en outre une SG verte de 0.47 (base volume vert), soit une densité basique effective d'environ 470. Soit l'étiquette
   est erronée (560 serait alors une densité à 12 %, cohérente avec CIRAD/Pryor), soit la valeur est trop haute. `elasticity_gpa` 8.0
   est annotée « vert ~7-9 », pas une valeur de *P. avium*. Le bois est semi-poreux : Jeske & Grosser décrivent un cercle poreux presque
   continu et des limites de cerne nettes, alors que le génome indique un contraste de cerne faible (0.3, `ring_porous: false`).
9. **Rejets de racine.** `root_suckering: 0.8` (élevé) et `stump_sprouting: 0.35`. Pryor n'a vu que peu de rejets après abattage, Aas
   dit que le merisier drageonne moins que d'autres *Prunus*, Forest Research « can sucker prolifically » : sources contradictoires, à
   relativiser.
10. **Gourmands.** `epicormic_potential: 0.2` : cohérent avec Pryor (peu fréquents) mais pas avec Albrecht (toujours après mise en
    lumière brutale). Dépend du scénario d'éclaircie.
11. **Saison d'élagage.** Pretzsch donne aussi la fin d'hiver comme période convenable (Pryor et Albrecht : été). Le génome (risque
    dormant 0.45 contre 0.04 en été) reflète la convention britannique ; la fin d'hiver n'est pas mauvaise dans l'essai bavarois.
12. **Ce qui concorde.** Plafond de hauteur ~25 m (Pretzsch ; génome `hydraulic_path_max_m` 25) ; âge de transition ombre-lumière ~10 ans
    (Albrecht ; génome 8-20 ans, `shade_juvenile_years` 8) ; chancre bactérien et plomb comme risque principal ; maximum de hauteur
    dominante 21-24 m à 40-60 ans en peuplement (génome 17-19 m à 40 ans et 20-22 m à 60 ans en isolé, plus bas que le peuplement de Pryor mais
    proche de la croissance libre).

## 4 bis. Vérification des citations du génome et de `docs/sources.md`

| Citation du génome / de sources.md | Vérification |
|---|---|
| Pryor 1988, table 7 : 14 / 30 / 44 / 50 cm à 15 / 40 / 60 / 70 ans ; H 9.2 / 16.3 / 21.7 m à 15 / 40 / 70 ans | confirmé (tableau lu). sources.md traite ces valeurs comme borne basse d'un arbre vraiment isolé : l'auteur donne un modèle préliminaire sur 92 arbres, dont les arbres isolés pris en compte sont surtout des arbres d'alignement et de lisière |
| Pryor : floraison « souvent avant 10 ans » | confirmé (cité par Pryor d'après Gordon & Rowe 1982) |
| Russell 2003 : floraison dès ~4 ans | confirmé (traduction italienne) |
| Russell 2003 : « few live beyond ~100-150 y » | **non confirmé** : Russell dit 70-100 ans ; 100-150 vient de l'Atlas/FTT |
| Welk et al. 2016 : 15-32 m, 90-120 cm, 100-150 (100-120 max) ans, fructification 10-15 ans, héliophile, enracinement superficiel | confirmé |
| Welk et al. 2016 : floraison dès fin mars, une semaine par arbre | confirmé |
| Future Trees Trust (shade-tolerant juvenile, 100-150 y, citant Joyce 1998 et Savill) | non retrouvé (404) ; Pryor et Albrecht confirment le jeune plant tolérant à l'ombre, Albrecht donne 10 ans |
| Evans 1984 via FTT : 60 cm en 50-60 ans | non vérifiable ; plus rapide que tout ce qui a été lu (Pryor croissance libre : 44 cm à 60 ans) |
| Kutschera & Lichtenegger 2002 + Savill 2013 : système traçant, pas de pivotants | non relus ici ; en tension avec Albrecht, Aas, Atlas, Wikipédia de (racine en coeur étalée) et Erlbeck via Stojecová (profondeur ~3 m) |
| Wood Handbook FPL-GTR-190 (green *P. serotina* 0.47 / 55 / 9.0) | valeurs vues dans un résumé de recherche (même tableau) ; pas de table verte pour *P. avium* trouvée |
| Alburquerque et al. 2008 (176-788 h de froid, cultivars) | non relu ici ; la Wikipédia es donne 500-1300 h pour des variétés (Sozzi 2008) |

## 5. Observables ajoutés au format

`dbh_increment_mm_yr`, `slenderness_hd`, `mai_m3_ha_yr`, `rotation_years`, `heart_rot_onset_years`, `heart_rot_fraction`,
`flowering_duration_days`, `chilling_hours`, `compressive_strength_mpa`, `tensile_strength_mpa`, `shrinkage_tangential_pct`,
`sapwood_width_cm`, `sapwood_diameter_fraction`, `sapwood_rings`, `bark_thickness_mm`, `bark_area_fraction`, `stem_volume_m3`,
`stand_volume_m3_ha`, `basal_area_m2_ha`, `min_soil_depth_cm`, `sucker_distance_m`, `root_suckering_relative`, `root_architecture`,
`soil_ph_range`, `frost_tolerance`, `max_altitude_m`, `pruning_wound_rot_fraction`, `max_occluded_branch_diameter_mm_4y`,
`pruning_max_branch_diameter_mm`, `pruning_season`, `dead_branch_persistence`, `gummosis`, `stump_sprouting`, `shoot_length_cm`,
`ring_width_trend`. Les observables catégoriels (tolérances, mycorhize, architecture racinaire) ont `value: None` et la catégorie dans
`conditions.extra`.

Conventions : les mois sont convertis en jour de l'année (avril 91-120, mai 121-151, juillet 182-212, année non bissextile) et la
conversion est notée dans `extra`. Les âges donnés en plage ont une valeur centrale ajoutée par nous (signalée). Les valeurs calculées
(circonférence -> DBH, double écorce / 2, rapport 18/34) sont signalées comme telles.
