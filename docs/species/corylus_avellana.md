# Noisetier — Corylus avellana : fiche de traits pour le génome

Jeu de référence : `data/validation/corylus_avellana.ron` (40 entrées). Le noisetier est un **arbuste cépée multicaule** ; il n'a pas de variante « greffée » : les variétés fruitières sont multipliées par marcottage, drageons ou boutures (Wikipedia fr), donc le franc est le cas normal. Légende : **mesuré** (id) ou **estimé**.

## Port et architecture
Touffe de 10-12 troncs, 2-5 (rarement 8) m [fr_wiki_stool, fr_wiki_height] ; 4-8 m, parfois > 10 m selon l'Atlas [atlas_height] ; 5-6 m en cépée, rarement arbre de 10 m [de_wiki_height]. Rejets de souche de plusieurs mètres dès la 1re année, ramifiés en 2e année ; ramification sympodiale. Hallé-Oldeman : non trouvé (la page de ne le mentionne pas). Brins : vie courte (une souche entière 70-90 ans, jusqu'à 60 pour le cultivé : divergence [atlas_lifespan, fls_lifespan, fr_wiki_cultivar_life]) ; taillis traditionnel de 5-7 ans [rhs_coppice_rotation].

## Traits du génome du merisier
- **Architecture** : dominance apicale faible, rejets basaux vigoureux (rajeunissement par recépage) : **estimé**, avec confirmation qualitative [atlas_sprout, de_wiki_height]. Un compteur de brins par souche est nécessaire.
- **Feuille** : arrondie, 6-12 cm, doublement dentée, poilue, caduque [atlas_leaf] : **mesuré**.
- **Phénologie** : floraison très précoce, avant les feuilles : Slovaquie 19 mars (jour 78) en 1964-83 puis 5 j plus tôt [sk_flowering_*] ; nord de l'Italie côtière 19 janvier [sk_italy_onset] ; Allemagne février-mars, fin janvier en 2018 [de_wiki_flowering, de_wiki_shift_2018] ; France décembre-mars (qualitatif). Durée 7-9 j [sk_flowering_duration] ; maturité des noisettes 30 août (jour 242) en Slovaquie [sk_nut_ripeness]. Aucune date bretonne ni britannique chiffrée. Besoin en froid et températures critiques des chatons : non trouvés [absence_corylus_*].
- **Fleurs** : monoïque, autostérile, chatons mâles formés dès l'automne précédent et nus pendant l'hiver ; chatons ≤ 10 cm ; fleurs femelles minuscules à styles rouges (de, Atlas) [atlas_catkin] : pollen anémogame à gérer en tant que saison (allergène, Atlas).
- **Bois** : 570-630 kg/m³ à 15 % (r15) [de_wiki_density] ; pas de duramen distinct ; pas de données MOR/MOE trouvées. `growth_coefficient`, `adaptive_stress_mpa` : **estimé**.
- **Écorce** : lisse, brun cuivré à lenticelles blanches horizontales, pas de rhytidome ; fissures longitudinales avec l'âge [de_wiki, fr_wiki] : **mesuré (qualitatif)**.
- **Racines** : longueur 3-4 m en haies [de_wiki_root_length] ; pas de profondeur chiffrée ; `taproot` faible (**estimé**).
- **Tolérances** : croît à pleine lumière comme à l'ombre (Atlas) : `shade` ≈ 0,5-0,6 (**estimé**) ; rustique, résiste aux gelées ; sols fertiles peu acides ou calcaires secs ; jusqu'à 1700 m (fr) ; en Bretagne, présent en haies et sous-bois.
- **Reproduction** : pleine production à 8-12 ans en culture [fr_wiki_full_production] ; noix ≤ 2 cm [atlas_nut] ; `first_flowering_age_years` du franc : non trouvé (**estimé** 5-7 ans).
- **Longévité** : voir ci-dessus : l'arbuste meurt par brins, la souche dure plus.
- **Symbiose** : **ectomycorhize** avec *Tuber* (truffes), *Amanita rubescens*, *Boletus edulis*, *Cenococcum*, *Laccaria laccata*, *Paxillus involutus* [de_wiki_mycorrhiza, en_wiki_mycorrhiza] ; contrairement au merisier (arbuscule). Nouveau type à utiliser dans le génome (`Ecto`).
- **Taille / blessures** : rejette vigoureusement de souche ; recépage régulier ; sève peu abondante (**estimé**). Chancre bactérien (*Pseudomonas syringae* pv. *coryli*) et virus de la mosaïque du pommier signalés [Atlas].

## Moteur
Cépée multicaule (plusieurs troncs d'âges différents), rejets de souche et drageons, recépage cyclique, chatons mâles hivernaux, mycorhize ectomycorhizienne, floraison en plein hiver.

## Non trouvé
Besoin en froid, gel des chatons, dates bretonnes (PEP725 non téléchargé), DBH par brin, nombre de tiges en fonction de l'âge, masse de noisette, profondeur racinaire.
