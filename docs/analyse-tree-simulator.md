# Analyse concurrentielle : « Tree Simulator » (Eselium, Steam 5307140)

Date de l'analyse : 2026-11-14. Recherche en lecture seule (pas de connexion, achat, ni téléchargement).

Légende : **[V]** = vérifié sur une source citée en fin de document ; **[I]** = inférence ou connaissance générale non vérifiée ici.

## 0. Résumé exécutif

- **Le jeu n'est pas sorti** : page Steam « Coming soon », sans date ni prix, sans aucun avis [V]. Il n'existe donc ni retours joueurs, ni chiffres de joueurs, ni notes de mise à jour exploitables. Tout ce qu'on sait vient de la page de store (texte + API) et d'une capture d'écran fournie par l'équipe.
- **Positionnement apparent : jeu « cosy / idle / ambiance »**, pas un simulateur scientifique. Tags : Idler, Nature, 3D, Cozy, Relaxing, Wholesome, Procedural Generation, Simulation, Casual, Hobby Sim [V]. Pitch : « A living tree for the room you're in » ; usages cités : étude/travail, endormissement, méditation, streaming [V].
- **Un seul arbre par partie, un lieu réel**, 8 lieux et 8 espèces [V]. Progression par « anneaux » dépensés en « adaptations » (30+ par arbre) [V].
- **Risque de ressemblance** : réel sur le seul mécanisme « anneaux = monnaie de l'arbre de talents » ; faible sur tout le reste (génome, labo, arboretum, commissions, partage, phylogénie, coupe de tronc) qui n'apparaît pas dans la page publique [V pour l'absence dans la page ; I pour l'absence dans le jeu].
- **Conclusion** : ce n'est pas un concurrent direct par le cœur de métier, c'est un voisin d'audience (curieux de nature) avec un public « cosy » différent du public « geek systèmes ». La menace est surtout de **perception** (même mot-clé « Tree Simulator », même mécanique d'XP).

## 1. Ce qu'on a pu établir

### 1.1 Fiche Steam [V — API appdetails + page store]

| Élément | Valeur |
|---|---|
| Nom / AppID | Tree Simulator / 5307140 |
| Statut | Coming soon, date non annoncée, prix non affiché |
| Développeur / éditeur | Eselium (pied de page : « NOVA ESELIUM SYSTEMS LIMITED ») |
| Genre | Indie |
| Plateformes | Windows, macOS (Apple Silicon requis) ; **pas de Linux** |
| Langues | Anglais (interface uniquement) |
| Fonctions Steam | Solo, Steam Cloud, Family Sharing, options d'accessibilité (texte, daltonisme, caméra, sans chrono) |
| Config min. Windows | i5-4460, 4 Go RAM, GTX 750 Ti, DX11, 1 Go disque (très léger) |
| Médias | 1 bande-annonce, 16 captures |
| Site / support | Pas de site renseigné ; e-mail support@eselium.com |
| Avis, followers, joueurs | Aucun avis ; SteamDB inaccessible (HTTP 403) |

### 1.2 Contenu annoncé [V — texte de la page store]

- **Boucle** : on fait pousser « un seul arbre, branche par branche, dans un lieu réel ». Le temps passe à 6 vitesses (de l'année réelle à 15 s/an). Les arbres gagnent des **anneaux** avec l'âge ; on les dépense en **adaptations** (30+ par arbre) qui modifient la couronne (feuilles, branches, fleurs, fruits).
- **Soin** : un arbre négligé s'éclaircit et dépérit (avertissements préalables) ; « un arbre soigné ne meurt jamais » et aucun arbre ne meurt en votre absence ; un arbre mort peut être remplacé, déplacé ou gardé. Autrement dit, **l'échec est volontairement adouci**.
- **8 lieux/espèces** : chêne pédonculé (Oxfordshire), cerisier Somei-yoshino (Kyoto), érable à sucre (Québec), eucalyptus rouge de rivière (Murray), baobab (Tarangire), cyprès de Montezuma (Tule, Oaxaca), pin bristlecone (White Mountains), séquoia géant (Sierra Nevada). Chaque lieu a son climat, son calendrier, son paysage sonore ; les anneaux débloquent les lieux suivants ; petites interactions (cloche d'église, balançoire, lanterne).
- **Rendu** : 3D (tag), procédural ; branches qui fleurissent, changent de couleur, perdent leurs feuilles, plient au vent, portent les cicatrices des hivers rudes.
- **Ambiance** : audio **synthétisé en temps réel** (aucun fichier audio), météo (soleil, pluie, brume, nuit, neige, orage, saisonnier), modes jour (toujours jour / 24 min / horloge système), mode Calme (caméra orbitale), 10 thèmes, 30 FPS en arrière-plan, mode fenêtré pour travailler.
- **Aucune mention** sur la page de : génétique/traits éditables, bac à sable, mode narratif, partage de codes, racines simulées, taille/élagage, vue en coupe, contenu pédagogique explicite, modèle économique (prix, DLC, F2P).

### 1.3 Capture « Adaptations » (fournie par l'équipe, non re-vérifiée en source publique)

- Arbre lu « des racines vers le haut ». Monnaie : anneaux (« 25 rings to spend, 305 spent »).
- **Tronc commun** à toutes les espèces, 6 colonnes (Racines / Eau / Bois / Feuillage / Saisons / Récupération) avec des nœuds d'inspiration réelle (cambium, plateau racinaire, contrôle stomatique, réparation d'embolie, bois de réaction, banque de bourgeons dormants, réitération...).
- **Couronne propre à l'espèce** (ex. cerisier) débloquée à des seuils d'anneaux 20/60/130/220 ; chaque nœud = un effet chiffré (+1 an de vie de bourgeon, +20 % de récupération, +3 % tolérance à l'ombre) + une courte note botanique ; score « Splendour 46 % ».
- La page store dit « 30+ adaptations par arbre » : cohérent avec la capture [V pour le chiffre, la capture est une donnée du client].

### 1.4 Ce qu'on n'a PAS pu établir

- Moteur/technologies (SteamDB bloqué, 403) ; taille de l'équipe ; autres jeux d'Eselium ; date de sortie ; prix ; modèle économique ; démo ; newsletters, devlogs, hub communautaire, Reddit, YouTube/Twitch, presse : **aucun résultat trouvé** dans nos recherches (les résultats « Tree Simulator » remontés concernent d'autres jeux sans rapport, de Revmatek et autres).
- Le domaine eselium.com renvoie un site d'**astrologie** (thème natal) : lien avec l'éditeur Steam **non démontré** (homonymie ou même entité, on ne sait pas). Ne pas en tirer de conclusion.
- Si la croissance est une vraie simulation (hormones, carbone, hydrique) ou un système de morphologie procédurale piloté par scripts/niveaux : **non documenté**. [I] Le vocabulaire (« adaptations », « Splendour », « un arbre soigné ne meurt jamais », temps accéléré en fond d'écran) suggère un modèle de croissance procédural stylisé couplé à un système de ressources de type idle, non une simulation écophysiologique. À confirmer à la sortie.

## 2. Forces et faiblesses

### 2.1 Forces (apprendre)

| Force | Statut |
|---|---|
| Promesse claire en une phrase, cas d'usage concrets (étude, sommeil, méditation, stream) | [V] page store |
| Ancrage « lieu réel + espèce réelle + son du lieu » : très évocateur, marketable en capture | [V] |
| Accessibilité soignée (5 options Steam, thèmes, texte agrandi) et légèreté (1 Go, 750 Ti) | [V] |
| Notes botaniques courtes attachées à chaque bonus : manière élégante d'enseigner sans cours | [V pour la capture fournie] |
| Anneaux comme monnaie diégétique : cohérent avec le thème, lisible immédiatement | [V] |
| Pas d'échec punitif : public large (cosy) | [V] |

### 2.2 Faiblesses / limites (aucun avis disponible, donc tout est [I])

- Anglais seul ; pas de Linux ; pas de site ni de présence communautaire repérable.
- Un seul arbre à la fois, sans mort définitive : peu de tension, risque de rejouabilité faible une fois les 8 lieux vus.
- Effets de gameplay chiffrés sur des mécanismes réels (« +3 % tolérance à l'ombre ») : probablement des bonus de jeu, pas des résultats d'un modèle ; la rigueur scientifique est donc **présentationnelle**.
- Aucun levier de créativité/expérimentation (pas de génome ni de labo visibles).
- Aucune preuve de partage, d'écosystème ou de modding.

## 3. Recouvrement avec Dendrologia

| Fonction Dendrologia | Tree Simulator | Remarque |
|---|---|---|
| Croissance simulée à partir d'un génome de traits | Absent de la page (arbre procédural, adaptations) [I] | Différenciateur majeur |
| Saisons, sécheresse, gel | Similaire (calendrier/climat par lieu, météo, cicatrices de rudes hivers) | Chez eux surtout décoratif/ambiance [I] |
| Racines (système souterrain) | Similaire côté talents (colonne Racines) ; simulation visible non annoncée | À vérifier |
| Anneaux annuels en coupe de tronc | Anneaux = monnaie ; **coupe visible non mentionnée** | Chez eux l'anneau est une unité de score, chez nous une donnée physique |
| Élagage | Absent de la page | Soin/ « care » seulement |
| Mode arboretum (espèces à tous les âges) | Absent (8 lieux, 1 arbre) | Différent |
| Labo bac à sable (éditer traits, créer des espèces plausibles) | Absent | Fort différenciateur |
| Mode narratif / commissions | Absent | Fort différenciateur |
| Partage d'arbres par code | Absent | Différenciateur |
| Arbre de talents (perks) | **Identique dans le principe** : arbre de compétences dépensé en anneaux, tronc commun + branche d'espèce, notes botaniques | Risque de clone n°1 |
| Layout du perk tree = arbre phylogénétique | Absent (tronc commun + couronne d'espèce, mais pas phylogénie) | Différenciateur si l'exécution est réelle |
| Espèces : chêne pédonculé puis Prunus | Recoupement : chêne anglais **et** cerisier Somei-yoshino (un Prunus) dans leur 8 | Même premières espèces, risque de comparaison directe |
| 3D stylisée | Similaire (3D) | Style probablement différent [I] |
| Illustrations 2D planche naturaliste | Absent de la page [I] | Identité visuelle forte |
| Ambiance / audio génératif / relaxation | Cœur de leur offre | Ne pas les concurrencer sur ce terrain |
| Contenu pédagogique | Notes botaniques courtes | Nous : plus profond, sourcé |

**Franchement** : (1) la monnaie « anneaux » + arbre de perks avec un tronc commun, des colonnes thématiques (racines, eau, bois...) et des branches par espèce est la partie la plus proche de la capture ; si Dendrologia sortait après avec exactement cela, on passerait pour un clone. (2) Les deux premières espèces (chêne, puis Prunus) coïncident avec deux de leurs huit ; ce n'est pas copiable à l'identique, mais cela rend la comparaison inévitable. (3) Le titre « Tree Simulator » générique est déjà occupé par une série de jeux sans rapport (Revmatek, etc.) [V] : confusion de recherche garantie, à éviter pour nous.

## 4. Recommandations

### 4.1 Positionnement

- Se placer **à l'opposé de « cosy ambiance »** : « le simulateur où l'arbre est le résultat d'un génome et d'un climat, et où l'on peut démontrer pourquoi il a réussi ou échoué ». Public : Automation, Bladesong, joueurs de systèmes.
- Message à porter : « créez/éditez une espèce et regardez-la vivre 300 ans », « lisez l'histoire du climat dans la coupe du tronc », « vos commissions sont des problèmes d'ingénierie du vivant ».
- Ne pas promettre le relaxant ni l'idle ; accepter l'échec (mort d'arbre, cause lisible) : c'est exactement ce que leur page écarte.

### 4.2 À doubler

1. **Coupe de tronc comme interface de diagnostic** (largeur des cernes, bois de gel, blessures, taille). Leur anneau est un jeton, le nôtre est une mesure.
2. **Labo/génome + commissions** : la boucle « un client, un climat, un cahier des charges » est absente chez eux.
3. **Arbre phylogénétique** comme carte de déblocage : le seul élément qui transforme le perk tree en contenu scientifique distinct.
4. **Partage par code + arboretum** : valeur communautaire et pédagogique.
5. **Sources et transparence** : chaque trait avec référence et niveau de confiance (à confirmer côté contenu).

### 4.3 À rethinker ou abandonner

- **Ne pas utiliser les anneaux comme monnaie**. Alternatives : débloquer les gènes via les **commissions réussies**, des **observations/échantillons** (arboretum, relevés), des **points de recherche** liés à des preuves (ex. « avoir fait survivre un arbre à 3 gels tardifs ») ; garder les anneaux comme donnée de simulation, jamais comme jeton.
- **Ne pas reproduire** la structure « tronc commun + colonnes Racines/Eau/Bois/Feuillage/Saisons/Récupération + couronne d'espèce à seuils 20/60/130/220 ». Le layout phylogénétique évite déjà cela : le tenir comme règle.
- Ne pas afficher de score « Splendour/beauté » ni de pourcentage de bien-être générique ; préférer des indicateurs mesurables (bilan carbone, risque d'embolie, réserves).
- Ne pas déplacer le jeu vers l'idle/fond d'écran, terrain déjà occupé.

### 4.4 Emprunts légitimes (idées génériques) vs expressions à ne pas copier

| Idée générique, empruntable | Expression spécifique, à ne pas reprendre |
|---|---|
| Lier un bonus à un phénomène botanique réel avec une note d'une ligne | Leurs noms de nœuds et leur formulation (Splendour, « Petal blizzard », « Second spring », etc.), leurs valeurs |
| Lieux réels comme ancrage (climat/calendrier locaux) | Leur sélection exacte de 8 lieux/espèces et leurs scènes (Kyoto/Philosopher's Path, etc.) |
| Paysage sonore génératif | Leurs ambiances et animaux cités |
| Accessibilité complète dès le départ | Leurs thèmes et préréglages (Rainy/Night/Calm) |
| Vitesses de temps multiples | Leur échelle exacte (de 1 an réel à 15 s/an) à copier à l'identique |
| Notes pédagogiques courtes | Mise en page de l'écran « Adaptations » (lecture bas-haut, colonnes, compteur « X to spend, Y spent ») |

### 4.5 Nom et identité

- Éviter tout titre contenant « Tree Simulator » (série Revmatek existante + ce nouveau jeu). « Dendrologia » est distinctif ; garder.
- Garder les planches naturalistes 2D et une 3D moins « diorama doux » ; éviter la palette pastel cosy et la mention « relaxing/wholesome » dans les tags (utiliser Simulation, Science, Strategy/Management, Education, Genetics).
- Choisir tôt nos propres termes pour la monnaie et les nœuds (pas « adaptations », ni « rings »).
- Tags Steam visés : Simulation, Education, Nature, Management, Sandbox, Procedural Generation, Realistic ; éviter Idler/Cozy/Wholesome/Relaxing.

### 4.6 À surveiller

- Date de sortie, prix, démo, avis, premiers streams ; SteamDB (technologies, joueurs) ; devlogs. Revoir cette analyse à la sortie : une simulation réelle derrière les « adaptations » changerait l'évaluation.
- Date de notre propre annonce : l'annonce de la page Steam rapproche les deux audiences ; communiquer tôt sur le génome et les commissions.

## 5. Autres jeux voisins [I : connaissance générale, non revérifiée dans cette recherche]

- **Prune** (Joel McDonald) : élagage d'un arbre, abstrait et poétique ; référence pour la sensation de taille.
- **Botany Manor** : énigmes botaniques documentées ; ton pédagogique cosy, public proche.
- **Strange Horticulture** : gestion/énigmes de plantes avec livre de références ; bon exemple de savoir intégré au gameplay.
- **Terra Nil** : reconstruction d'écosystèmes ; pédagogie de systèmes et esthétique soignée.
- **Reus** : jeu de dieu où l'on fait croître des écosystèmes ; synergies et lisibilité.
- **The Grove 3D** (outil) : génération 3D d'arbres réalistes ; référence pour la plausibilité morphologique.
- **Jeux de bonsaï (divers)** : formation/taille esthétique ; marché cosy.
- **Jeux « Tree Simulator » (Revmatek 2020–2023, Tree Simulator 2.0)** : [V] existent sur Steam ; autre type (jeux de bac à sable/mème), mais pollution du mot-clé de recherche.
- **PolyPine, Arboreal, Heliotropism** : [V] cités par la recherche comme jeux d'arbres/forêts ; non étudiés en détail.
- **Automation / Bladesong** : modèles de ton (profondeur de simulation + créativité), pas des jeux d'arbres.

## Sources

- Page Steam : https://store.steampowered.com/app/5307140/Tree_Simulator/
- API Steam appdetails : https://store.steampowered.com/api/appdetails?appids=5307140&l=english
- API avis Steam : https://store.steampowered.com/appreviews/5307140?json=1&language=all&num_per_page=100 (réponse inexploitable : renvoyait la page d'accueil du store, aucun avis ; cohérent avec « Coming soon »)
- SteamDB : https://steamdb.info/app/5307140/ (HTTP 403, non consulté)
- Site eselium.com : https://eselium.com (site d'astrologie ; lien avec l'éditeur non établi)
- Autres « Tree Simulator » (homonymes) : https://store.steampowered.com/app/3054710 , https://store.steampowered.com/app/1591290/Tree_Simulator_2022/ , https://steamdb.info/app/1043500/
- Capture « Adaptations » : fournie par l'équipe Dendrologia (non re-vérifiable publiquement ici).
