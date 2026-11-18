# Feuille de route : de *Dendrologia* à *Seed Master*

Date : 2026-11-16. Document vivant : à relire à chaque jalon terminé. Pas de dates calendaires, des tailles relatives (**S** : quelques séances, **M** : quelques semaines, **L** : un à deux mois de travail solo).

## 1. Vision

*Genus Loci: Dendrologia* est le prototype : un arbre pousse jour après jour à partir de ses gènes (un génome de traits mesurables), de son lieu (sol, climat, météo) et du temps, au-dessus et au-dessous du sol. *Genus Loci: Seed Master* sera le jeu complet : plusieurs genres, arbustes et herbacées, un laboratoire de semences, un milieu physique qui met la plante à l'épreuve (vent, pluie, neige), et la possibilité de comprendre **pourquoi** une plante a réussi ou échoué.

### Principes directeurs (non négociables)

1. **Fidélité scientifique.** Toute valeur est soit sourcée (commentaire dans le code + `docs/sources.md`), soit déclarée « calibration ». Tout écart connu à la littérature va dans « Écarts connus ». Une fonctionnalité de jeu ne contourne jamais la simulation : elle la lit ou agit sur ses entrées.
2. **Génome de traits, zéro branche par espèce.** Une nouvelle espèce = un fichier RON. Si une espèce exige du code, c'est un trait générique qui manque : on l'ajoute au génome pour toutes.
3. **Calme et lisible.** Ambiance peinte, renouveau printanier, entre sombre et aérien. Pas de minuterie punitive ; l'échec existe mais sa cause est lisible (cernes, journal, flux).
4. **Budget de performance.** La machine cible (i5-3570K, 16 Go, Radeon HD 7950) fixe le plafond. Toute fonctionnalité annonce son coût et passe les tests de performance (§5.2).
5. **Cœur indépendant du moteur.** `dendro-core` reste du Rust pur, déterministe ; GDScript ne fait que présentation et interface.

## 2. État actuel (honnête)

| Domaine | Ce qui existe | Maturité |
|---|---|---|
| Cœur de simulation | `dendro-core` (~9 000 lignes) : climat ancré sur la fiche Météo-France de Rennes (températures Tx/Tn par un générateur de type WGEN calé sur les normales et les jours de gel mensuels ; pluie encore stylisée), sol en couches avec bilan hydrique type BILJOU, phénologie chilling-forcing, croissance par bourgeons et compétition pour la lumière (Palubicki 2009), loi de Pressler / modèle du tuyau, cernes (initial/final, poreux ou non), racines stylisées, reproduction, longévité, taille. Déterministe (`rng.rs` : `advance_days(365) == 365 × advance_days(1)`). | Solide pour un POC, documenté et sourcé. |
| Espèces et sols | 2 espèces (*Quercus robur*, *Prunus avium*), 4 sols. **Compilés dans le binaire** (`include_str!` dans `genome.rs` et `soil.rs`). | Génome riche (13 groupes de traits) mais testé sur deux feuillus seulement. |
| Taille | Coupes, plaies, compartimentation et pourriture, chronologie rejouable (`LogEvent`), annulation, arbre témoin « fantôme », mesures objectives. | Le plus abouti côté jeu. |
| Flux | Xylème, minéraux, phloème, O₂/CO₂, vapeur, mycorhizes en unités réelles, tracés lumineux. `flux.rs` est **sans état et sans rétroaction** sur la croissance. | Bonne vitrine pédagogique, c'est une lecture, pas un modèle. |
| Rendu et interface | Godot 4.7 Forward+, bloc de sol en iceberg, vue des cernes, planches d'herbier générées par code, menus (titre, installation, options, pause), fonds peints optionnels. « Laboratoire » et « Commandes » sont déjà au menu, désactivés (« à venir ») ; l'entrée « Commandes » est à retirer (voir §3, mode mis de côté). | Belle base, jamais éprouvée en vraie partie. |
| Outillage | plus de 70 tests Rust (dont 2 tests de perf `#[ignore]`), auto-test Godot sans affichage (`DENDRO_SELFTEST`), captures (`DENDRO_SHOT`), banc (`DENDRO_BENCH`). | Bon, mais **aucune CI** (pas de `.github/`). |
| Plateformes | `godot/dendrologia.gdextension` ne déclare que `linux.*`. | **Pas de build Windows**, donc pas de Steam en l'état. |
| Sauvegarde | Aucune (seuls les réglages sont dans `user://settings.cfg`). | À faire. |

**Points faibles connus** (rapports récents) : diamètres du tronc et des branches trop faibles (**en cours, autre chantier**) ; pas de photosynthèse pilotée par le rayonnement ni de bilan carbone/réserves ; limite hydraulique de hauteur indépendante de l'eau du site ; été probablement trop sec (70 à 140 jours sous REW 0,4) ; transpiration des grands arbres probablement trop forte ; mycorhizes sans rétroaction ; phloème dessiné dans un seul sens par jour ; nœuds et pourriture absents de la vue des cernes ; le merisier recépé devrait drageonner (`pruning.rs` : « root suckers placed away from the stump » non modélisés) ; le panneau des flux peut couvrir la couronne ; le fantôme est coupé par le cadrage ; accéléré à ~12 FPS à 10 ans/s sur un chêne de 150 ans ; aucune session de jeu complète à la souris et au clavier ; plein écran et résolutions non testés en vraie fenêtre ; dossier local encore nommé `dendrologia` ; README périmé (ne cite que le chêne).

## 3. Jalons

Ordre décidé le 2026-11-16 : **M0 d'abord** (sans CI ni sauvegarde, tout le reste est fragile), puis **M1, la physique du milieu**, remontée en priorité par le propriétaire. M2 et M3 peuvent ensuite s'entrelacer (une séance de réalisme, une séance de jeu).

**Mode « Commandes » : mis de côté.** Le propriétaire veut y repenser avant de l'inscrire ici. Il ne figure plus dans aucun jalon ; l'entrée désactivée du menu titre est à retirer en attendant. Les mesures objectives de la taille restent disponibles pour un futur système d'objectifs, quel qu'il soit.

### M0. POC consolidé (stabilisation et dette) — taille M

**But.** Un prototype que quelqu'un d'autre peut installer, lancer, jouer 30 minutes et sauvegarder, sur Linux et Windows.

- [ ] Session de jeu complète à la souris et au clavier (2 × 30 min, une par espèce), notes dans `docs/qa/` ; corriger tout ce qui bloque.
- [ ] Panneau des flux : ne plus couvrir la couronne (ancrage latéral ou repli automatique). Cadrage : le fantôme entièrement visible.
- [ ] Accéléré : ≥ 30 FPS à 10 ans/s sur un chêne de 150 ans (voir §4, tâche 3).
- [ ] Sauvegarde/chargement d'un arbre : génome (nom ou RON), graine, climat, sol, journal `LogEvent`, jour courant ; rechargement par rejeu. Format RON versionné (`version: 1`).
- [ ] Build Windows : entrées `windows.*` dans `dendrologia.gdextension`, compilation croisée ou native, modèles d'export Godot ; un exécutable lancé sur la machine cible.
- [ ] CI (GitHub Actions) Linux + Windows : `cargo test`, `cargo test --release -- --ignored`, `godot --headless` avec `DENDRO_SELFTEST=1`.
- [ ] Test de déterminisme inter-plateformes : hachage de l'état à 100 ans identique sous Linux et Windows (voir risques).
- [ ] Plein écran et résolutions vérifiés en vraie fenêtre (4 résolutions de `game.gd`, 2 échelles d'interface).
- [ ] README à jour (deux espèces, sols, taille, flux, captures, commandes) ; renommer le dossier local en `genus-loci`.
- [ ] Choisir et poser la licence (voir §7) : `LICENSE` + en-tête dans le README.

**Critères de sortie.** CI verte sur les deux OS ; les deux tests `#[ignore]` passent (`performance_cherry_130_years` < 2 s) ; un arbre taillé, sauvegardé puis rechargé redonne le même hachage d'état ; `DENDRO_BENCH` mesure ≥ 30 FPS sur la machine cible ; fiche QA sans bloquant ouvert.

**Risques.** Le déterminisme entre OS : `powf`, `exp`, `sin` en f32 passent par la libm de la plateforme et peuvent différer au dernier bit, ce qui suffit à faire diverger 150 ans de croissance. Parades : `libm` (crate Rust pur) pour les fonctions transcendantes du cœur, ou sauvegarder un instantané d'état en plus du journal. Le rejeu coûte ~1 à 2 s par siècle au chargement : afficher une planche ou une barre de progression.

### M1. Physique du milieu : vent, pluie, neige, casse — taille L (priorité haute)

**But.** Voir la résistance de l'arbre dans son milieu, jusqu'à la rupture. Le joueur observe l'arbre plier, se charger d'eau ou de neige, perdre une branche, casser ou se déraciner, et comprend pourquoi (levier, diamètre, bois, ancrage, sol détrempé). Tout passe par la simulation : aucun dégât tiré au hasard hors modèle.

**Fait (2026-11-16) : mécanique statique**, `crates/dendro-core/src/mechanics.rs` (lecture sans état ni rétroaction, comme les flux), tests `tests/mechanics.rs`, panneau « Mécanique » (touche M, `DENDRO_MECA`), sources et écarts dans `docs/sources.md` (« Mécanique statique »). Poids propre + rafale constante, facteur de sécurité par segment, point faible, mode de ruine, vitesse critique de chaque mode. Une masse supplémentaire par segment (`WindLoad::extra_mass_kg`) attend la neige et l'eau ; rien ne casse encore (la rupture effective et son inscription au journal sont l'étape suivante). Écarts principaux : jeunes arbres trop fragiles en feuilles, charpentières encore cassantes en feuilles au vent (~28 m/s pour le chêne de 150 ans ; la croissance adaptative ne répond qu'au poids propre), ancrage et traînée des feuillus estimés à partir de conifères et de juvéniles d'autres genres.

- [ ] **Prérequis** : diamètres du tronc et des branches réalistes (chantier en cours) et densité du bois vert par espèce ; sans eux, toute mécanique est fausse. *Densité verte, MOR et MOE par espèce : faits (traits `mechanics`, Wood Handbook, espèces américaines du même genre). Diamètres : charpentières corrigées pour le poids propre par une croissance radiale adaptative (consigne de contrainte uniforme, 11,1 MPa, Evans et al. 2008 ; `growth.rs`), sécurité du chêne de 150 ans 1,6 → 5,0 ; reste la réponse au vent.*
- [~] **Vent, charge** : profil de vent avec la hauteur, rafales ; traînée sur la couronne proportionnelle à la surface foliaire, avec reconfiguration (le feuillage se profile : la traînée croît moins vite que v²) ; arbre en feuilles contre arbre nu. *Fait en statique : profil logarithmique (rafale à 10 m), traînée du houppier sur sa surface frontale avec exposant de Vogel, traînée des rameaux nus, feuillé/défeuillé selon la phénologie. Reste : rafales dynamiques.*
- [x] **Mécanique du bois** : moment fléchissant cumulé le long de chaque axe (poids propre + vent + charges), contrainte de flexion σ = 32 M / (π d³), comparée au module de rupture du bois vert (trait d'espèce, avec le module d'élasticité) ; plaies et pourriture de `pruning.rs` réduisent la section résistante (une cavité fragilise). *Fait (statique) ; la carie est une cavité centrée.*
- [~] **Deux modes de ruine** : casse du tronc ou d'une branche (volis), et déracinement (chablis) quand le moment au collet dépasse l'ancrage, calculé depuis la plaque racinaire simulée et l'état du sol (un sol saturé ancre moins). Références de départ à vérifier et citer : modèles HWIND et GALES (Peltola et al. 1999 ; Gardiner et al. 2000, 2008). *Fait en statique : rupture du tronc ou d'une charpentière, déracinement `Creg × masse de la tige` (ForestGALES), réduit en sol saturé et pour un enracinement superficiel ; le coefficient vient d'un conifère (estimation). Reste : plaque racinaire calculée depuis les racines simulées, rupture effective.*
- [ ] **Acclimatation au vent** (thigmomorphogenèse) : un arbre exposé pousse plus trapu, plus conique, avec plus de racines du côté sous tension ; port en drapeau sous vent dominant. Traits génériques, pas de branche par espèce.
- [ ] **Pluie** : poids de l'eau interceptée sur le feuillage, ruissellement le long du tronc, sol détrempé, engorgement et crue (relie l'asphyxie racinaire existante).
- [ ] **Neige et givre** : accumulation selon la température et la surface porteuse (les feuilles encore présentes en automne aggravent tout), charge sur les branches, courbure puis casse, décharge au redoux ; verglas.
- [ ] **Autres agents physiques** (second temps) : gel tardif marqué, canicule et sécheresse extrêmes, foudre.
- [ ] **Climat, suite du générateur WGEN** : résidus de température non gaussiens (queue chaude de Tx l'été, queue froide de Tn plus légère que la gaussienne l'hiver, voir « Écarts connus ») ; pluie recalée sur les tableaux mensuels de la fiche (chaîne de Markov sec/humide de WGEN) ; idéalement calage sur une série journalière de Rennes.
- [ ] **Deux échelles de temps** : la croissance reste au pas journalier ; un événement (rafale, chute de neige) est résolu à part, à pas fin, pendant quelques secondes de jeu, puis ses dégâts sont inscrits dans le journal rejouable (`LogEvent`) comme les coupes.
- [ ] **Rendu** : balancement des branches en temps réel selon leur raideur et leur fréquence propre (shader, pas de coût CPU par branche), feuillage qui se couche, pluie, neige posée sur les branches et le sol, branche qui tombe, arbre renversé avec sa galette racinaire.
- [~] **Banc d'essai** : le joueur peut déclencher les éléments lui-même (curseur de vent jusqu'à la rupture, averse, chute de neige), avec lecture des contraintes en couleur sur le bois (du vert au rouge) et vitesse critique affichée ; *fait pour le vent statique (panneau « Mécanique » : curseur 0-50 m/s, bois coloré, vitesses critiques)* ; et les subir au fil des ans, tirés du générateur de climat avec des périodes de retour sourcées (tempêtes de Rennes et de l'ouest de la France).
- [ ] Les dégâts laissent leur trace : plaie de casse reprise par le modèle de plaies existant, rejets, cerne étroit, bois de réaction dans la vue des cernes, entrée au journal.

**Critères de sortie.** Un chêne isolé de 100 ans casse ou se déracine à une vitesse de vent comprise dans la plage publiée pour l'espèce et le sol ; le même arbre sur sol saturé cède plus tôt et par déracinement ; un arbre pourri à cœur casse avant un arbre sain ; un arbre élevé sous vent est mesurablement plus trapu ; une neige lourde d'octobre sur merisier en feuilles casse des branches, la même neige en janvier non ; 60 FPS en vue libre pendant une tempête sur la machine cible ; chaque loi a sa source dans le code et dans `docs/sources.md`.

**Risques.** La mécanique dépend de diamètres et d'un ancrage justes : valider d'abord sur des cas publiés (essais de treuillage). Le coût : calculer les contraintes par axe et non par segment fin, et seulement pendant les événements. Le ton : la casse doit rester lisible et récupérable (l'arbre rejette, cicatrise), pas punitive. Le déterminisme : les événements météo doivent être rejouables à l'identique (journal).

### M2. Réalisme du moteur — taille L

**But.** Faire disparaître les écarts majeurs de `docs/sources.md` (« Écarts connus », déjà classés par importance : on suit cet ordre) et prouver le modèle par une suite de validation.

- [ ] **Photosynthèse par le rayonnement** : efficience d'utilisation du rayonnement (Monteith) avec le rayonnement déjà calculé dans `climate.rs`, interception par la couronne (loi de Beer-Lambert sur le LAI), saturation, effet de la température et de l'eau.
- [ ] **Bilan carbone explicite** : production brute → respiration (entretien ∝ masse vivante, construction) → allocation (feuilles, bois, racines, fruits) → **réserves pluriannuelles** (amidon). Le bois initial du chêne puise dans les réserves réelles au lieu de « productivité de l'an dernier » ; la fructification a un coût.
- [ ] **Allométrie** : vérifier diamètres, hauteur, rayon de couronne contre des relations publiées pour arbres isolés ; reprendre la sortie du chantier diamètres en cours.
- [ ] **Bilan hydrique** : interception dépendant du LAI et de la pluie (Aussenac, comme Granier 1999) ; transpiration bornée par la conductance et la surface d'aubier ; recaler l'été (cible : jours sous REW 0,4 comparables aux chênaies suivies par Bréda et Granier).
- [ ] **Hauteur maximale dépendant du site** : `hydraulic_path_max_m` modulé par l'eau disponible et la fertilité (Ryan & Yoder 1997), pour que la rendzine donne un chêne nettement plus bas.
- [ ] **Mycorhizes avec rétroaction** : c'est un changement d'architecture (`flux.rs` est une lecture sans état) : déplacer l'échange carbone ↔ phosphore/azote dans l'état de `sim.rs`, avec un coût en carbone et un gain d'absorption selon le sol.
- [ ] Drageons du merisier après recépage (racines traçantes, pousses à distance de la souche).
- [ ] **Suite de validation** (`crates/dendro-core/tests/validation.rs` + `docs/validation.md`) : courbes hauteur/diamètre par âge contre données publiées, dates de débourrement contre séries phénologiques, largeur des cernes en année sèche, chaque test citant sa source et sa tolérance.

**Critères de sortie.** Chaque point ci-dessus a disparu d'« Écarts connus » ou y est requalifié avec une raison ; la suite de validation passe en CI ; le temps de `performance_200_years` n'a pas augmenté de plus de 30 %.

**Risques.** Recalibrer tout ce qui a été calé à la main (les deux espèces bougent ensemble). Mitigation : instantanés de référence (`examples/snapshot.rs`) comparés avant/après chaque changement, et un seul mécanisme par séance. Risque de modèle trop lourd : garder un pas journalier, pas horaire.

### M3. Boucle de jeu du prototype — taille L

**But.** Transformer le simulateur en jeu : des choix (lieu, espèce, taille, épreuves du milieu) et une lecture claire de leurs conséquences. La forme des objectifs reste à définir par le propriétaire (le mode « Commandes » est mis de côté).

- [ ] Choix du site comme décision : sol et exposition dans l'écran d'installation, avec conséquences visibles.
- [ ] **Journal de l'arbre** : événements datés (gel tardif, sécheresse, coupe, première floraison, branche morte), cliquables pour sauter dans la chronologie.
- [ ] **Bulles pédagogiques** liées aux sources : chaque notion (cerne, compartimentation, débourrement) renvoie à une entrée de `docs/sources.md` ; un seul fichier de textes, prêt pour la traduction.
- [ ] Nœuds et pourriture dessinés dans la vue des cernes (la coupe devient l'outil de diagnostic).
- [ ] Phloème dessiné dans les deux sens selon la saison (source → puits : feuilles → racines l'été, réserves → bourgeons au débourrement).
- [ ] **Didacticiel** : une première partie guidée (planter, avancer d'une saison, tailler, lire un cerne), 10 minutes au plus.

**Critères de sortie.** Un testeur extérieur termine le didacticiel sans aide ; chaque bulle pointe vers une source existante (test automatique : aucune clé orpheline).

**Risques.** Sans objectifs, le prototype reste un bac à sable : décider tôt de leur forme. Dérive vers le menu à options : garder une seule boucle (site → croissance → épreuves et taille → lecture du résultat).

### M4. Contenu : espèces et généralisation du génome — taille L

**But.** Passer de 2 à ~10 espèces, chacune choisie pour **casser** une hypothèse du génome, puis sortir des arbres.

- [ ] Prérequis : charger espèces, sols et climats **à l'exécution** depuis un dossier de données (`data/` + `user://`), plus seulement par `include_str!`. Indispensable aussi pour le labo et le partage.
- [ ] Prérequis : un climat de plus (continental ou méditerranéen) en RON, pour éprouver le climat comme donnée.

Ordre proposé et trait mis à l'épreuve :

| Ordre | Espèce | Ce qu'elle éprouve |
|---|---|---|
| 1 | *Prunus padus* ou *P. spinosa* | Même genre : le génome tient-il sans code ? *P. spinosa* : drageonnement, port multicaule (prépare les arbustes). |
| 2 | *Fagus sylvatica* | Tolérance à l'ombre extrême, bois diffus, sensibilité à la sécheresse, débourrement photopériodique. |
| 3 | *Betula pendula* | Pionnière à vie courte, croissance très rapide, graines légères. |
| 4 | *Fraxinus excelsior* | Feuilles composées, débourrement tardif, bois à zone poreuse, chalarose (prépare M5). |
| 5 | *Salix alba* / *Populus nigra* | Sols engorgés, têtards, rejets abondants : stress de la taille. |
| 6 | *Pinus sylvestris* | Premier conifère : sempervirence réelle (aujourd'hui un stub à date fixe dans `phenology.rs`), verticilles, aiguilles, bois sans vaisseaux, résine. |
| 7 | *Corylus avellana* | Premier arbuste : touffe de tiges, recépage naturel. |
| 8 | Une herbacée vivace (*Digitalis purpurea* ou une graminée) | Pas de croissance secondaire ni de cernes : le génome doit rendre ces blocs optionnels. |

**Critères de sortie.** Chaque espèce : un RON, une entrée sourcée dans `docs/sources.md`, une planche d'herbier, un test de forme (hauteur et diamètre dans les plages de l'Atlas européen) ; **zéro** `if species ==` dans le code (vérifié par grep en CI).

**Risques.** Le génome peut exploser en traits ad hoc : chaque nouveau trait doit servir au moins deux espèces ou une loi générale. Les conifères et herbacées touchent l'architecture (`growth.rs`, `mesh.rs`) : prévoir une refonte, pas un ajout.

### M5. Le monde vivant — taille L

**But.** Un lieu vivant : saisons soignées, voisins, bioagresseurs. (Les événements physiques, vent, pluie, neige, sont passés en M1.)

- [ ] Saisons : ciel, lumière, sol enneigé, litière, couleurs ; rythme sonore saisonnier.
- [ ] Plusieurs plantes : compétition pour la lumière (le modèle de bourgeons s'y prête) et pour l'eau du sol (partage des couches) ; budget de performance d'abord (LOD, instanciation du feuillage, arbres lointains figés).
- [ ] Bioagresseurs : 2 ou 3 cas bien documentés (oïdium du chêne, chalarose du frêne, pucerons), avec une cause lisible.

**Critères de sortie.** Une parcelle de 5 arbres tient 30 FPS en accéléré sur la machine cible ; chaque bioagresseur a sa trace dans les cernes et le journal.

**Risques.** Le coût de performance de la compétition (n arbres × lumière) ; le ton : les bioagresseurs ne doivent pas rendre le jeu anxiogène (rares, explicables, récupérables).

### M6. Méta-jeu : laboratoire et sélection — taille L

**But.** Activer « Laboratoire » : le joueur crée, croise et sélectionne des semences. C'est ce qui justifie le nom *Seed Master*.

- [ ] Hérédité : chaque trait du génome reçoit une variance et une héritabilité ; croisement intra-spécifique (puis entre espèces proches), mutation bornée par les plages publiées.
- [ ] Banque de semences : graines récoltées sur les arbres simulés (la reproduction existe déjà), étiquetées par provenance et climat.
- [ ] Arbre de compétences du labo, sous forme d'**arbre phylogénétique** (les déblocages suivent la parenté réelle des genres).
- [ ] **Règle ferme** (voir `docs/analyse-tree-simulator.md` §4.3) : les cernes ne sont **jamais** une monnaie. Les déblocages viennent d'observations prouvées (« faire survivre un arbre à 3 gels tardifs »).
- [ ] Partage : une graine ou un arbre = un code court (génome + graine + site + journal), import/export.

**Critères de sortie.** Un croisement de deux merisiers donne une descendance dont les traits suivent la moyenne parentale ± variance attendue (test statistique) ; un code partagé reproduit le même arbre sur un autre PC (dépend du déterminisme de M0).

**Risques.** La sélection peut produire des arbres irréalistes : bornes dures par espèce, avec source. Le périmètre : c'est le jalon le plus facile à repousser après 1.0 (voir §7).

### M7. Finition et sortie — taille L

**But.** *Seed Master 1.0*.

- [ ] Passe artistique : feuillage peint, écorces, ciel, sol ; planches d'herbier finales ; fonds peints (IA optionnelle, prompts dans `docs/prompts-images.md`, mentionnés dans les crédits).
- [ ] Audio : ambiance générative (vent dans le feuillage selon le LAI, pluie, oiseaux selon la saison), musique calme.
- [ ] Localisation FR/EN via le système de traduction de Godot (fichiers CSV/PO hors scènes) ; noms latins inchangés.
- [ ] Accessibilité : échelle d'interface (existe), daltonisme (flux et pigments d'automne), remappage des touches, sous-titres des bulles, pas d'information portée par la couleur seule.
- [ ] Page Steam (captures, bande-annonce en accéléré, tags), démo, build Steam Deck si le coût est faible.
- [ ] Partage communautaire : codes d'arbres, galerie, éventuellement Steam Workshop pour les espèces RON.

**Critères de sortie.** Build Windows et Linux signés, testés sur la machine cible ; jeu entièrement traduit ; 3 testeurs extérieurs sans bloquant.

## 4. Les 10 prochaines tâches (une séance chacune, par priorité)

1. **Session de QA en vraie fenêtre** : jouer 30 min par espèce à la souris et au clavier, tester plein écran et les 4 résolutions ; noter chaque accroc dans `docs/qa/2026-11-session1.md`. Sortie : liste priorisée de bogues.
2. **Valider le chantier diamètres** quand il est fusionné : `cargo run --example snapshot` et captures `DENDRO_SHOT` à 25, 60, 100, 200 ans ; mettre à jour « Écarts connus » (paragraphe sur les charpentières grêles).
3. **Profiler l'accéléré** (12 FPS à 10 ans/s, chêne de 150 ans) : mesurer avec `DENDRO_BENCH` la part simulation / reconstruction des maillages / transfert GPU. Pistes : maillages reconstruits sur un fil Rust séparé, reconstruction du bois seulement aux événements structurels, feuillage instancié (`MultiMesh`), plafond `REBUILD_INTERVAL` adaptatif.
4. **Sauvegarde/chargement v1** : sérialiser (espèce, graine, climat, sol, journal, jour) en RON dans `user://saves/`, recharger par rejeu, test Rust aller-retour avec hachage d'état.
5. **CI GitHub Actions** Linux : `cargo test`, tests `--ignored` en release, auto-test Godot sans affichage. Puis ajouter Windows.
6. **Build Windows** : `windows.*` dans `dendrologia.gdextension`, export Godot, lancement sur la machine cible ; test de déterminisme Linux/Windows (hachage à 100 ans).
7. **Panneau des flux et cadrage du fantôme** : repli automatique du panneau quand il chevauche la couronne projetée ; cadrage caméra qui englobe l'arbre et son fantôme.
8. **README et licence** : README à jour (deux espèces, sols, taille, flux, captures), fichier `LICENSE` une fois la décision prise, renommage du dossier local.
9. **Phloème bidirectionnel** dans `flux.rs` : sens fixé par le rapport source/puits du jour (réserves → bourgeons au débourrement, feuilles → racines et cerne en été) ; test dans `tests/flux.rs`.
10. **Nœuds et pourriture dans la vue des cernes** : exposer par la liaison les plaies et branches incluses à une hauteur de coupe donnée, les dessiner dans `ring_view.gd`. 

Ensuite : **M1, physique du milieu**, en commençant par la mécanique statique (poids propre et vent constant → contrainte par axe, vitesse critique de casse et de déracinement, test contre des valeurs publiées), avant le rendu du balancement, la pluie et la neige.

## 5. Chantiers transversaux

### 5.1 Validation scientifique
- Chaque séance qui touche une valeur met à jour `docs/sources.md` (source lue, ou « calibration »).
- `docs/validation.md` : tableau espèce × grandeur × source × tolérance × statut, alimenté par la suite de tests de M2.
- Niveau de confiance par trait (lu / recoupé / calibré) affiché dans la planche d'herbier : la transparence est un argument de vente.
- Relecture externe : proposer la suite de validation à un forestier ou un écophysiologiste avant 1.0 (INRAE, ONF, universités ; le code ouvert facilite la relecture).

### 5.2 Budget de performance (machine cible)
| Mesure | Budget | Mesuré (2026-11-16, machine cible, 1600×900, MSAA 4×) |
|---|---|---|
| Accéléré 10 ans/s, chêne 150 ans | ≥ 30 FPS | 60 FPS en vsync (132 sans), 10 ans/s tenus, p99 ≈ 42 ms (avant : 12 à 16 FPS, 7,6 à 9,2 ans/s). Chêne à 1 an/s et merisier 100 ans à 10 ans/s : 60 FPS (214 et 221 sans vsync) |
| Vue libre, saison figée | ≥ 60 FPS | 60 FPS en vsync (166 sans), chêne 200 ans ; script 2,3 ms par image (avant 7,8), GPU 5,6 ms (inchangé) |
| Croissance 200 ans sans rendu (`performance_200_years`) | ≤ 2 s | 1,91 s |
| Rejeu au chargement | ≤ 3 s par siècle | 0,7 s (chêne de 100 ans) |
| Parcelle de 5 arbres (M5) | ≥ 30 FPS | — |
Les tests `#[ignore]` deviennent des tests de CI en release ; une régression de plus de 20 % bloque la fusion.

### 5.3 Direction artistique
- Ton : calme, peint, renouveau ; les planches d'herbier sont l'identité visuelle la plus distinctive, à mettre en avant.
- Le rendu par code reste toujours le repli : les fonds IA sont optionnels et le jeu doit être beau sans eux.
- Une palette et une typographie figées dans `ui_style.gd` ; une capture de référence par écran, comparée à chaque passe.

### 5.4 Outillage et tests
- Instantanés de référence (`examples/snapshot.rs`) comparés en CI : tout changement de forme est visible dans la revue.
- Captures automatiques `DENDRO_SHOT` des écrans clés jointes aux PR.
- Panneau de débogage (Godot) pour les grandeurs internes (réserves, potentiel hydrique, vigueur) : indispensable pour M2.

### 5.5 Communauté et open source
- Le format d'espèce RON documenté (`docs/genome.md`) est la vraie porte d'entrée des contributeurs : botanistes plutôt que programmeurs.
- Ouvrir le dépôt au plus tard à la fin de M3 (boucle jouable), avec un `CONTRIBUTING.md` qui impose la règle des sources.
- Devlog court à chaque jalon (captures en accéléré, coupe de tronc commentée).

## 6. Risques globaux

- **Périmètre** : seul, sur un jeu à la fois simulateur scientifique et jeu calme. Parade : M6 et la partie « herbacées » de M4 sont les premiers candidats à sortir de 1.0.
- **Précision contre amusement** : un modèle plus juste peut être plus lent et moins lisible. Parade : chaque mécanisme de M2 doit produire une chose visible (cerne, couleur, journal) ou il attend.
- **Déterminisme** : condition du partage, des événements rejouables et des sauvegardes compactes ; à protéger par la CI dès M0.
- **Matériel cible** : HD 7950 (GCN 1) en Forward+ ; garder en tête le moteur de rendu Compatibility si le Vulkan RADV pose problème sur d'autres vieilles cartes.

## 7. Décisions ouvertes (avec recommandation)

| Décision | Recommandation |
|---|---|
| **Licence** | Deux licences : code sous **GPLv3** (protège contre une reprise fermée, compatible avec Steam ; `docs/analyse-brutal.md` raisonne déjà comme si le code était en GPLv3, alors qu'aucun choix n'est écrit : à trancher et à corriger dans ce document), données d'espèces et textes sous **CC BY-SA 4.0**, art et audio sous **CC BY-NC-SA** ou droits réservés, pour qu'on ne puisse pas revendre le jeu tel quel. Alternative plus permissive : MPL-2.0 pour le code. |
| **Positionnement : calme ou « simulation de systèmes »** | `docs/analyse-tree-simulator.md` §4.1 et §4.5 recommandent de s'opposer au « cosy » et d'éviter les tags Relaxing/Cozy. Recommandation : garder l'ambiance calme (décision du propriétaire), mais communiquer sur la science : tags Steam Simulation, Education, Nature, Sandbox, Realistic ; pas Idler ni Wholesome. « Calme et rigoureux » est l'angle que personne n'occupe. |
| **Périmètre de 1.0** | Arbres feuillus et 1 conifère, 1 arbuste, physique du milieu (vent, pluie, neige, casse), journal, laboratoire simple (sélection dans une espèce). Herbacées, compétition multi-plantes complète et croisements entre espèces après 1.0. |
| **Génétique et sélection dans 1.0** | Oui, mais réduite : sélection et croisement intra-spécifique, héritabilité par trait. C'est ce qui justifie le nom *Seed Master* ; la version complète peut venir en mise à jour. |
| **Steam ou itch.io** | Les deux : itch.io (gratuit, versions de test, public open source) dès la fin de M3 ; Steam pour 1.0 (visibilité, Workshop). Un jeu libre peut être vendu sur Steam ; décider tôt si la version Steam est payante (les sources restent libres). |
| **Plateformes** | Linux et Windows en 1.0 ; Steam Deck si l'interface tient au gamepad ; macOS seulement si un contributeur s'en charge (signature et notarisation coûteuses). |
| **Nom du dépôt et du dossier** | `genus-loci` partout (dépôt, dossier, identifiant d'application) ; garder *Dendrologia* comme nom du prototype et nom de code du moteur. |
| **Fonds peints par IA** | Optionnels, crédités, jamais nécessaires ; vérifier la politique Steam de déclaration de contenu généré au moment de la page. |
| **Échelle de temps** | Garder le pas journalier pour la croissance (fondement des saisons et des cernes). Exception décidée : les événements physiques de M1 (rafale, chute de neige) sont résolus à part, à pas fin, puis inscrits au journal. |
| **Moteur** | Rester sur Godot (`docs/analyse-brutal.md`), avec le cœur Rust indépendant comme assurance. |
