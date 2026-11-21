# Refonte de l'interface : recherche documentaire

Document de recherche (2026-11-21), sans modification de code. Il prépare la refonte de l'interface de « Genus Loci: Dendrologia » (Godot 4.7, cœur Rust), prototype du jeu « Genus Loci: Seed Master ».

## Convention de lecture des sources

Chaque source porte un niveau de lecture, appliqué strictement :

| Niveau | Sens |
|---|---|
| **TI** | Texte intégral lu (PDF converti en texte avec `pdftotext`, puis relu par extraits ciblés). |
| **PL** | Page web lue par extraction (WebFetch). L'extraction passe par un petit modèle de résumé : fidèle sur la structure, pas mot à mot. |
| **RS** | Résumé de recherche seulement (extrait de moteur de recherche). **Aucun principe de la partie A ne repose sur une source RS.** Les RS ne sont citées que pour situer un exemple. |
| **NA** | Page inaccessible (403, 404, captcha, redirection d'authentification). |

Le détail source par source est en F. Ce qui est déduit par moi (non sourcé) est marqué **[inférence]**.

## Contexte vérifié dans le dépôt

Relevé rapide (lecture seule) de `godot/` pour fonder le diagnostic :

- `godot/project.godot` : base 1280×720, `stretch/mode="canvas_items"`, `aspect="expand"`, Godot 4.7, Forward Plus.
- Une échelle d'interface utilisateur existe déjà (`content_scale_factor` réglé depuis `settings["ui_scale"]`, dans `game.gd` et `main.gd`).
- `scripts/ui_style.gd` construit déjà un `Theme` en code et des `StyleBoxFlat` (`box`, `panel_box`, `focus_box`) : la fondation d'un thème centralisé existe.
- Les tailles de police littérales trouvées dans `godot/scripts` vont de 11 à 20 px (surtout 12-13 px). Le contraste des couleurs de `UiStyle` n'a **pas** été mesuré ici.
- 30 occurrences de « tooltip » dans les scripts : des info-bulles existent, mais leur forme n'a pas été auditée.

---

## A. Principes retenus

Dix-huit principes. Pour chacun : source, niveau de lecture, ce que cela implique pour Dendrologia.

### A1. Cohérence : supprimer ce qui n'aide pas l'apprentissage
- **Source** : Mayer R. E. (2021), « Research-Based Principles for Designing Multimedia Instruction », chapitre de *In Their Own Words*, <https://www.unh.edu/teaching-learning-resource-hub/sites/default/files/media/2023-06/itow-research-based-principles-for-designing-multimedia-instruction-mayer.pdf>. **TI**.
- **Contenu** : supprimer le matériel superflu ; 18 tests positifs sur 19, taille d'effet médiane d = 0,86. Le chapitre précise que les effets peuvent dépendre de conditions limites.
- **Implication** : aujourd'hui tout est affiché en même temps (température, vent, profondeur des racines, étalement, etc.). Chaque donnée doit justifier sa présence à l'écran *à ce moment-là*. Les autres s'obtiennent à la demande (A12).

### A2. Signalement : désigner l'essentiel
- **Source** : Mayer 2021 (même chapitre). **TI**.
- **Contenu** : mettre en évidence le matériel essentiel ; 26 tests sur 28, d médian = 0,70.
- **Implication** : quand le joueur active « Sécheresse », l'interface doit montrer *où* regarder (eau du sol, racines concernées, hausse des réserves en baisse), pas laisser vingt valeurs au même poids.

### A3. Contiguïté spatiale : l'étiquette près de la chose
- **Source** : Mayer 2021. **TI**.
- **Contenu** : placer le texte près de la partie correspondante du graphique ; 9 tests sur 9, d médian = 0,82.
- **Implication** : le DBH s'affiche *sur* le tronc (ou relié à lui), la profondeur racinaire *sur* les racines, plutôt que dans un panneau de texte distant en haut à gauche. Tension avec A7 (voir D).

### A4. Contiguïté temporelle, segmentation, pré-apprentissage
- **Source** : Mayer 2021. **TI**.
- **Contenu** : contiguïté temporelle 8/8, d = 1,31 (formulée pour paroles et images simultanées) ; segmentation (parties au rythme de l'apprenant) 7/7, d = 0,67 ; pré-apprentissage (nommer les composants avant la leçon) 10/10, d = 0,78. Le chapitre indique que le pré-apprentissage peut ne pas s'appliquer aux apprenants très experts.
- **Implication** : (a) le retour visuel d'un réglage (curseur Sol, Sécheresse) doit être immédiat et visible au même endroit **[inférence : la contiguïté temporelle est établie pour la parole, la transposition est mienne]** ; (b) le curseur de vitesse est déjà un outil de segmentation, à garder visible ; (c) un lexique très court (cambium, aubier, cerne, houppier) avant la première lecture, désactivable pour les forestiers.

### A5. Charge cognitive : réduire l'extrinsèque, la difficulté du sujet est intrinsèque
- **Source** : Nielsen Norman Group, « Minimize Cognitive Load to Maximize Usability », <https://www.nngroup.com/articles/minimize-cognitive-load/>. **PL**.
- **Contenu** : la charge intrinsèque (apprendre, suivre ses buts) ne s'élimine pas ; l'extrinsèque (clutter, typographie sans sens) est la cible ; déléguer (images plutôt que texte, valeurs par défaut).
- **Limite** : Sweller (2010 ; Sweller, van Merriënboer et Paas 2019) était l'objet demandé, mais les deux pages d'éditeur sont derrière une authentification (**NA**). La partie « Sweller » repose donc sur NN/g et sur la structure intrinsèque/extrinsèque reprise par Mayer (TI). Je ne cite pas Sweller comme source lue.
- **Implication** : la biologie de l'arbre est la charge intrinsèque (inévitable, c'est le but pédagogique) ; l'interface ne doit rien y ajouter. Ne jamais faire lire un nombre quand une forme suffit.

### A6. Échafaudage implicite : guider par la conception, pas par des consignes
- **Source** : Podolefsky N. S., Moore E. B., Perkins K. K. (2013), « Implicit scaffolding in interactive simulations: Design strategies to support multiple educational goals », arXiv:1306.6544, <https://arxiv.org/pdf/1306.6544>. **TI**.
- **Contenu** : la guidance vient des affordances, contraintes, indices (*cueing*) et retours (*feedback*) de l'outil. Les onglets successifs ajoutent de la complexité ou relâchent des contraintes, ce qui permet de retirer progressivement l'échafaudage. Les options fondamentales se placent en haut des boîtes à outils (les élèves les parcourent de haut en bas). Les contrôles communs (« Reset All ») sont sortis des panneaux, bien visibles. Dans Energy Skate Park Basics, aucun élément n'est enterré dans un menu.
- **Implication** : structurer par *niveaux de liberté* plutôt que par thèmes : un mode « Découvrir » (l'arbre, le temps, une ou deux variables), puis « Explorer » (sol, eau, vent, taille), puis « Laboratoire » (mécanique, flux détaillés).

### A7. Lignes directrices d'interface PhET
- **Source** : Adams W. K. et al. (2008), « A Study of Educational Simulations, Part II – Interface Design », *Journal of Interactive Learning Research* 19(3), <https://phet.colorado.edu/publications/archive/PhET%20interview%20Paper%20Part%20II.htm>. **PL** (consigné : environ 200 entretiens filmés sur trois ans, d'après la page de recherche, **RS** pour ce chiffre).
- **Contenu** (liste de la page) : zone de jeu dominante à gauche, panneau de contrôle à droite ; environ trois groupes d'environ trois contrôles ; contrôles avancés derrière un bouton « avancé » ; étiquettes de 1 à 3 mots, collées à leur contrôle ; **pas de texte dans la zone de jeu** ; démarrage avec peu ou pas d'animation, en mode simple ; manipulation directe plutôt que modes ; la couleur comme indice principal ; supprimer les détails incidentaux (les élèves donnent un sens à tout, même un pixel parasite) ; aide brève, non directive, à la demande ; pas de case à cocher pour *désactiver*.
- **Implication** : la barre du bas à 8 contrôles sur deux lignes dépasse largement « trois groupes de trois ». Chiffre à tester, non universel.

### A8. PhET : exploration engagée, cohérence, cacher plutôt que griser
- **Source** : Adams W. K. et al. (2008), « A Study of Educational Simulations, Part I – Engagement and Learning », <https://phet.colorado.edu/publications/archive/PhET%20interview%20Paper%20Part%20I.htm>. **PL**.
- **Contenu** : l'exploration engagée ; le « mode performance » (qui croit déjà savoir utilise la simulation comme démonstration et apprend moins) ; la cohérence ; **cacher** les contrôles inapplicables plutôt que les griser ; donner accès à tous les paramètres perçus comme importants, même faux (pour corriger les idées reçues) ; éviter le look « cahier de laboratoire » (nombres denses, courbes serrées) qui rend les élèves craintifs ; objets reconnaissables.
- **Implication** : le grand panneau de chiffres a précisément l'allure « cahier de labo ». Les forestiers peuvent être en « mode performance » (ils croient connaître) : les questions de prédiction (A10) aident.

### A9. Les explorables : documents réactifs, exemples explorables, information contextuelle
- **Source** : Victor B. (2011, postscript 2024), « Explorable Explanations », <https://worrydream.com/ExplorableExplanations/>. **PL**.
- **Contenu** : le texte devient un environnement où l'on pense ; le lecteur modifie un modèle et voit l'effet ; plusieurs représentations en parallèle ; l'élément interactif est inséré dans une explication normale que l'on peut sauter ; l'auteur guide et ne laisse pas seul dans un bac à sable ; consultation d'un contexte sans perdre sa place.
- **Implication** : chaque nombre du simulateur est sourcé : l'info-bulle riche (A16) est exactement la « consultation contextuelle » de Victor, avec la source du chiffre.

### A10. Nicky Case : question d'abord, prédiction, bac à sable à la fin
- **Sources** : Case N., « How I Make an Explorable Explanation », <https://blog.ncase.me/how-i-make-an-explorable-explanation/> et « Explorable Explanations: 4 More Design Patterns », <https://blog.ncase.me/explorable-explanations-4-more-design-patterns/>. **PL** toutes deux. (Mon essai d'une troisième page, « 4 ways to visualize data », a donné 404 : **NA**.)
- **Contenu** : commencer par une question qui intéresse, un premier contact concret, monter d'un cran à la fois ; motif « Place Your Bets » (prédire, puis voir le résultat) ; motif « Sandbox » avec une version « pataugeoire » simplifiée ou une introduction graduelle, souvent en fin de parcours ; « apprendre exige de penser ».
- **Implication** : la « Nouvelle graine » peut proposer en option « Que sera cet arbre dans 50 ans ? » (prédire la hauteur, puis comparer). Le sandbox complet existe déjà ; c'est le chemin qui y mène qui manque.

### A11. Interfaces d'articles interactifs : vue d'ensemble, puis détails à la demande
- **Source** : « Communicating with Interactive Articles », Distill, 2020, <https://distill.pub/2020/communicating-with-interactive-articles/>. **PL**. Auteurs non relevés par l'extraction (je crois qu'il s'agit de Hohman, Conlen, Heer et Chau, de mémoire, **non vérifié sur la page**).
- **Contenu** : la page cite le principe « Overview first, zoom and filter, then details-on-demand » ; segmenter ; lecture/pause/défilement au rythme du lecteur ; simulations à paramètres réglables ; « tout n'a pas besoin d'être interactif ».
- **Implication** : le schéma « arbre entier → coupe d'un organe → valeur sourcée » structure D.

### A12. Divulgation progressive : deux niveaux, séparés par l'usage
- **Source** : Nielsen J., Nielsen Norman Group, « Progressive Disclosure », <https://www.nngroup.com/articles/progressive-disclosure/>. **PL**.
- **Contenu** : afficher peu d'options, déporter les avancées ; la séparation doit se fonder sur l'analyse de tâches et les données d'usage ; **limiter à deux niveaux** (trois niveaux ou plus nuisent) ; chemin vers le niveau 2 évident et bien libellé ; éviter plusieurs chemins vers les mêmes options.
- **Implication** : tout ne peut pas être au niveau 1. Mais « deux niveaux » interdit aussi un empilement tiroir-dans-tiroir (écran → onglet → sous-menu → info-bulle).

### A13. Tableaux de bord : tout sur un écran, du contexte, sans fioritures
- **Source** : Few S. (2007), « Why Most Dashboards Fail », Perceptual Edge, <https://blogs.ischool.berkeley.edu/i247s13/files/2013/02/WhyMostDashboardsFail.pdf>. **TI** (2 pages). Les « 13 erreurs » du livre (*Information Dashboard Design*) ne sont connues que par des résumés (**RS**, non utilisées).
- **Contenu** : le défi est d'afficher l'information requise sur un seul écran, sans distraction et pour un examen rapide ; le suivi se fait en trois temps (vue d'ensemble, regard rapproché sur ce qui requiert l'attention, détails) ; une valeur sans contexte ne dit rien (« comparé à quoi ? ») ; les jauges rondes occupent de la place et informent peu ; ses *sparklines* (historique sur 12 mois) et *bullet graphs* (valeur, cible, états pauvre/satisfaisant/bon) condensent plus d'information dans le même espace ; tout ce qui n'ajoute pas de sens doit sortir. Il cite 10 % des hommes et 1 % des femmes comme daltoniens.
- **Implication** : « la réserve : 62 % » ne dit rien ; un *bullet graph* ou une sparkline sur les dix dernières années, oui. Les cernes sont déjà une sparkline naturelle (A15).

### A14. Quelle forme pour quelle donnée
- **Source** : Nielsen Norman Group, article NN/g (titre exact non relevé par l'extraction ; URL `dashboards-preattentive`) <https://www.nngroup.com/articles/dashboards-preattentive/>. **PL**.
- **Contenu** : longueur et position 2D sont les attributs préattentifs les plus efficaces pour des quantités ; la couleur sert aux catégories, pas aux quantités ; éviter secteurs, jauges radiales et 3D ; la couleur doit être doublée de forme ou de regroupement ; 4,5 % de la population a une vision des couleurs altérée (chiffre du NN/g ; il diffère de celui de Few, 10 % des hommes et 1 % des femmes : les deux populations ne sont pas la même, **je ne les concilie pas**).
- **Implication** : les barres du type « eau du sol » restent bonnes ; pas de jauges circulaires pour les réserves ; la légende des flux doit ajouter une forme (tiret, pointillé, épaisseur) à chaque couleur.

### A15. Sparklines : un mot-graphique, dans son contexte
- **Source** : Tufte E. R., « Sparkline theory and practice », <https://www.edwardtufte.com/notebook/sparkline-theory-and-practice-edward-tufte/>. **PL**.
- **Contenu** : « petit graphique intense, simple, de la taille d'un mot » ; il se glisse dans une phrase ou un tableau ; une valeur isolée prend sens contre son historique ; proportions allongées (environ 5:1) ; peu d'encadrés ; sparklines en rangées pour comparer (*small multiples* au sens où Tufte les utilise sur cette page).
- **Limite** : la définition générale du *data-ink ratio* et des *small multiples* (*The Visual Display of Quantitative Information*) n'est connue que par recherche (**RS**) : je ne m'en sers pas comme principe. L'essai sur `edwardtufte.com/tufte/sparkline_theory` a donné 404.
- **Implication** : à côté de « Hauteur : 14,2 m » : une courbe de 40 ans en 60×16 px. À côté de « Réserves » : leur tendance saisonnière. Rend la dimension temporelle (que le texte seul n'a pas) visible en place.

### A16. Taxonomie de l'interface de jeu : espace, fiction, signifiants
- **Source** : Fagerholt E., Lorentzon M. (2009), « Beyond the HUD: User Interfaces for Increased Player Immersion in FPS Games », Chalmers, <https://publications.lib.chalmers.se/records/fulltext/111921.pdf>. **TI** (conversion `pdftotext`, chapitres 7 et 10 relus).
- **Contenu** : l'espace de conception croise deux dimensions : la *spatialité* (l'élément existe-t-il dans la géométrie 3D ?) et la *fiction* (existe-t-il dans le monde fictif ?). Six catégories : **non diégétique** (HUD en surimpression), **méta-perception** (état interne rendu comme perception, ex. sang à l'écran), **méta-représentation** (objet fictif présenté hors de l'espace, ex. le téléphone de GTA IV), **géométrique** (dans la 3D sans appartenir à la fiction, ex. contours, *runner vision*), **diégétique**, et **signifiants** (sous-groupe du diégétique : indices subtils à interpréter, ex. fumée d'une arme qui va casser). Quatre lignes directrices : *Know Your Design Space*, *Know Your Game*, *Establish Player Agency*, *Strengthen the Player-Avatar Perceptual Link*. Je n'ai pas retrouvé dans le texte l'affirmation, vue dans un résumé de recherche, selon laquelle la cohérence du thème « est un déterminant plus fort que le type d'affichage » : **je l'écarte**. Le texte parle bien de la cohérence du monde de jeu (Juul), pas de cela.
- **Limite** : thèse sur les FPS ; la transposition est mienne **[inférence]**. Le vocabulaire « spatial / méta » de la demande diffère un peu des catégories de la thèse (je suis la thèse).
- **Implication** : Dendrologia dispose d'un matériau idéal pour les **signifiants** : feuillage flétri ou jauni, houppier clairsemé, écoulement de sève, exsudats, cernes étroits. Les **éléments géométriques** (colorier la zone racinaire selon l'eau, surligner la branche à tailler) remplacent des panneaux. Le **non diégétique** (HUD) est réservé aux nombres exacts. Le « tout panneau » actuel est la case « non diégétique » seule.

### A17. Vues superposées : un thème à la fois, ouvertes par le contexte
- **Sources** : (i) Oxygen Not Included, wiki, « Overlays », <https://oxygennotincluded.wiki.gg/wiki/Overlays> : **PL** (16 superpositions : oxygène F1, énergie F2, température F3, matériaux F4, lumière F5, etc. ; certaines se débloquent par la recherche ; la page ne dit pas comment la scène est recolorée). (ii) Colossal Order, « Dev Diary 9 – Info Views » (Cities: Skylines, 2015), via <https://beyondsims.com/2015/02/skylines-dev-diary-9-info-views/> : **PL, extrait seulement** (19 vues, trois familles ; les vues de services s'ouvrent aussi avec le menu de construction correspondant). Mon essai sur le wiki Paradox donne **NA**.
- **Implication** : un seul thème visible à la fois, avec sa légende ; l'overlay s'ouvre tout seul quand le joueur agit sur le sujet (Sécheresse ouvre « Eau », Taille ouvre « Structure »), comme les vues de services de Cities: Skylines avec le menu de construction. Les overlays se débloquent avec la progression (ONI) : c'est un levier pour le mode Seed Master. **[inférence, non sourcée]** : sur HD 7950, un overlay doit recolorer les matériaux existants (paramètre de shader ou *uniform*), pas ajouter une passe de rendu.

### A18. Info-bulles structurées avec état vivant ; encyclopédie en un clic
- **Sources** : Wube Software, Factorio « Friday Facts #318 – New Tooltips » (2019), <https://factorio.com/blog/post/fff-318> et « #397 – Factoriopedia » (2024), <https://factorio.com/blog/post/fff-397> : **PL** toutes deux. Nielsen Norman Group, « Tooltip Guidelines », <https://www.nngroup.com/articles/tooltip-guidelines/> : **PL**.
- **Contenu** : FFF #318 : propriétés regroupées en catégories, tri pour le balayage visuel, suppression de l'inutile, compacité, **état courant** affiché (carburant, fluide). FFF #397 : une encyclopédie de pages navigables, Alt+clic gauche saute à la fiche, Alt+flèches parcourent l'historique. NN/g : brièveté, déclenchement souris *et* clavier, ne pas y mettre l'information indispensable (elles disparaissent), contraste suffisant, ne pas masquer ce qu'elles décrivent. Remarque du NN/g : cet article ne traite pas les info-bulles riches.
- **Implication** : l'info-bulle de « Réserves » montre l'état vivant (valeur, tendance, seuil de stress), la source du chiffre et un lien vers la fiche de la section « sources » ; en Godot : `_make_custom_tooltip` (A19). Une fiche « Pourquoi ? » navigable est l'équivalent de Factoriopedia, par espèce.

### A19. Accessibilité mesurable : contraste, couleur, taille
- **Sources** : W3C, WCAG 2.2, SC 1.4.3 Contrast (Minimum) <https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html> ; SC 1.4.11 Non-text Contrast <https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html> ; SC 1.4.1 Use of Color <https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html>. **PL** toutes trois. Valve, « Steam Deck Compatibility Review Process », <https://partner.steamgames.com/doc/steamdeck/compat>. **PL**.
- **Contenu** : texte 4,5:1 (3:1 si grand, soit environ 24 px, ou 18,5 px en gras ; seuils stricts, 4,499 échoue) ; éléments d'interface et objets graphiques utiles 3:1 ; la couleur ne doit pas être le seul moyen de distinguer (doubler par forme, texte ou icône ; une différence de luminosité à 3:1 compte comme indice supplémentaire) ; Steam Deck : texte lisible à 30 cm, **au moins 9 px de haut à 1280×800, 12 px recommandés**, taille de texte et contraste réglables recommandés ; Valve préfère 1280×800 ou 1280×720 ; 30 ips à 800p ; entièrement jouable à la manette.
- **Implication** : audit chiffré à faire (texte actuel à 11-13 px, donc **au-dessus du minimum de 9 px mais sous le confort de 12 px dans plusieurs cas**) ; viser 12 px minimum, davantage pour les chiffres clés) ; légende des flux : couleur + forme ; focus visible à 3:1 (le `focus_box` de `ui_style.gd` existe, à mesurer) ; navigation manette si Steam Deck est un objectif (non demandé, voir E).
- **Serif** : la recherche donne un résultat nuancé (revue de Richardson, **RS** uniquement) : pas de vainqueur net entre empattements et bâtons à l'écran, sauf peut-être très petit corps ou faible luminance. Je n'ai pas lu ce livre. Ne pas bannir le serif ; l'utiliser pour les titres et grands corps, et tester le petit corps (légendes, chiffres) en pratique.

### A20. Godot 4 : thème unique, conteneurs, échelle, info-bulles
- **Sources** : documentation Godot stable : *GUI skinning* <https://docs.godotengine.org/en/stable/tutorials/ui/gui_skinning.html> ; *Multiple resolutions* <https://docs.godotengine.org/en/stable/tutorials/rendering/multiple_resolutions.html> ; *Using Containers* <https://docs.godotengine.org/en/stable/tutorials/ui/gui_containers.html> ; classe `Control` <https://docs.godotengine.org/en/stable/classes/class_control.html> (200 000 caractères, **lue sur ses 200 000 premiers**). **PL**. Elle est la doc *stable*, pas spécifiquement 4.7 : à vérifier.
- **Contenu** : un `Theme` central (types, variantes de type, ordre de résolution), surcharges locales avec parcimonie, options d'accessibilité (police, daltonisme) dans un thème pour qu'elles se propagent ; `FoldableContainer` (repliable), `TabContainer`, `SplitContainer`, `PanelContainer`, `MarginContainer` ; recommandation desktop : base 1920×1080, `canvas_items`, `expand` ; `Keep Width` convient aux HUD ancrés en bas ; `content_scale_factor` réglable à l'exécution ; `_make_custom_tooltip(for_text)` renvoie un `Control` dans un `PopupPanel` stylable via `TooltipPanel` (il se réduit à sa taille minimale ; renvoyer `null` pour l'info-bulle par défaut) ; délai réglé par `gui/timers/tooltip_delay_sec`.
- **Implication** : tout l'UI en un seul `Theme` (le dépôt a déjà `UiStyle` à cheval entre `Theme.new()` et des appels `box()` locaux : à unifier en variantes de type) ; panneaux en conteneurs, pas en positions manuelles ; le projet est en base 1280×720 alors que la doc recommande 1920×1080 pour desktop : décision à prendre (E). Le panneau « Flux de l'arbre » est un candidat naturel pour `FoldableContainer` (présent dans la doc stable lue ; disponibilité en 4.7 à confirmer).

---

## B. Exemples de jeux et de logiciels

Les principes de la partie A ne s'appuient que sur les sources PL/TI. Les lignes « RS » ne servent qu'à situer ; **je n'affirme rien sur leur conception**.

| Exemple | Ce qui est réellement établi (niveau) | Transposable à Dendrologia |
|---|---|---|
| **PhET** (simulations de l'Université du Colorado) | Lignes directrices publiées, échafaudage implicite (TI/PL, A6-A8). Pages : <https://phet.colorado.edu/>. La page « about/research » a donné 404 (NA). | Le modèle direct : zone de jeu dominante, 3 groupes de 3 contrôles, onglets de complexité croissante, pas de texte dans la scène. |
| **Oxygen Not Included** : superpositions | 16 overlays, touches F1 à F11 et Maj+F1 à F4, certains débloqués par la recherche (PL, A17). Capture : à chercher sur <https://oxygennotincluded.wiki.gg/wiki/Overlays> (je n'en ai pas ouvert). | Un thème à la fois (Eau, Flux, Réserves, Mécanique, Vent) ; déblocage progressif en mode jeu. |
| **Cities: Skylines** : vues d'information | 19 vues, 3 familles, ouverture avec le menu de construction (PL extrait, A17). Pages : <https://beyondsims.com/2015/02/skylines-dev-diary-9-info-views/>. « 33 vues » de Cities: Skylines II : RS, non utilisé. | Overlay ouvert par l'action : Sécheresse ouvre « Eau ». |
| **Factorio** : info-bulles et Factoriopedia | Info-bulles structurées, état vivant, compacité ; encyclopédie navigable (PL, A18). | Info-bulles de chaque chiffre avec valeur, tendance, source ; fiche par espèce. |
| **Explorables** (Victor, Case, Distill) | PL (A9-A11). | Prédire puis voir ; sandbox en fin de parcours ; détails à la demande. |
| **OpenAlea** | Page d'accueil lue : projet libre pour l'architecture végétale, bibliothèques Python pour analyser, visualiser et modéliser la croissance ; la page ne décrit ni la programmation visuelle ni PlantGL (PL, faible). <https://openalea.readthedocs.io/en/latest/>. | Rien de transposable établi sur l'interface. |
| **GroIMP** | Page wiki lue : modélisation, visualisation et interaction intégrées ; scènes éditables, rendu OpenGL, lancer de rayons (PL). <https://wiki.grogra.de/doku.php?id=start>. | Idée de scène que l'on édite et que l'on modifie en direct ; peu sur l'agencement de l'interface. |
| **AMAPstudio** (Xplo, Simeo) | Seulement par recherche : suite d'édition et de simulation de l'architecture des plantes sur MTG ; open source (RS). HAL (NA, page Anubis), page amapstudio.cirad.fr non lue. | Rien d'affirmé. |
| **Dendrochronologie** (CooRecorder, CDendro, dplR) | Seulement par recherche (RS), sauf le graphique en squelette (PL ci-dessous). | Rien d'affirmé hors du squelette. |
| **Graphique en squelette (skeleton plot)**, Université de l'Arizona | PL : <https://www.ltrr.arizona.edu/skeletonplot/plotting.htm> : on ne marque que les cernes remarquables, trait d'autant plus long que le cerne est étroit, « b » pour les très larges, marques spéciales pour cernes absents. | Vue de cernes qui met en avant les années de crise (sécheresse, tempête, scolytes) plutôt que toutes les largeurs : très lisible et pédagogique pour forestiers. |
| **Dwarf Fortress** (version Steam), **Tiny Glade**, **Terra Nil**, **Kerbal Space Program**, **Mini Metro**, **Eco**, **Townscaper** | Seulement des résumés de recherche (RS) ou des pages d'accroche (Mini Metro : annonce de la conférence GDC « When Less Is More », PL très mince : <https://www.gamedeveloper.com/design/attend-gdc-2017-and-learn-the-value-of-less-is-more-from-a-i-mini-metro-i-dev>). **Aucune affirmation de conception.** Les articles PC Gamer sur la refonte de Dwarf Fortress donnent seulement une accroche (« à la souris, avec des boutons »). Eco et Townscaper : non recherchés à fond. | À documenter par captures lors d'une seconde passe (voir E). |
| **Game UI Database** | <https://www.gameuidatabase.com/> : **NA** (403). | À consulter à la main : recherche par jeu et par type d'écran. |

---

## C. Diagnostic de l'interface actuelle

Le diagnostic s'appuie sur la description de l'interface fournie et sur A. Il ne repose pas sur des sources RS.

1. **Un seul niveau : tout est à plat.** Panneau texte, cernes, flux, barre à deux lignes, légende, raccourcis, le tout visible en même temps. NN/g (A12) recommande deux niveaux avec un premier niveau court ; PhET (A7) vise environ trois groupes de trois contrôles ; la barre du bas en compte huit sur la ligne 1, plus quatre sur la ligne 2, soit plus de quatre fois la cible. Chiffre de PhET à prendre comme ordre de grandeur, pas comme loi.
2. **L'arbre, sujet de la simulation, n'occupe qu'environ un tiers de l'écran.** PhET place la zone de jeu comme élément dominant (A7). Le KSP, Cities, ONI laissent l'espace central aux objets d'étude (RS, non utilisés comme preuve). Conséquence pédagogique : l'architecture, l'effet de la taille, le houppier restent difficiles à regarder.
3. **Panneau « cahier de laboratoire ».** Une colonne de lignes de nombres dans un coin : c'est l'image que PhET dit décourageante (A8) ; chaque valeur est isolée de son contexte (Few, A13 : « comparé à quoi ? ») ; peu de signalement (A2) : la température est au même poids que l'état de l'arbre.
4. **Distance entre le nombre et la chose (contiguïté spatiale, A3).** Le DBH, la hauteur, la profondeur racinaire sont dans un coin, les organes dans la scène. Le joueur doit faire des allers-retours oculaires.
5. **Aucune hiérarchie d'attention (signalement, A2 ; préattentif, A14).** Pas de forme ni de position pour dire « ceci demande ton attention maintenant » ; la couleur dorée des liserés est identique partout, donc ne signale rien.
6. **Les cernes et les flux sont des panneaux permanents.** Ce sont des vues de niveau 2 (détails à la demande, A11), présentées comme du niveau 1. Les cernes sont une série temporelle : une sparkline (A15) ou un squelette (B) aurait un meilleur rendement par pixel.
7. **Légende des flux par la couleur seule** (à vérifier, d'après la description) : risque pour WCAG 1.4.1 et pour le daltonisme (A14, A19).
8. **Taille du texte et contraste non mesurés.** Polices de 11 à 13 px prédominantes dans le code, sur fond vert sombre avec liseré doré. Au-dessus du minimum Steam Deck de 9 px, sous la recommandation de 12 px pour une partie (A19). Les contrastes n'ont pas été calculés : **à mesurer avant toute refonte esthétique**. Le serif n'est pas en soi un défaut (A19, RS).
9. **Peu de guidage implicite, aucun niveau de découverte (A6, A10).** Le premier contact est le même pour un curieux et pour un arboriste. Pas de mode simple, pas de prédiction, pas de question d'amorce.
10. **Pas d'usage du « langage de l'arbre » (A16).** Les signifiants (feuillage flétri, houppier, exsudats) et les éléments géométriques (zones colorées sur les racines) sont les moyens diégétiques/géométriques d'afficher l'état. L'interface actuelle utilise presque uniquement la case « non diégétique ».
11. **Interactions cachées derrière des lettres** (F, R, X, G, T) : peu découvrables, non sourcés ici ; la page de Factorio sur alt-mode (RS) laisse penser que c'est un problème connu des jeux à raccourcis, mais je ne m'appuie pas dessus. Les cases à cocher « désactivantes » sont déconseillées par PhET (A7), ce qui concerne les boutons à bascule Repères/Cernes/Flux.
12. **Beaucoup de surfaces, un seul thème.** Écran titre, configuration, pause, sauvegardes, outil de taille : cohérence à auditer ; `ui_style.gd` existe mais l'unification en variantes de type n'a pas été vérifiée (A20).
13. **Information sourcée mais non consultable.** « Chaque chiffre est sourcé » est un atout rare, aujourd'hui invisible dans l'interface : le joueur ne le voit pas (A9, A18).

---

## D. Directions de refonte

Les trois directions **prennent des partis différents** sur trois tensions réelles entre les sources :

- **T1 : Mayer (contiguïté spatiale, d = 0,82) vs PhET (« pas de texte dans la zone de jeu »).** Étiqueter dans la scène, ou garder la scène propre ?
- **T2 : Few (tout sur un écran) vs NN/g (divulgation progressive, deux niveaux).** Densité d'abord, ou profondeur d'abord ?
- **T3 : PhET « rien d'enterré dans des menus » (Energy Skate Park) vs PhET « avancé derrière un bouton ».** Tout accessible, ou une couche avancée ?

Échelle des schémas : 1 colonne ≈ 13 px, 1 ligne ≈ 26 px (1280×720 ≈ 98 × 28).

### Direction 1 : « Arbre plein écran, HUD minimal, panneaux à la demande »

**Partis pris** : T1 vers PhET (scène propre, peu de texte ; les étiquettes n'apparaissent qu'au survol ou en mode « Repères »), T2 vers NN/g (deux niveaux stricts : HUD, puis panneaux), T3 vers « avancé derrière un bouton » (Mécanique, Flux détaillés).

```
+--------------------------------------------------------------------------------------------+
| Chêne sessile - 42 ans - 14 juin - Été          [Cernes]  [Flux]  [Sol]  [Mécanique]   [≡] |  <- 1 ligne, 32 px
|                                                                                            |
|                                                                                            |
|                                       .  ,  ' .                                           |
|                                    ,  '  _/\_  ' ,                                        |
|                                   '  _/        \_  '                                      |
|                                    ,'     ||     ',                                       |
|                                  .   '    ||    '  .                       (panneau à la   |
|                                           ||                                demande, 300 px|
|                                           ||                                ex. Cernes)   |
|                                           ||        .- - - - - -.          +-------------+ |
|                                      ~~~~~||~~~~~   : étiquette  :          | coupe 3D    | |
|                                    ~~ racines  ~~    : DBH 48 cm  :          |  + sparkline| |
|                                  ~~~~~~~~~~~~~~~~~   '- - - - - -'          +-------------+ |
|                                                      (au survol du tronc)                  |
|                                                                                            |
| [>||] vitesse: ----o---- 1 mois/s        [Sécheresse]   [Taille]          Eau du sol ▇▇▇▇▁ |  <- 1 ligne, 40 px
+--------------------------------------------------------------------------------------------+
```

- **Niveau 1 (toujours)** : identité (espèce, âge, date, saison), temps (lecture, vitesse), 2 à 3 actions de jeu (Sécheresse, Taille, et Tempête quand elle existe), une jauge d'eau du sol. Sélecteurs Espèce/Terrain/Graine déplacés dans « ≡ » (menu), car ils ne servent qu'au début d'une partie.
- **Niveau 2 (à la demande)** : un seul panneau latéral à la fois, de 300 px, ouvert depuis la barre du haut ; le survol d'un organe affiche une info-bulle riche (valeur, tendance, source).
- **Avantages** : arbre à ~80 % de l'écran ; la charge initiale chute ; implémentation la plus proche de l'existant (`PanelContainer`, `FoldableContainer`, info-bulles `_make_custom_tooltip`) ; bonne pour le grand public.
- **Compromis** : les données de forestiers (hauteur, DBH, réserves) ne sont plus visibles d'un coup d'œil (T2 tranché contre Few) ; risque de « mystère » si les panneaux sont mal annoncés (PhET : les élèves manquent les petits onglets, A7 : il faut des onglets grands et lisibles) ; deux clics pour comparer deux vues.

### Direction 2 : « Tableau de bord scientifique à onglets »

**Partis pris** : T1 vers Mayer (étiquettes contiguës : chaque valeur à côté de son graphique, dans le panneau, sparkline incluse), T2 vers Few (tout sur un seul écran, par onglet), T3 vers « rien d'enterré » (tout est atteignable en un clic via les onglets, sans couche « avancé »).

```
+-----------------------------------------------------+--------------------------------------+
|                                                     | [Arbre] [Cernes] [Flux] [Sol] [Méca.]|
|                                                     +--------------------------------------+
|                                                     | Chêne sessile      42 ans - 14 juin |
|                      .  ,  ' .                      |                                      |
|                   ,  '  _/\_  ' ,                   | Hauteur   14,2 m  ~~~~/~~  (40 ans)  |
|                  '  _/        \_  '                 | DBH       48 cm   ~~~~/~~            |
|                   ,'     ||     ',                  | Réserves  62 %    ▇▇▇▇▇▇▁▁ | seuil   |
|                 .   '    ||    '  .                 | État      Bon     ~~\_/~~~~            |
|                          ||                         |  ------------------------------------|
|                          ||                         | Racines   2,1 m prof. - 6 m étal.    |
|                     ~~~~~||~~~~~                    | Sol  eau  ▇▇▇▇▁   T° 18 °C  vent 3 m/s|
|                   ~~ racines  ~~                    |  ------------------------------------|
|                 ~~~~~~~~~~~~~~~~~                   | (chaque ligne : info-bulle riche     |
|                                                     |  avec source et tendance)            |
|                                                     |                                      |
|   [>||] ----o---- 1 mois/s     [Sécheresse] [Taille]| Espèce [Chêne ▾] Terrain [Argile ▾]  |
+-----------------------------------------------------+--------------------------------------+
        ~ 60 % de la largeur (770 px)                         ~ 40 % (510 px)
```

- **Onglet « Arbre »** : cartes à sparklines et *bullet graphs* (A13, A15) ; l'onglet « Cernes » remplace le panneau en haut à droite et peut offrir la vue « squelette » (B) ; « Flux » : légende avec formes ; « Méca. » : l'actuel panneau Mécanique. Les onglets sont grands, en haut à gauche du panneau (PhET, A7).
- **Avantages** : cohérence avec les exigences des forestiers et étudiants (valeurs, tendances et sources toujours visibles) ; les données temporelles (sparklines) ajoutent une dimension qui manque aujourd'hui ; structure la plus simple à tester par captures (un onglet = un écran) ; maintenance simple pour un seul développeur (un `TabContainer` + cartes réutilisables).
- **Compromis** : l'arbre ne gagne qu'une fraction de l'écran (~60 % de la largeur) : c'est le moins « diégétique » (case non diégétique de A16) ; risque de ressembler à un « cahier de laboratoire » (A8) si les cartes sont denses ; le curieux voit du jargon ; difficile à décliner en Seed Master (jeu) où l'arbre doit dominer.

### Direction 3 : « Superpositions thématiques à la Oxygen Not Included »

**Partis pris** : T1 vers Mayer, mais *dans la scène* (la scène elle-même est recolorée : l'information est portée par l'arbre, pas par du texte ; signifiants et éléments géométriques de A16), T2 vers NN/g (un thème à la fois, deux niveaux : overlay, puis info-bulle), T3 vers « avancé derrière un bouton » (les overlays avancés se débloquent, comme dans ONI, A17).

```
+--------------------------------------------------------------------------------------------+
| Chêne sessile - 42 ans - 14 juin - Été                                                     |
|                                                                                            |
|  OVERLAYS            (scène : arbre recoloré selon l'overlay actif)                        |
|  +----+                                                  .  ,  ' .                         |
|  | Eau|<-- actif                                      ,  '  _/\_  ' ,                      |
|  +----+                                              '  _/        \_  '                    |
|  |Flux|                                               ,'     ||     ',                     |
|  +----+                                             .   '    ||    '  .                    |
|  |Rés.|                                                      ||                            |
|  +----+                                                      ||         LÉGENDE            |
|  |Vent|                                                 ~~~~~||~~~~~     ▇ sec    (tiret)  |
|  +----+                                               ~~ racines  ~~     ▇ humide (plein)  |
|  |Cern|                                             ~~~~~~~~~~~~~~~~~    ▇ saturé (pointillé)|
|  +----+                                                                                   |
|  |Méca|  [info-bulle contextuelle sur l'élément pointé : valeur, tendance, source]         |
|  +----+                                                                                   |
|                                                                                            |
| [>||] vitesse ----o---- 1 mois/s       [Sécheresse] [Taille]       Hauteur 14,2 m ~~/~    |
+--------------------------------------------------------------------------------------------+
```

- **Principe** : un thème à la fois, avec légende (forme + couleur, A19) ; l'action ouvre l'overlay pertinent (Sécheresse ouvre « Eau », Taille ouvre « Structure ») sur le modèle de Cities: Skylines (A17). Les overlays secondaires peuvent se débloquer selon un mode « jeu » ou selon une progression (PhET : échafaudage retiré peu à peu, A6).
- **Avantages** : l'arbre reste le sujet (~85 % de l'écran) ; la plupart des nombres deviennent des couleurs et formes sur l'arbre : la contiguïté spatiale est maximale (A3) ; exploite signifiants et éléments géométriques (A16) ; le plus proche de « Seed Master » (jeu de gestion).
- **Compromis** : la plus lourde à réaliser (shaders de recoloration par overlay, légendes, coordination avec les panneaux de cernes et de mécanique) ; **[inférence non sourcée]** sur la HD 7950, il faut recolorer les matériaux existants et non ajouter de passes ; moins d'accès direct aux valeurs numériques (il faut survoler : mauvais pour les forestiers pressés) ; collisions avec les touches actuelles (F cadrer) si l'on adopte F1-F10 à la ONI ; surcoût pour chaque nouvelle variable.

### Synthèse comparative

| | D1 Arbre d'abord | D2 Tableau de bord | D3 Overlays |
|---|---|---|---|
| Part de l'écran pour l'arbre | ~80 % | ~60 % | ~85 % |
| Public le plus servi | Grand public | Forestiers, étudiants | Grand public, joueurs |
| Valeurs numériques visibles | Au survol | Toujours | Au survol |
| Coût de réalisation | Faible à moyen | Moyen | Élevé |
| Lien avec Seed Master | Bon | Faible | Fort |
| Risque principal | Fonctions introuvables | « Cahier de labo » | Complexité des shaders |

Variante hybride **[mon avis, non sourcé]** : D1 comme squelette (HUD + panneaux), avec D3 pour deux ou trois overlays seulement (Eau, Flux), et les cartes à sparklines de D2 à l'intérieur des panneaux. À trancher en E.

---

## E. Questions à trancher

1. **Public principal de la première version** : grand public, ou forestiers/étudiants ? (Détermine D1 ou D2.)
2. **Dendrologia est-il un prototype jetable ou la base visuelle de Seed Master ?** Si c'est la base, D3 devient pertinent dès maintenant.
3. **Faut-il garder la densité numérique visible en permanence pour les professionnels ?** Option : un mode « Expert » qui affiche le panneau de cartes (D2) par-dessus D1.
4. **Cible Steam Deck et manette ?** Si oui : navigation manette de bout en bout, texte à 12 px minimum, bases de focus. Non demandé jusqu'ici.
5. **Base de résolution** : rester sur 1280×720 (cible machine et Deck) ou passer à 1920×1080 comme le conseille la doc Godot pour desktop (A20) ? Les tailles de police seraient à reconvertir.
6. **Raccourcis clavier** : accepter F1-F10 pour d'éventuels overlays (collision avec F « cadrer » actuel) ou garder des lettres ?
7. **Style visuel** : conserver vert sombre et liseré doré ? Les contrastes n'ont pas été mesurés ; voulez-vous un audit WCAG avant de décider ? Serif conservé pour les titres seulement ?
8. **Mode découverte** : voulez-vous un parcours guidé (question d'amorce, prédiction, pré-apprentissage du lexique, A4/A10), ou le bac à sable immédiat ?
9. **Déverrouillage progressif** (overlays, panneaux) : seulement en mode jeu, ou aussi dans le simulateur ?
10. **Seconde passe sur les exemples** : voulez-vous que j'aille consulter, ouvrir et capturer Game UI Database, Dwarf Fortress, Tiny Glade, Terra Nil, KSP, Eco, Townscaper (sources non lues ici), et AMAPstudio, CooRecorder, dplR ?
11. **Langue et fiches « Pourquoi ? »** : voulez-vous une fiche navigable par chiffre (comme Factoriopedia), avec la source de `docs/sources.md` ?

---

## Décisions de l'utilisateur (2026-11-21)

- **Direction : hybride.** Le squelette suit D1 : arbre à environ 80 % de l'écran, une barre fine en haut et une en bas, un seul panneau à la demande à la fois. On y ajoute deux ou trois overlays D3 (Eau et Flux au moins, avec une légende qui combine forme et couleur) et, à l'intérieur des panneaux, les cartes à sparklines et *bullet graphs* de D2.
- **Statut : base de Seed Master.** L'interface est conçue pour être reprise dans le jeu : overlays, déblocages possibles plus tard.
- **Steam Deck et manette : non**, pas dans cette refonte (souris et clavier).
- **Mise en œuvre (2026-11-21)** : barres haute et basse, un seul panneau de niveau 2 (`panel_slot.gd` : Cernes, Flux, Sol, Mécanique, Taille), calques Eau (paramètre `water_view` du shader du sol : sec hachuré, humide plein, saturé à pois) et Flux (une forme par sorte de flux, dans la scène et la légende), mode Expert mémorisé (cartes à sparklines sur l'historique de la partie, `game_history.gd`), thème unique `UiStyle.theme()` à variantes, contrastes et police >= 12 px vérifiés par le selftest. Reste à faire : sources par valeur (non exposées par la liaison), historique conservé au chargement, échelle 85 % sous 12 px réels.
- **Premier contact : bac à sable et mode Expert.** On arrive directement dans la simulation avec le HUD minimal ; le mode Expert affiche en permanence les cartes de valeurs.

## F. Bibliographie

### Lues en texte intégral (TI)
- Podolefsky N. S., Moore E. B., Perkins K. K. (2013). *Implicit scaffolding in interactive simulations: Design strategies to support multiple educational goals*. arXiv:1306.6544. <https://arxiv.org/pdf/1306.6544>
- Fagerholt E., Lorentzon M. (2009). *Beyond the HUD: User Interfaces for Increased Player Immersion in FPS Games*. Mémoire de master, Chalmers University of Technology. <https://publications.lib.chalmers.se/records/fulltext/111921.pdf>
- Mayer R. E. (2021). *Research-Based Principles for Designing Multimedia Instruction*, chapitre dans *In Their Own Words: What Scholars and Teachers Want You to Know About Why and How to Apply the Science of Learning in Your Academic Setting* (Overson, Hakala, Kordonowy, Benassi, éd.). <https://www.unh.edu/teaching-learning-resource-hub/sites/default/files/media/2023-06/itow-research-based-principles-for-designing-multimedia-instruction-mayer.pdf>
- Few S. (2007). *Why Most Dashboards Fail*. Perceptual Edge. <https://blogs.ischool.berkeley.edu/i247s13/files/2013/02/WhyMostDashboardsFail.pdf>

### Pages lues par extraction (PL)
- Adams W. K. et al. (2008). *A Study of Educational Simulations, Part I* et *Part II*. <https://phet.colorado.edu/publications/archive/PhET%20interview%20Paper%20Part%20I.htm> ; <https://phet.colorado.edu/publications/archive/PhET%20interview%20Paper%20Part%20II.htm>
- Victor B. (2011). *Explorable Explanations*. <https://worrydream.com/ExplorableExplanations/>
- Case N. *How I Make an Explorable Explanation*. <https://blog.ncase.me/how-i-make-an-explorable-explanation/> ; *Explorable Explanations: 4 More Design Patterns*. <https://blog.ncase.me/explorable-explanations-4-more-design-patterns/>
- *Communicating with Interactive Articles* (2020). Distill. <https://distill.pub/2020/communicating-with-interactive-articles/>
- Nielsen Norman Group. *Progressive Disclosure*. <https://www.nngroup.com/articles/progressive-disclosure/> ; *Minimize Cognitive Load to Maximize Usability*. <https://www.nngroup.com/articles/minimize-cognitive-load/> ; *(titre non relevé)* <https://www.nngroup.com/articles/dashboards-preattentive/> ; *Tooltip Guidelines*. <https://www.nngroup.com/articles/tooltip-guidelines/>
- Tufte E. R. *Sparkline theory and practice*. <https://www.edwardtufte.com/notebook/sparkline-theory-and-practice-edward-tufte/>
- Wube Software. *Friday Facts #318 – New Tooltips*. <https://factorio.com/blog/post/fff-318> ; *#397 – Factoriopedia*. <https://factorio.com/blog/post/fff-397>
- Oxygen Not Included wiki (wiki.gg), *Overlays*. <https://oxygennotincluded.wiki.gg/wiki/Overlays> ; Gamepressure, *Tabs, Overlays*. <https://www.gamepressure.com/oxygen-not-included/tabsoverlays/z59cfc> (liste d'overlays seulement ; non citée comme principe)
- Colossal Order. *Cities: Skylines Dev Diary 9 – Info Views* (extrait seulement). <https://beyondsims.com/2015/02/skylines-dev-diary-9-info-views/>
- W3C. WCAG 2.2 Understanding SC 1.4.3, 1.4.11, 1.4.1. <https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html> ; <https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html> ; <https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html>
- Valve. *Steam Deck Compatibility Review Process*. <https://partner.steamgames.com/doc/steamdeck/compat>
- Godot Engine documentation (stable) : <https://docs.godotengine.org/en/stable/tutorials/ui/gui_skinning.html> ; <https://docs.godotengine.org/en/stable/tutorials/rendering/multiple_resolutions.html> ; <https://docs.godotengine.org/en/stable/tutorials/ui/gui_containers.html> ; <https://docs.godotengine.org/en/stable/classes/class_control.html> (lue sur les 200 000 premiers caractères).
- University of Arizona, Laboratory of Tree-Ring Research. *Making Skeleton Plots*. <https://www.ltrr.arizona.edu/skeletonplot/plotting.htm>
- OpenAlea documentation (page d'accueil, contenu limité). <https://openalea.readthedocs.io/en/latest/> ; GroIMP wiki. <https://wiki.grogra.de/doku.php?id=start>
- PC Gamer / Mini Metro (annonce de conférence) : accroches seulement, voir B.

### Résumés de recherche seulement (RS), non utilisées comme principe
- Richardson J., *serif vs sans-serif* (SpringerBriefs, via recherche) ; recommandations « 33 vues » de Cities: Skylines II ; Dwarf Fortress (PC Gamer, Gamasutra), Tiny Glade, Terra Nil, Kerbal Space Program (fils de forum), Eco, Townscaper ; les « 13 erreurs » de Few (livre) ; *data-ink ratio* et *small multiples* de Tufte (*The Visual Display of Quantitative Information*) ; Sweller (1988-2019) ; CooRecorder / CDendro / dplR ; AMAPstudio ; Factorio alt-mode (FFF #208, #266) ; PhET « about/research » (chiffre de 200 entretiens) ; études York et UCL sur le HUD (citées dans un résumé).

### Inaccessibles (NA)
- Game UI Database (403). Sweller 2010 et Sweller et al. 2019 (Springer, redirection vers une authentification). Wiki Paradox des vues d'information (page d'erreur). HAL (AMAPstudio, page Anubis). PMC (captcha). Factorio « Tooltips » wiki (404). Fandom ONG (402). `edwardtufte.com/tufte/sparkline_theory` (404). Blog de Nicky Case « 4 ways to visualize data » (404). uxdesign.cc « Pacing the flow of information » (403). Site `www.grogra.de` (certificat) ; <https://amapstudio.cirad.fr/> non lu. La page de recherche PhET (404).
