# Prunier — Prunus domestica : fiche de traits pour le génome

Jeu de référence : `data/validation/prunus_domestica.ron` (39 entrées), l'espèce la **moins bien documentée** du lot : aucune table hauteur/DBH-âge, aucune phénologie bretonne. Parenté directe avec le merisier (Prunus, Rosaceae) : réutiliser les traits du merisier là où rien n'est sourcé. Légende : **mesuré** (id) ou **estimé**.

## Franc ou greffé
| Cas | Taille | Fruits | Longévité |
|---|---|---|---|
| Franc de pied (semis ou drageon) | 3-8 m [fr_wiki_height] | non trouvé | 30-50 ans [fr_wiki_longevity_franc] |
| Greffé Pixy / St Julien A / Torinel | 3-4 / 4,5-5 / 2,4-3 m [rhs_*] | 3-4 ans [rhs_*_first_fruit] | non trouvée |
| Myrobalan (franc ou porte-greffe) | non couvert | non trouvé | non trouvée |

Type *insititia* (damas, mirabelle, prunier de Saint-Julien) : arbre parfois épineux, hexaploïde, drageonne beaucoup et forme des fourrés [fr_wiki_suckers, fr_wiki_ploidy]. Seul le franc de pied de longévité 30-50 ans est sourcé (Wikipedia fr).

## Traits du génome du merisier
- **Architecture** : arbre moyen, souvent en parasol ou en vase [it_wiki_height] : **mesuré (qualitatif)** ; pas de port Hallé-Oldeman trouvé. Phyllotaxie 2/5 (Prunus) : identique au merisier, **estimé**. Fructification sur rameaux courts et brindilles (type lambourde) : non sourcée.
- **Feuille** : taille non relevée dans les sources lues ; reprendre le merisier (**estimé**).
- **Phénologie** : floraison mars-avril, avant les feuilles [fr_wiki_flowering] ; une semaine avant le pommier (Maine) [maine_bloom_vs_apple] ; maturité des fruits de juin à fin septembre selon variétés [it_wiki_fruit_maturity] ; besoin en froid 700-1100 h (prunier européen, Géorgie, seuil de température non précisé) [uga_chill_european] : **mesuré (hors Europe)**. Aucune date bretonne.
- **Gel** (prunier, type non précisé, tableau NMSU repris de WSU) : pleine floraison 10 % : -2,2 °C, 90 % : -5,0 °C ; jeune grappe -4,4/-8,9 °C ; bourgeon gonflé -10,0/-17,8 °C [nmsu_plum_*]. Wikipedia fr : « le gel compromet rarement les récoltes » grâce à la profusion de fleurs (affirmation). Proposé `flower_frost_c` = -2,2 (même que le merisier).
- **Bois** : 795 kg/m³ à 12 % (basale 610), MOR 88,4 MPa, MOE 10,19 GPa [wdb_plum_*] ; densité anhydre 750 kg/m³ (de) : **mesuré, mais conventions différentes** ; diffus-poreux (Prunus).
- **Écorce** : lenticelles horizontales comme le merisier (**estimé**).
- **Racines** : non trouvées ; superficielles/traçantes avec drageons (**estimé**). Drageons du prunier : le moteur a `root_suckering` (0,8 pour le merisier) à augmenter pour le type *insititia* (« drageonne beaucoup »).
- **Tolérances** : sol quelconque, silico-calcaire apprécié ; jusqu'à 1000 m [fr_wiki_altitude] ; ombre faible (**estimé**) ; Myrobolan GF 31 : très vigoureux sur sols secs et caillouteux, Saint-Julien : sols frais/argileux [fr_wiki_stjulien].
- **Reproduction** : fruits 2-3 cm pour l'insititia, jusqu'à 8 cm pour les variétés cultivées (it) ; 23-36 kg par arbre en pleine production dans le Maine [maine_yield_tree] ; `first_flowering_age_years` du franc : non trouvé (**estimé** ~5 ans comme le merisier).
- **Longévité** : franc 30-50 ans [mesuré, secondaire] : nettement plus court que le merisier (110 ans) ; `typical_lifespan_years` 40.
- **Symbiose** : non trouvée ; arbuscule (**estimé**).
- **Taille** : élaguer en été, plaies sensibles à la gommose et au chancre bactérien (UGA : bactérie qui suinte une gomme rouge-orangé en avril-mai) ; garder la valeur du merisier (`gummosis`) : **estimé**.

## Moteur
Drageons clonaux (fourrés), greffe sur St Julien/Myrobolan, hexaploïdie sans effet direct, gommose et chancre bactérien, floraison abondante avant feuilles et gel par stade.

## Non trouvé
Courbes hauteur/DBH-âge, rendement par arbre de franc, phénologie bretonne (CTIFL), mycorhizes, profondeur racinaire, masse des fruits.
