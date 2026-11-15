# Analyse : BRUTAL (Ahwoo / RocketWerkz) pour Dendrologia

Date de l'analyse : 2026-11-15. Convention : **[V]** = vérifié dans une source citée en fin de document ; **[I]** = inférence ou estimation de ma part.

## 1. Verdict

**(b) Garder Godot maintenant, laisser la porte ouverte.** BRUTAL n'est pas un moteur : c'est une boîte à outils C# (interop natif, Vulkan, ImGui, etc.) sans éditeur, sans scène, sans moteur de rendu ni UI de jeu. Passer dessus reviendrait à réécrire tout ce que Godot fournit aujourd'hui, avec une licence probablement incompatible avec un code GPLv3.

Les 5 raisons principales :
1. Ce n'est pas un moteur : « no editor, scene graph, or renderer », et les mainteneurs disent ne pas prévoir d'en ajouter [V].
2. Licence : MIT + obligations (logo avant le jeu, NOTICE.txt, crédits), non approuvée OSI. Conflit probable avec la GPLv3 [I], alors que Godot est MIT sans condition.
3. Maturité : dépôt créé le 2026-10-14, deux releases, 5 contributeurs, équipe « petite, réponses lentes » [V]. Aucun build depuis les sources (pas de .sln ni scripts) [V].
4. Pile de langages : il faudrait ajouter .NET 10 + C# entre Rust et le GPU ; vous perdez gdext, qui marche déjà [I].
5. Les gains de BRUTAL (contrôle Vulkan, instancing/LOD maison, compute) ne sont pas votre goulot sur un HD 7950 ; le coût (UI 2D riche, localisation, export, Steam) est énorme pour un développeur solo [I].

## 2. Ce qu'est BRUTAL (vérifié)

- **Nature** [V] : « free, open source C# framework for making games or building your own game engine » (README). Développé par Ahwoo Limited et l'équipe RocketWerkz Enterprise (fondateur : Dean Hall). Le cœur est une couche d'interop donnant à C# un accès direct aux API natives. Pas de choix d'architecture imposé ; tout paquet NuGet est utilisable.
- **Langage / runtime** [V] : C#, .NET 10 SDK obligatoire. Distribué en paquets NuGet `Ahwoo.Brutal.*`, un dossier = un paquet.
- **Graphique** [V] : Vulkan 1.3 (GPU capable de Vulkan 1.3 requis « si rendu »), liaisons OpenGL aussi présentes.
- **Contenu** [V] : Core, Interop, Memory, Collections, Strings, Maths/Numerics, Concurrency (job system), Logging, Localization ; Vulkan + Abstractions + VMA, ShaderC et Slang (SPIR-V), Render.Mesh, textures (Stb, KTX, Gli), glTF, MeshOptimizer, FreeType ; GLFW et SDL (Image/Mixer/Ttf) ; Dear ImGui (branche docking) + ImPlot ; audio FMOD (licence FMOD à fournir par l'utilisateur) et OpenAL ; RakNet ; physique Jolt (3D) et Box2D (2D) ; RenderDoc ; dialogues de fichiers natifs ; Monitor (watchdog).
- **Plateformes** [V] : Windows et Linux (README, 80.lv). Rien sur Wayland vs X11, ni sur le Steam Deck, dans ce que j'ai lu.
- **Documentation** [V] : README, `samples/Hello/*` (un exemple par paquet) et `samples/Demo/*` (Vulkan triangle, visionneuse glTF, quad texturé KTX2, ImGui, RenderDoc, Jolt, Box2D). `docs/` ne contient que `branding` (vide à la lecture). Forum ahwoo.com/forums/brutal et Discord. Pas de manuel ni d'API reference lus.
- **Activité** [V] (API GitHub) : 2 492 étoiles, 166 forks, 22 issues ouvertes, créé le 2026-10-14, dernier push le 2026-11-12. Releases `v2026.10.0` (« initial public release ») et `v2026.10.1` (mise à jour Slang/Vulkan/Gltf/ImGui, natifs déplacés en sous-paquets) ; les dates `published_at` renvoyées sont incohérentes (10-23 pour .1, 10-28 pour .0), je les rapporte telles quelles. Contributeurs listés par l'API : 5 (4, 4, 2, 1, 1 contributions), probablement pas toute l'équipe interne (un historique publié a pu être aplati) [I]. Issues récentes : bugs de détail (ImGui, Slang, Ktx, glTF, Vulkan.Abstractions : acquire/present sur swapchain « out of date »). Un seul mentionne Linux/Wayland, et sans l'avoir observé (testé sous Windows 11) [V].
- **Usage par KSA** [V] : le README liste Kitten Space Agency comme « Direct on BRUTAL » (sans moteur intermédiaire), avec Torpedia, Art of the Rail, Gruntz (RocketWerkz), Afterskies (Floating Origin) ; RimWorld utilise le moteur Waylith bâti sur BRUTAL. Ludeon annonce 2 à 4 fois la performance de la version Unity, mesures propres à Ludeon, non vérifiées par moi. GamingOnLinux (déc. 2025) cite BRUTAL pour KSA et note que le jeu fonctionnait sous Linux via Proton Experimental ; je n'ai pas pu établir si KSA a aujourd'hui un build Linux natif.
- **Philosophie** [V] : Hall dit préférer que d'autres écrivent leurs propres cadres plus simples plutôt que de faire de BRUTAL un standard ; Unity/Unreal restent adaptés à de nombreux projets (80.lv, Game Developer). Ludeon et RocketWerkz le décrivent comme un cadre pour des jeux très spécifiques et lourds en simulation.

## 3. Licence (vérifié, puis inférence)

[V] « BRUTAL License v1.0 », base MIT, New Zealand law (6.0). Tout *Product* distribué doit : (2.1) mettre le copyright dans les crédits ; (2.2) afficher le logo Brutal **avant le début du jeu** (dispense : permission écrite d'Ahwoo, ou produit sans affichage ; le README ajoute une exemption pour les projets non lucratifs ou si cela nuit à l'objet de l'application) ; (2.3) livrer `NOTICE.txt`. Pas de copyleft, pas de royalties. Licence commerciale séparée possible. README : « not yet OSI approved ». GitHub : licence « Other / NOASSERTION ». Presse (GamingOnLinux) : « source-available » plutôt qu'open source.

[I] GPLv3 : la GPLv3 interdit les « restrictions supplémentaires » hors liste du §7 ; l'obligation d'un logo plein écran avant le jeu n'y figure pas clairement, et la clause « non lucratif » est floue pour un jeu gratuit sur Steam. Risque réel d'incompatibilité, que je ne peux pas trancher (pas juriste). Il faudrait une dispense écrite d'Ahwoo (2.2) ou un avis juridique. Distribution Steam : aucune clause contraire lue ; l'obligation de logo reste à afficher.
À noter : Godot (MIT) n'impose rien de tel.

## 4. Comparaison Godot vs BRUTAL pour Dendrologia

| Besoin | Godot 4.7 (actuel) | BRUTAL |
|---|---|---|
| Nature | Moteur + éditeur complets [V connu] | Bibliothèques C#, ni éditeur ni scène ni renderer [V] |
| Interop Rust | gdext déjà en place, fonctionne | C# + P/Invoke/Interop vers un cdylib Rust (C ABI) [I] ; Rust → Vulkan direct impossible via BRUTAL |
| Upload de gros maillages dynamiques | `ArrayMesh`/`RenderingServer` (copie CPU→GPU gérée) | Maison : buffers VMA, staging pool (présents dans Vulkan.Abstractions) [V], à vous de gérer sync/frames en vol [I] |
| Thread / job | Threads Rust côté cœur ; Godot gère le sien | Job system Brutal.Concurrency [V] ; threads Rust inchangés |
| Caméra, scène, matériaux, éclairage, ombres, ciel | Fournis | À écrire (PBR/ombres/ciel/foliage alpha-test, MSAA/FXAA) : plusieurs mois [I] |
| UI 2D (HUD, perks, lab, tooltips, polices, layout) | Contrôles, thèmes, tooltips, Control nodes | Seulement ImGui (outil de debug, pas de skin de jeu) ; pas de toolkit UI de jeu [V] ; à écrire ou à bricoler |
| Localisation FR/EN | Système de traduction intégré | Brutal.Localization existe [V], contenu non évalué |
| Entrées / remapping | InputMap | GLFW/SDL bruts [V] ; mapping à écrire |
| Audio | Intégré | FMOD (licence à part) ou OpenAL/SDL_mixer [V] |
| Export Linux/Windows, Steam | Export templates ; GodotSteam | `dotnet publish` possible [I] ; Steam à brancher soi-même |
| Outillage | Éditeur, profileur, remote debug | RenderDoc intégré, ImGui, Monitor [V] ; pas d'éditeur |
| Compute shaders, instancing/LOD sur mesure | Possible (RenderingDevice, MultiMesh, visibility ranges) mais cadré | Contrôle total Vulkan 1.3 [V] |
| Linux / Wayland | Mature, Wayland natif | Windows et Linux annoncés [V] ; maturité Linux et Wayland non démontrées dans mes sources |
| Licence | MIT sans condition [V connu] | MIT + logo/NOTICE/crédits ; GPLv3 douteux [I] |
| Maturité | Années, large communauté | 1 mois publiquement, API qui bouge (« natives offloaded to sub-packages » dès la 2e release) [V] |
| Build | Standard | Pas de sources buildables (NuGet seulement) [V] |

## 5. Interop Rust, ce que cela impliquerait [I]

- Architecture : `dendro-core` compilé en `cdylib` avec une façade `extern "C"` (buffers `#[repr(C)]` : sommets, indices, état de l'arbre), appelée depuis C# par les `LibraryImport`/P/Invoke de BRUTAL Interop ; pas d'allocations par appel, tampons partagés.
- Maillage : Rust remplit un tampon ; C# le copie dans un staging buffer VMA puis `vkCmdCopyBuffer`, double ou triple buffering. 260k triangles reconstruits plusieurs fois par seconde = quelques dizaines de Mo/s, trivial pour la bande passante PCIe ; la difficulté est la synchronisation, pas le débit.
- Alternative sans C# : appeler Vulkan depuis Rust (ash/wgpu) ; BRUTAL n'apporterait alors rien.

## 6. Ce qu'il faudrait construire (estimation solo) [I]

Fenêtre/boucle/swapchain/resize : 1 à 2 semaines. Renderer forward PBR avec ombres cascadées, ciel, alpha-test, MSAA : 2 à 3 mois. Caméra et contrôles : 1 semaine. Framework UI de jeu (layout, polices, thèmes, tooltips, sliders, localisation, accessibilité, gamepad) : 2 à 4 mois, c'est le plus gros risque vu le volume prévu (arbre de perks, planches naturalistes, labo). Audio, sauvegarde, export, Steam : 2 à 4 semaines. Total ordre de grandeur : 5 à 9 mois de travail avant de retrouver l'équivalent actuel, hors forêts/LOD/vent. Cela vampirise un projet dont la valeur est la simulation et le contenu éducatif.

## 7. Ce que BRUTAL ferait mieux, et si cela compte ici

- Contrôle fin : pipelines d'instancing de feuillage, LOD, culling GPU, compute, empreinte mémoire réduite [I]. Plausible pour des forêts massives.
- Perf mesurée : 2 à 4 fois pour RimWorld (Ludeon), mais face à Unity/C# et pour un jeu 2D à beaucoup d'entités [V, déclaratif] ; cela ne se transpose pas à Godot/Rust.
- Sur un **HD 7950** (GCN 1, RADV) [I] : BRUTAL exige un GPU Vulkan 1.3 [V] ; votre pile RADV l'annonce, donc le seuil est atteint, mais je n'ai pas pu vérifier quelles extensions/features (dynamic rendering, synchronization2, descriptor indexing, BDA) les abstractions BRUTAL exigent ni leur support sur Southern Islands. Pas de RT ni mesh shaders : sans effet, rien dans vos besoins ne les demande. Le goulot probable est le fill-rate et la bande passante d'un GPU de 2012, pas le surcoût CPU du moteur ; Godot Forward+ s'y règle (MultiMesh, visibility ranges, Mobile/Compatibility si besoin) sans quitter l'éditeur.

## 8. Risques

- **Bus factor** : projet de quelques personnes, centré sur RocketWerkz/Ahwoo ; contributions externes acceptées mais lentes [V]. Le projet dépend de la santé d'un studio.
- **Stabilité d'API** : versionnage par mois (2026.10.x), refontes de paquets déjà en deux releases [V].
- **Linux** : annoncé [V], mais KSA y tournait via Proton fin 2025 [V] ; support natif et Wayland non établis dans mes sources.
- **Documentation** : exemples par paquet, pas de manuel lu [V].
- **Juridique** : voir §3.
- **Écosystème** : peu de tutoriels, communauté jeune (forum + Discord).

## 9. Garder la porte ouverte (recommandation, option b)

1. Conserver le cœur strictement indépendant du moteur (déjà : `MeshData` pur) ; figer une **API C ABI** minimale et testée (`dendro_core_*`, tampons `repr(C)`), utilisable depuis gdext et depuis tout autre front-end.
2. Ne pas laisser de logique de jeu/simulation dans les scripts GDScript ; garder GDScript pour la présentation et l'UI.
3. Données et contenu (espèces `.ron`, textes de localisation en fichiers neutres type PO/CSV/JSON, planches) hors du projet Godot.
4. Isoler un module « renderer » côté Godot (une classe qui consomme `MeshData`) pour qu'un second consommateur reste simple.
5. Poser une réévaluation conditionnelle : revoir BRUTAL dans 12 mois si (a) licence clarifiée/dispense GPL, (b) sources buildables et API stable, (c) mesures de Godot montrant un mur de performance sur forêts malgré MultiMesh/LOD. Éventuel prototype jetable d'un après-midi : `DemoGltfViewer` + un maillage exporté du cœur, pour mesurer l'upload sur le HD 7950 (ce serait la seule vraie donnée utile).

## 10. À emprunter quel que soit le moteur [I, d'après les sources]

- **ImGui/ImPlot pour l'outillage interne** : console intégrée, widgets liés aux enums, graphes en direct (HelloImGuiAbstractions/Extensions). Côté Rust : `egui` ou `imgui-rs`, ou panneau debug Godot, pour régler la simulation sans UI de jeu.
- **RenderDoc** (capture sur F12) comme réflexe de profilage GPU, pour Godot aussi.
- **Allocation zéro, buffers non gérés, job system** : mêmes principes que votre cœur Rust.
- **Staging pool + double buffering** pour l'upload de maillages dynamiques, si vous passez par `RenderingServer` avec des tampons pré-alloués.
- **Mod-friendliness** (revendiquée pour KSA par GamingOnLinux) : données en RON, API stable, partage d'arbres par code.
- **Monde à grande échelle (KSA)** : je n'ai lu aucun devlog technique ; je ne peux pas citer leur approche (coordonnées flottantes, etc.). Non vérifié.

## 11. Ce que je n'ai pas pu lire / vérifier

- `https://ahwoo.com/brutal` : le contenu renvoyé est vide (page probablement rendue côté client) ; mes informations viennent du README GitHub, de l'API GitHub et de la presse.
- Texte intégral de `CONTRIBUTING.md`, `NOTICE.txt`, code source, forum, Discord, NuGet, issues au-delà de #33 (lecture tronquée).
- Support Linux/Wayland/Steam Deck réel de BRUTAL ; build Linux natif de KSA ; exigences Vulkan précises (extensions) ; devlogs techniques KSA.
- Avis juridique sur GPLv3 + licence BRUTAL.

## Sources

- https://ahwoo.com/brutal (illisible : vide)
- https://github.com/ahwootech/BRUTAL (README via https://raw.githubusercontent.com/ahwootech/BRUTAL/main/README.md)
- https://raw.githubusercontent.com/ahwootech/BRUTAL/main/LICENSE.txt
- https://raw.githubusercontent.com/ahwootech/BRUTAL/main/samples/README.md
- https://api.github.com/repos/ahwootech/BRUTAL (+ /contents, /contents/docs, /contents/samples, /releases, /contributors, /issues)
- https://www.gamedeveloper.com/programming/rocketwerkz-c-framework-brutal-is-now-available-for-all-developers
- https://80.lv/articles/dayz-creator-presents-brutal-a-free-open-source-development-framework
- https://www.gamingonlinux.com/2026/10/brutal-the-framework-behind-rimworlds-waylith-engine-releases-its-source-code/
- https://www.gamingonlinux.com/2025/12/kitten-space-agency-is-the-spiritual-successor-to-kerbal-space-program
