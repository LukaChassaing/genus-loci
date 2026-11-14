# Analyse de Florasynth et implications pour Dendrologia

Date : 2026-11-14. Périmètre : analyse en lecture seule, sans inscription, connexion, paiement ni téléchargement.

## Correctif du 2026-11-14 : vérification directe dans le navigateur

L'analyse ci-dessous repose surtout sur des fils de forum de 2023-2025 et n'a pas pu lire le site.
Vérification faite ensuite sur florasynth.com (build 2.95) : **le produit actuel est une vraie simulation de croissance**, ce qui contredit le point « forme statique » plus bas.

Ce que le site annonce et que l'outil montre :

- Croissance par pousses annuelles, compétition pour la lumière, dominance apicale, tropismes, épaississement des branches, chute des branches faibles et repousse depuis les bourgeons dormants, fléchissement sous le poids.
- Forêt : semis naturel, ombrage entre voisins, obstacles.
- Outil de taille, squelette et animation de vent, niveaux de détail jusqu'au billboard, export OBJ/GLB.
- 13 espèces préréglées (dont Gambel Oak), greffon Blender, bibliothèques Python/JS/DLL. Produit payant.

Ce qui est absent de l'interface de simulation (constaté, pas déduit) :

- Aucune saison ni phénologie : le temps avance par années entières (« Grow by (years) »).
- Aucune météo, aucun sol, aucune eau : la lumière est le seul facteur du milieu.
- Aucune racine, aucun cerne, aucune coupe du tronc.
- Aucun contenu pédagogique : les espèces sont des préréglages de paramètres, sans lien entre elles.
- Lenteur mesurée : environ une seconde par année simulée pour un seul arbre de 25 à 50 ans.

Conséquence : Dendrologia ne se distingue pas par « une vraie simulation » (Florasynth l'a), mais par le temps fin (jours, saisons), le milieu (eau, froid, sol), le souterrain, les cernes, la vitesse et la pédagogie.

## 0. Limites de l'enquête (à lire d'abord)

- **florasynth.com est une application JS (SPA)** : WebFetch ne récupère que le titre "Florasynth Tree Generator" sur `/`, `/about`, `/docs`. Je n'ai donc **pas pu lire** l'application, la landing, la doc, la FAQ, le changelog, la galerie, la liste exacte des presets ni la page de prix.
- Les faits ci-dessous viennent presque tous du **fil du forum three.js** (annonce du créateur + réponses, 2023-2026) et de métadonnées de moteurs de recherche. Pas d'avis de presse indépendant, pas de Reddit/Discord/itch trouvés.
- Un snippet Context7 attribue à florasynth.com/docs la description "procedural tree simulation engine that models biological and physical growth..." (tronquée). Je n'ai pas pu lire la doc complète : **à ne pas prendre pour acquis** (c'est un texte marketing, pas une preuve de simulation réelle).
- Tout ce qui est marqué **[inférence]** est mon raisonnement, pas une source.

## 1. Ce qu'est Florasynth (vérifié)

| Point | Constat | Source |
|---|---|---|
| Nature | Outil web "built with three.js" générant modèles 3D d'arbres et de plantes ; barre d'espace = nouvelle variation ; presets d'arbres types | [1] |
| Auteur | Jacopo Folgoni Borsa, "passion project" travaillé "quand il a le temps" (annonce 30/11/2023, posts jusqu'en 2026) | [1][3] |
| Technique | Règles **maison, "pas un L-system standard"** ; l'auteur vise une croissance naturelle, dit lui-même que le jeu de règles actuel est "simpliste" et qu'une prochaine version modélisera mieux la croissance réelle (15/06/2025) | [3] |
| Paramètre d'âge | "Growth Increments (age)" : l'âge est un paramètre de forme, pas une animation. Pas d'export d'animation natif ; contournement = exporter chaque incrément et les chaîner dans Blender ; ajouté "à la roadmap" (29/05/2025) | [3] |
| Export | OBJ (exporteur maison, géométrie + UV, textures non incluses fin 2023) ; GLB/GLTF ajouté ensuite (discuté en mai 2024) avec masques d'opacité séparés | [1][2] |
| Vent | Sway par shader GLSL personnalisé (ShaderMaterial), juin 2025 ; pas d'animation squelettique exportable (essais "avec difficulté") | [1][3] |
| Licence code | **Source fermée** ; code minifié "utilisable en usage personnel tant qu'on ne le distribue pas" ; open source envisagé "peut-être plus tard" | [1][2][3] |
| Distribution payante | Un paquet npm "florasynth" ("A robust procedural tree model generator") est vendu **20 $** sur PrivJs, 119 téléchargements ; auteur "jacopo" (lien avec le site : probable, non confirmé) | [4] |
| Kit dev / plugin Blender | Mentionnés dans un résumé de la page 3 du fil (kit vendu sur le site, plugin Blender, preset "WhippingWillow") ; **une recherche séparée n'a trouvé aucune preuve** d'un kit ou plugin officiel. Statut non confirmé | [3] vs recherche |
| Prix de l'app web | Non trouvé | - |

Conclusion technique vérifiée : **générateur paramétrique de forme statique** (avec un curseur d'âge), règles propriétaires, rendu three.js, export OBJ/GLB. Aucune source ne montre une simulation de croissance dépendant de la lumière, du sol, des saisons, ni de racines.

## 2. Faiblesses et plaintes

### (a) Vérifiées (avec source)

| # | Faiblesse | Détail | Source |
|---|---|---|---|
| A1 | CPU/GPU au repos | Un utilisateur mesure CPU ~10 % -> ~70 %, GPU 0 -> ~26 % onglet au premier plan, boucle de rendu continue ; suggestion : rendu à la demande | [1] |
| A2 | Export fragile | Feuilles opaques / "blocs pleins" dans Blender ; GLTF sans canal alpha séparé donc masques d'opacité non fusionnés ; feuilles inversées en GLB (pas en OBJ), corrigé partiellement en juin 2024 | [1][2] |
| A3 | Tri de transparence | Feuilles arrière visibles à travers les feuilles avant dans three.js | [2] |
| A4 | Pas d'animation de croissance | Demandée deux fois (mai 2024, mai 2025) ; réponse : pas d'export natif | [1][3] |
| A5 | Pas de rig/skeleton auto | Wind sway exportable impossible sans auto-rigging ; un utilisateur propose de rétro-ingénierer le code minifié | [1][2] |
| A6 | Feuillage non animé individuellement | Demande de flottement par feuille, pas de solution propre | [3] |
| A7 | Source fermée, licence floue | "usage personnel" non défini (question posée sans réponse nette) ; pas d'open source | [2][3] |
| A8 | Fiabilité du kit | Texture manquante (`willow_foliage_color.png`) dans le kit dev, correctif promis (mai 2026) | [3] |
| A9 | Règles de croissance "simplistes" | Aveu de l'auteur (juin 2025) | [3] |
| A10 | Instancing / perf de scène | Un contributeur avertit que l'animation squelettique par arbre limiterait à ~10 arbres ; recommande vent en vertex shader + instancing | [2] |

### (b) Inférences (non vérifiées, à confirmer en testant l'app nous-mêmes)

- **[inférence]** Pas de réponse à l'environnement (lumière, voisins, vent dominant, sécheresse, gel) : aucune source n'en parle, l'auteur décrit des "règles", et l'âge est un curseur. Probablement pas de compétition pour la lumière ni de taille d'élagage.
- **[inférence]** Pas de racines, pas d'anneaux de croissance, pas de saisons/phénologie : aucune mention nulle part.
- **[inférence]** Reconnaissabilité par espèce limitée à des presets (ex. WhippingWillow) plutôt qu'à des données botaniques ; pas de fiche espèce, pas de contenu éducatif visible.
- **[inférence]** Qualité des jonctions et de la sinuosité inconnue ; la plainte "unique" sur le rendu concerne les textures, pas la silhouette.
- **[inférence]** Beaucoup de curseurs abstraits (un utilisateur parle de "nombreuses options UI" évoquant un "modèle de croissance élaboré") : courbe d'apprentissage possible, non mesurée.
- **[inférence]** Ni jeu, ni narration, ni mode arboretum, ni pédagogie : c'est un outil d'asset pour développeurs/artistes.
- Retours positifs réels : "très utile", "dépasse les attentes", "amusant à manipuler" [1][3]. Il ne faut pas sous-estimer la barre UX : **une touche = un arbre**.

## 3. État de l'art : la barre à franchir

| Outil / recherche | Ce qu'il fait mieux que Florasynth | À emprunter |
|---|---|---|
| **SpeedTree** (commercial) | Standard industrie : LOD, billboards, vent, bibliothèque d'espèces | Stratégie LOD + impostors ; vent par vertex shader hiérarchique (tronc/branche/feuille) |
| **The Grove 3D** (Blender, ~109 EUR en 2018, à vérifier) | Vraie **simulation année par année** : phototropisme, gravitropisme, affaissement des branches, élagage (auto + interactif), réaction à l'élagage, rejets, objets d'environnement qui bloquent/attirent/dévient la croissance | Cœur de notre boucle annuelle ; l'élagage comme action de jeu ; contraintes d'environnement comme obstacles |
| **Tree It / Sapling / EZ-Tree** | EZ-Tree : MIT, 15 presets (chêne, frêne, pin, tremble...), >50 options, GLB/PNG, 10-40 k triangles "non optimisés" ; Sapling : Weber & Penn en add-on gratuit | Presets comme point de départ de génomes ; EZ-Tree prouve qu'un web/open source gratuit séduit |
| **Weber & Penn 1995** | Paramétrage compact par niveaux de branches (longueur, courbure, split, down-angle) ; rapide | Utile pour le **mode lab** et l'initialisation de génomes, pas comme modèle principal |
| **Palubicki et al. 2009** (Self-organizing tree models) | Croissance pilotée par la **compétition pour la lumière et l'espace** (bourgeons, "shadow propagation", priorité résolue par Borchert-Honda) ; silhouettes spécifiques à l'espèce émergent | Algorithme central proposé (voir 4) |
| **Stava et al. 2014** (inverse procedural modelling) | Estime les paramètres d'une grammaire à partir de modèles | Calibrer un génome sur des mesures/photos de chêne ; idée "later" |
| **Pirk et al. 2012/2014** (plastic trees ; vent ; bois) | Réaction aux obstacles/lumière, ; modèles de bois et de croissance (le Purdue/Benes a aussi des travaux sur les racines dans un sol, Li 2023) | Plasticité, anneaux, racines à gradient de ressources |
| **Hallé-Oldeman (architectures)** | 23 modèles d'architecture (Rauh, Troll, Massart...) distinguant monopodial/sympodial, orthotrope/plagiotrope | Vocabulaire de génome : mode de ramification et réitération |
| **AMAP / GreenLab / FSPM** | Modèles fonctionnels-structurels : bilan carbone par organe, allocation, âge physiologique | Budget de carbone par année ; épaisseur par modèle du tube (pipe model) |

Dendrologia doit surtout se démarquer par ce qui n'existe **dans aucun outil gratuit** : simulation d'environnement réelle + météo + racines + anneaux + contenu pédagogique.

## 4. Exigences de conception priorisées

Légende : **P** = prototype maintenant ; **M** = prochain jalon ; **L** = plus tard.

| Prio | Exigence | Réalisation concrète |
|---|---|---|
| P | **Pas de curseur d'âge statique : simuler année par année** | Core Rust : boucle annuelle (état de l'arbre persistant) ; chaque année = pousse des bourgeons, ramification, épaississement. Fait de l'âge une *conséquence*, pas un paramètre. Corrige A4 |
| P | **Compétition pour la lumière** (Palubicki 2009) | Bourgeons apicaux/axillaires avec quantité de lumière issue d'une grille d'ombre voxel ; allocation Borchert-Honda du vigueur. Donne la couronne du chêne de façon émergente |
| P | **Génome = données (RON/TOML), pas du code** | Un chêne pédonculé = un fichier ; Prunus = variantes. Traits : angle apical, probabilité de bourgeon latent, tropismes, vigueur, sinuosité. Édité dans le lab plus tard |
| P | **Jonctions et tronc : modèle de tube continu** | Générer le squelette puis une surface unique : convolution surface / "sweep" de sections avec rayon par pipe model (r^2 enfant = r^2 parent, exposant ~2-2,5), raccord par blending des cercles de base (bifurcations à la Weber-Penn/Sapling) ; éviter les cylindres qui se coupent. Subdiviser près des fourches |
| P | **Rendu économe, pas de boucle continue** | Rendu à la demande quand rien ne bouge (leçon A1) ; budget triangles par LOD (ci-dessous) ; instancing pour les feuilles |
| P | **Feuilles : cartes alpha-testées, pas alpha-blended** | Alpha-to-coverage ou alpha-clip + sorting hors-sujet (corrige A3) ; l'atlas feuille/rameau ; feuilles groupées en touffes sur rameaux terminaux |
| M | **Phénologie pilotant le feuillage** | Machine à états journalière : dormance -> débourrement (degrés-jours ~5 °C) -> pleine feuille -> sénescence (photopériode + température) -> chute ; météo (gel tardif brûle les jeunes pousses, sécheresse = chute précoce) |
| M | **Météo/environnement** | Saison, pluviométrie, bilan hydrique du sol, gel ; efficacité photosynthétique -> budget de carbone annuel -> vigueur des bourgeons |
| M | **Racines** | Système racinaire avec sa propre croissance guidée par gradients d'eau/nutriments (cf. Pirk/Li 2023) ; vue en coupe ; ratio racines/couronne |
| M | **Anneaux de croissance** | Chaque année ajoute une couche de cambium dont l'épaisseur dépend du budget de carbone et de l'eau ; sécheresse = anneau fin ; coupe transversale lisible |
| M | **Élagage comme mécanique** | Couper une branche : cicatrisation (bourrelet), rejets/épicormiques, redistribution de vigueur (The Grove) |
| M | **LOD** | LOD0 maillage complet (<= ~15-25 k tris pour un arbre héros sur HD 7950) ; LOD1 branches simplifiées + feuilles en cartes ; LOD2 impostor/billboard ; vent par vertex shader |
| M | **UX en peu de leviers** | Mode jeu : 3-5 choix lisibles (espèce, sol, taille) ; mode lab : traits groupés (architecture, lumière, bois, feuille) avec infobulles pédagogiques ; "touche = nouvel individu" (graine aléatoire) comme Florasynth |
| M | **Exports ouverts** | glTF/GLB avec alpha en canal propre (ou atlas combiné), pas de blocage de licence ; documenter le format de génome |
| L | **Modèle FSPM complet** | Bilan de carbone par organe, réitération (Hallé-Oldeman), âge physiologique |
| L | **Calibration inverse** | Ajuster un génome à des mesures de chênes réels (Stava 2014) |
| L | **Vent physique** | Raideur par branche (Pirk) ; flottement par feuille ; ne pas passer par squelettes par arbre (A10) |
| L | **Contenu pédagogique, arboretum, narration** | Fiches espèces sourcées, comparaison côte à côte, journaux d'événements de l'arbre |

## 5. Les 5 leviers pour la crédibilité visuelle d'un chêne pédonculé

**[inférence/synthèse de pratiques connues — non issu de Florasynth]**

1. **Forme de couronne par compétition** : large, ovoïde-arrondie, branches maîtresses qui s'étalent presque horizontales et se tordent ; elle doit *émerger* de la lumière (Palubicki), pas d'un gabarit.
2. **Sinuosité des membres** : très marquée chez le chêne ; bruit de direction par entre-nœud, tropismes (phototropisme + gravitropisme faible), plus **changements de direction à chaque cycle de repousse** (réitération), et affaissement des grosses branches par poids.
3. **Géométrie des fourches** : fourches fréquentes, plusieurs axes de diamètre proche, sympodiaux ; base de branche évasée (collier de branche), tronc cannelé/lobé par fusion de rayons.
4. **Feuillage en touffes** : le chêne a ses feuilles en bouquets en bout de rameaux (lobées), vide à l'intérieur de la couronne, bords nets ; cartes de touffes orientées vers la lumière plutôt que feuilles dispersées.
5. **Écorce** : profondément sillonnée, crêtes longitudinales qui se ramifient/se fendent avec l'âge ; texture + normal map + déformation du maillage (displacement léger) avec fissures plus profondes sur vieux troncs ; transition lisse (jeune) -> sillonnée (vieux).

## 6. Positionnement

- Florasynth est un **générateur d'assets** (une forme, exportée) ; Dendrologia est une **simulation vivante** (processus, saisons, pédagogie). Ne pas concourir sur "nombre de presets" ; concourir sur **la croissance causale**.
- Gagnant clair de Florasynth à égaler : **réactivité immédiate** (espace = nouvel arbre).
- Risque : simulation + rendu sur i5-3570K/HD 7950 : plafonner nombre de bourgeons (milliers, pas dizaines de milliers), simuler par pas annuel (pas en temps réel), maillage régénéré incrémentalement.

## Sources

1. https://discourse.threejs.org/t/florasynth-procedural-tree-generator/58740 (annonce, export OBJ, perf, licence)
2. https://discourse.threejs.org/t/florasynth-procedural-tree-generator/58740?page=2 (GLB, alpha, feuilles inversées, vent)
3. https://discourse.threejs.org/t/florasynth-procedural-tree-generator/58740?page=3 (règles maison, kit, Growth Increments, texture manquante)
4. https://www.privjs.com/packages/florasynth (paquet npm 20 $)
5. https://context7.com/websites/florasynth (description tronquée "procedural tree simulation engine")
6. https://www.cgchannel.com/2025/04/ez-tree-is-a-lightweight-free-open-source-tree-generator (EZ-Tree)
7. https://www.cgchannel.com/2021/04/f12-ships-the-grove-release-10 et https://cgpress.org/archives/the-grove-2-3-adds-new-drawing-features-and-web-demo-updates.html (The Grove)
8. https://www.cs.purdue.edu/homes/bbenes/papers/Pirk12ToG.pdf et https://www.cs.purdue.edu/homes/bbenes/papers/Li23ToG.pdf (Pirk ; racines/sol)
9. Non consultés directement : Weber & Penn 1995, Palubicki 2009, Stava 2014, Hallé-Oldeman, AMAP/GreenLab -> résumés de mémoire, à vérifier avant citation.
10. Non lisibles (SPA) : https://www.florasynth.com/ , /docs , /about.
