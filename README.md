# Genus Loci: Dendrologia

Simulateur d'arbres éducatif et **prototype de *Genus Loci: Seed Master***, le jeu complet à venir (plusieurs genres, laboratoire de semences, commandes de clients ; voir la [feuille de route](docs/roadmap.md)).

Un arbre pousse jour après jour à partir de ses gènes (un génome de traits mesurables), de son lieu (sol, climat, météo) et du temps, au-dessus et au-dessous du sol. Le joueur observe, accélère le temps, taille, coupe le tronc pour lire les cernes, et peut comprendre **pourquoi** l'arbre a réussi ou échoué. Chaque valeur du modèle est soit sourcée (voir [`docs/sources.md`](docs/sources.md)), soit déclarée « calibration ».

## Ce que fait le prototype

- **Un registre d'espèces** : chaque espèce est un fichier RON de traits (`data/species/<nom_latin>.ron`), découvert et embarqué à la compilation, sans branche de code par espèce ; le menu « Espèce » les groupe en feuillus, fruitiers et résineux. Aujourd'hui : chêne pédonculé (*Quercus robur*) et merisier (*Prunus avium*), et deux pilotes du registre, le pommier (*Malus domestica*, arbre franc de plein vent) et l'épicéa commun (*Picea abies*, résineux persistant : cohortes d'aiguilles, verticilles, cônes, masting). Ajouter une espèce : [`docs/species/ADDING_A_SPECIES.md`](docs/species/ADDING_A_SPECIES.md).
- **Quatre sols** (`data/soils/`) : limon profond (sol brun), mince sur calcaire (rendzine), argile engorgée (gley), sable sec (podzol). Le sol est en couches, avec un bilan hydrique ; l'asphyxie des racines est modélisée.
- **Climat et météo** : climat stylisé (ancré sur les normales de Rennes), météo journalière, sécheresse forcée à volonté.
- **Phénologie** : débourrement, floraison, fructification, couleurs d'automne, chute des feuilles ; croissance par bourgeons en compétition pour la lumière ; cernes annuels.
- **Vieillissement et mort** : aucune date de mort. Le bois vivant qui alimente le feuillage coûte de plus en plus à mesure que l'arbre grandit et que sa cime s'éloigne du collet ; quand le coût rattrape la production, les réserves baissent, la croissance ralentit, les branches au bilan négatif meurent (descente de cime) et la probabilité annuelle de mort monte. La longévité en émerge (déterministe pour une graine). L'arbre mort reste sur pied (chandelle) : il perd ses rameaux puis ses branches et le tronc finit par tomber, au bout de quelques années (bouleau) à quelques décennies (chêne) ; la souche peut rejeter chez les espèces qui le font. L'accéléré se met en pause à la mort du tronc ; la ligne « État » de l'interface donne la vigueur, les réserves, puis l'âge et la cause de la mort. Détails, sources et limites : `docs/sources.md`, « Longévité ».
- **Racines et sol « en iceberg »** : le système racinaire pousse dans un bloc de sol dont la forme suit les racines ; l'opacité du sol se règle (curseur « Sol », touche X) pour voir les racines.
- **Flux** : eau et sels minéraux (sève brute), sève élaborée, O₂, CO₂, vapeur d'eau et mycorhizes, en unités réelles par jour, tracés en traits lumineux sur l'arbre. Panneau latéral « Flux » avec une case par type de flux, repliable en bandeau de légende. C'est une **lecture** de l'état de l'arbre : les flux ne rétroagissent pas sur la croissance.
- **Taille** (« Taille », touche T) : coupes à la souris, plaies, compartimentation et pourriture, chronologie rejouable, annulation, sauts dans le temps, opérations en un clic (élaguer, éclaircir, réduire, têtard), et **arbre témoin** : la silhouette fantôme de l'arbre jamais taillé (même graine), avec un tableau de comparaison (hauteur, diamètre, feuillage, réserves, vigueur, carie, espérance de vie).
- **Mécanique** (« Mécanique », touche M) : tenue de l'arbre sous son poids et une rafale constante (curseur 0 à 50 m/s, avec km/h et Beaufort) : facteur de sécurité, point faible, mode de ruine attendu (rupture du tronc, d'une charpentière, déracinement) et vitesse critique de chaque mode ; le bois est coloré du vert (sûr) au rouge (rompt). Lecture statique sans rétroaction : rien ne casse encore. Valeurs et écarts : `docs/sources.md`, « Mécanique statique ».
- **Vue des cernes** (« Cernes ») : coupe du tronc à 1,3 m, survol et zoom à la molette.
- **Planches d'herbier** (menu « Planches ») : planches botaniques du chêne et du merisier, entièrement dessinées par le code (les autres espèces ont une fiche générique).
- **Menus** : écran titre, préparation de l'Arboretum (espèce, sol, âge de départ, graine), options, pause. « Laboratoire » est au menu mais désactivé (« à venir »).

## Commandes

| Entrée | Effet |
|---|---|
| Clic gauche + glisser | Orbite autour de l'arbre |
| Molette | Zoom |
| Clic droit ou milieu + glisser | Déplacer la vue |
| Double-clic | Recentrer sur le tronc, la couronne ou les racines |
| **F** | Recadrer l'arbre |
| **R** | Vue des racines (à travers la tranche retirée du sol) |
| **E** | Vue d'ensemble du bloc |
| **X** | Sol opaque / transparent |
| **G** | Repères de hauteur et de profondeur |
| **T** | Outil de taille (survoler une branche, clic : couper ; Maj : coupe rase) |
| **M** | Panneau « Mécanique » (vent constant, sécurité, vitesses critiques) |
| **Ctrl+Z** | Annuler la dernière coupe (outil de taille actif) |
| **Espace** | Lecture / pause |
| **Échap** | Menu pause |

La barre du bas donne la vitesse (de quelques jours par seconde à une dizaine d'années par seconde), la sécheresse forcée, et les boutons « Taille », « Repères », « Cernes », « Flux » ; la seconde ligne change d'espèce, de sol ou de graine (ce qui relance la simulation) et ouvre, à droite, le panneau « Mécanique ». Les panneaux « Cernes », « Taille », « Flux » et « Mécanique » s'empilent dans une colonne à droite sans se recouvrir, et le cadrage de la caméra en tient compte ; en cas de manque de place le panneau des flux se replie, puis le disque des cernes rétrécit. En mode taille, avec « Témoin » activé, le cadrage englobe l'arbre taillé et son témoin. Dans les planches, les flèches gauche et droite changent de planche. Les options règlent l'affichage (fenêtré ou plein écran, quatre résolutions), l'anticrénelage, la synchronisation verticale et l'échelle de l'interface (85 à 130 %) ; elles sont enregistrées dans `user://settings.cfg`.

## Construire et lancer

Prérequis : Rust stable et Godot **4.7.2** (éditeur standard, pas .NET). Voir [`docs/build.md`](docs/build.md) pour le détail (Linux vérifié ; Windows non vérifié à ce jour).

```bash
cargo build -p dendro-godot            # extension Godot (debug)
cargo build -p dendro-godot --release  # extension Godot (release)
godot --path godot                     # lancer le jeu
```

L'extension est chargée par `godot/dendrologia.gdextension` depuis `target/debug` ou `target/release`.

## Tests

```bash
cargo test                                   # cœur et liaison
cargo test --release -- --ignored            # tests de performance et de longue durée
cargo run -p dendro-core --release --example grow -- --years 200   # un arbre en console
DENDRO_SELFTEST=1 godot --headless --path godot                    # auto-test de l'interface : doit finir par « ALL PASSED »
```

## Variables d'environnement de débogage

Elles court-circuitent l'écran titre (le jeu démarre directement sur la simulation). Liste complète et commentaires : bloc « Debug / verification hooks » de [`godot/scripts/main.gd`](godot/scripts/main.gd) et `godot/scripts/prune_tool.gd`.

| Variable | Rôle |
|---|---|
| `DENDRO_SHOT=/chemin/absolu.png` | Avance rapide, enregistre une capture, quitte |
| `DENDRO_YEARS`, `DENDRO_DOY` | Années à simuler, jour de l'année de la capture (200 = mi-juillet) |
| `DENDRO_SPECIES`, `DENDRO_SOIL`, `DENDRO_SEED` | Espèce (identifiant du registre, ex. `picea_abies`, ou nom latin ; un indice marche encore mais change quand on ajoute une espèce), sol, graine |
| `DENDRO_SIZE=1280x720` | Taille de la fenêtre |
| `DENDRO_CAM`, `DENDRO_CAMDIST`, `DENDRO_YAW`, `DENDRO_PITCH` | Caméra : préréglages `leaves`, `flowers`, `trunk`, `low`, `wide`, `roots` ; angles en degrés |
| `DENDRO_XRAY`, `DENDRO_SOIL_OPACITY`, `DENDRO_GUIDES` | Sol transparent, opacité du sol, repères |
| `DENDRO_FLUX`, `DENDRO_FLUX_KINDS`, `DENDRO_FLUX_T` | Flux : affichage, types montrés, phase pour les images fixes |
| `DENDRO_MECA=32`, `DENDRO_MECA_TINT=0` | Panneau « Mécanique » ouvert à cette rafale (m/s), bois coloré (ou non) |
| `DENDRO_RINGS`, `DENDRO_RINGZOOM` | Panneau des cernes (1 par défaut dans les captures), zoom |
| `DENDRO_DROUGHT_YEARS` | Sécheresse forcée pendant les n dernières années |
| `DENDRO_TAILLE=1`, `DENDRO_GHOST=1` | Outil de taille actif, arbre témoin visible |
| `DENDRO_PRUNE="an:jour:op:arg;..."` | Coupes scriptées, par exemple `20:30:top:0.45` |
| `DENDRO_PRUNE_HOVER`, `DENDRO_PRUNE_CAM`, `DENDRO_PRUNE_PLAY` | Survol d'une branche, gros plan sur une plaie, accéléré après les coupes |
| `DENDRO_SCREEN=title\|setup\|plate0\|plate1\|options\|pause` | Capture d'un écran de menu |
| `DENDRO_UISCALE` | Échelle d'interface pour une capture |
| `DENDRO_MSAA`, `DENDRO_FXAA`, `DENDRO_NOA2C` | Réglages d'anticrénelage |
| `DENDRO_BENCH=8`, `DENDRO_STATIC=1` | Banc : n secondes à 10 ans/s, temps d'image moyen et pire ; `STATIC` : la caméra tourne autour d'un arbre fixe |
| `DENDRO_SELFTEST=1` | Auto-test de l'interface, sans affichage |

Exemple :

```bash
DENDRO_SHOT=$PWD/godot/.screenshots/essai.png DENDRO_YEARS=30 DENDRO_DOY=200 \
DENDRO_FLUX=1 DENDRO_SIZE=1280x720 godot --path godot
```

## Structure du dépôt

| Dossier | Rôle |
|---|---|
| `crates/dendro-core` | Cœur de simulation en Rust pur et déterministe (génome, climat, sol, phénologie, croissance, racines, flux, mécanique, taille, cernes, maillages). Aucune dépendance à Godot. Exemples dans `examples/`, tests dans `tests/`. |
| `crates/dendro-godot` | Liaison Godot (gdext) : expose la classe `TreeSim`. |
| `godot/` | Projet Godot 4.7 : scène 3D, interface, menus, vue des cernes, planches (`scripts/`). Texte de l'interface en français, code et commentaires en anglais. |
| `data/species/`, `data/soils/` | Génomes d'espèces (un fichier par espèce, découverts par `crates/dendro-core/build.rs`) et profils de sol (RON), compilés dans le binaire. Fiches de traits : `docs/species/`. |
| `godot/art/` | Fonds peints facultatifs (voir `godot/art/LISEZMOI.md`) ; sans eux, un fond procédural est dessiné. |
| `docs/` | [`roadmap.md`](docs/roadmap.md) (feuille de route et état détaillé), [`sources.md`](docs/sources.md) (sources scientifiques et écarts connus), [`build.md`](docs/build.md), analyses préparatoires, prompts d'images. |

## État

Prototype. Ce qui est solide : le cœur de simulation (déterministe, documenté, testé), la taille et l'interface principale. Ce qui est faible ou absent : plusieurs écarts connus à la littérature (diamètres, photosynthèse sans bilan carbone explicite, été probablement trop sec, voir `docs/sources.md`), des espèces et des sols compilés dans le binaire, aucune partie de jeu complète testée de bout en bout, plein écran et résolutions non vérifiés en vraie fenêtre, build Windows non vérifié, pas de laboratoire ni de commandes. Les jalons à venir sont dans [`docs/roadmap.md`](docs/roadmap.md).

## Licence

Licence à définir.
