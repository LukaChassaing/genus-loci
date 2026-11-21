# Construire, lancer, exporter

Date : 2026-11-16. Statut : **Linux vérifié en local ; Windows non vérifié** (ni la CI ni un export Windows n'ont tourné au moment de l'écriture ; les passages concernés sont marqués *non vérifié*).

## Prérequis

- Rust stable (`rustup`), avec la cible voulue.
- Godot **4.7.2** (éditeur standard, pas la version .NET).
- Linux : un compilateur C et `pkg-config` suffisent pour le cœur et l'extension. Windows : Visual Studio Build Tools (charge de travail « Développement Desktop en C++ ») pour la cible `x86_64-pc-windows-msvc`.

## Linux

```bash
cargo build -p dendro-godot              # bibliothèque « debug » (celle que charge l'éditeur)
cargo build -p dendro-godot --release    # bibliothèque « release » (export)
godot --path godot                       # lancer le jeu
godot --path godot --editor              # ouvrir l'éditeur
```

Tests : `cargo test -p dendro-core --release` ; performance (ignorés par défaut) : `cargo test -p dendro-core --release -- --ignored` ; lint : `cargo clippy -p dendro-core -p dendro-godot --tests -- -D warnings`.

Auto-test de l'interface, sans fenêtre (code de sortie 1 s'il échoue, message `ALL PASSED` sinon) :

```bash
godot --headless --path godot --import          # pour créer .godot/, et après chaque pull qui ajoute des scripts à class_name (sinon « Could not resolve external class member »)
DENDRO_SELFTEST=1 godot --headless --path godot
```

`--headless` utilise des pilotes d'affichage et de rendu factices : **aucun GPU ni Vulkan n'est nécessaire**, ce qui permet de l'exécuter sur un runner GitHub. Limite : il ne prouve rien sur le rendu réel (shaders, MSAA, plein écran) ; cela reste à vérifier à la main sur la machine cible.

## Windows

Natif (recommandé) :

```powershell
rustup target add x86_64-pc-windows-msvc   # déjà la cible par défaut sous Windows
cargo build -p dendro-godot                # target\debug\dendro_godot.dll
cargo build -p dendro-godot --release      # target\release\dendro_godot.dll
godot --path godot
```

`godot/dendrologia.gdextension` déclare `windows.debug.x86_64` et `windows.release.x86_64` avec la même convention que Linux (`res://../target/{debug,release}/dendro_godot.dll` ; pas de préfixe `lib` sous Windows). *Non vérifié sur Windows.*

Compilation croisée depuis Linux : `rustup target add x86_64-pc-windows-gnu` puis `cargo build -p dendro-godot --target x86_64-pc-windows-gnu`. Vérifié ici : `cargo check` passe pour `x86_64-pc-windows-gnu` **et** `x86_64-pc-windows-msvc` (typage seulement). L'édition de liens échoue localement car `x86_64-w64-mingw32-gcc` n'est pas installé (`sudo apt install mingw-w64`, non fait). Attention : une DLL produite avec la cible `-gnu` dépend de runtimes MinGW et n'est pas celle qu'on distribuera ; la référence est la cible MSVC construite par la CI. Pour tester une DLL croisée, le chemin Godot attend `target/<profil>/…`, il faudrait la copier là ou ajouter `target/x86_64-pc-windows-gnu/…` au `.gdextension`.

## Intégration continue

`.github/workflows/ci.yml` (push et PR sur `main`) :

| Job | Contenu |
|---|---|
| `no-species-code` | échoue si le cœur contient `species ==`, un `match` sur l'espèce ou des noms d'espèce hors `genome.rs` et hors commentaires (règle « zéro branche par espèce »). |
| `test` (Linux, Windows) | `cargo test -p dendro-core --release` et clippy `-D warnings`. |
| `perf` (Linux, Windows) | tests `#[ignore]` en release. **Non bloquant** (`continue-on-error`) : le temps dépend du runner partagé. Le critère « < 2 s » se valide sur la machine cible. |
| `godot` (Linux, Windows) | build debug et release de l'extension, téléchargement de Godot 4.7.2 (assets officiels `Godot_v4.7.2-stable_linux.x86_64.zip` et `Godot_v4.7.2-stable_win64.exe.zip` sur `godotengine/godot-builds`, SHA-256 vérifié), `--import`, auto-test `DENDRO_SELFTEST=1` (le job échoue sur code de sortie non nul ou si `ALL PASSED` est absent), puis dépôt des bibliothèques release en artefacts (`dendro-godot-Linux`, `dendro-godot-Windows`). |
| `determinism` + `determinism-compare` | exécute `examples/state_hash.rs` sur les deux OS et échoue si les sorties diffèrent. Aucun hachage de référence n'est codé en dur. |

Si les SHA-256 de Godot changent (autre version), les mettre à jour dans le workflow (`SHA512-SUMS.txt` est publié avec la release).

Limites honnêtes : sous Windows le job utilise `Godot_*_win64_console.exe` (présent dans le zip officiel, *non vérifié*) pour que la sortie standard soit captée ; l'auto-test actuel échoue en local pour des raisons sans lien avec la CI (3 contrôles du menu titre et du cadrage, modifications en cours) ; `performance_200_years` a aussi échoué en local pendant que d'autres travaux compilaient.

## Exporter un jeu

*Non vérifié : aucun préréglage d'export n'existe encore (`godot/export_presets.cfg` absent).*

1. Installer les modèles d'export 4.7.2 (`Godot_v4.7.2-stable_export_templates.tpz`, menu Éditeur > Gérer les modèles d'export).
2. Projet > Exporter : créer « Linux » et « Windows Desktop », décocher « Embed PCK » au besoin.
3. **Point d'attention** : les entrées du `.gdextension` pointent hors du projet (`res://../target/...`). L'export Godot n'embarque que `res://` ; la bibliothèque ne serait pas livrée. Solution proposée : copier les bibliothèques release dans `godot/bin/` (déjà dans `.gitignore`) avant l'export et ajouter dans le `.gdextension` des entrées `res://bin/...` (ou un second fichier de chemins d'export), puis vérifier que l'exécutable lancé hors de l'éditeur charge bien `TreeSim`.
4. Ligne de commande : `godot --headless --path godot --export-release "Windows Desktop" ../build/dendrologia.exe` (idem `"Linux"`).

## Déterminisme

Objectif (roadmap M0) : même graine, même génome, mêmes appels, même état sur Linux et Windows. `examples/state_hash.rs` fait pousser le chêne et le merisier 100 ans (graine 42) et imprime, par arbre, `hash` (FNV-1a sur les bits des statistiques, cernes et quatre maillages) et `state` (`Simulation::state_hash()` de `save.rs`, sur l'état interne). En local : deux exécutions successives donnent des sorties identiques. Le job CI `determinism-compare` compare les sorties Linux et Windows.

### Constat de la première CI

Les deux plateformes divergeaient (chêne : Linux `height=23.9131 dbh=77.6333`, Windows `height=24.1681 dbh=75.8739`), et le test `limb_to_stem_ratios_and_elastic_similarity` échouait seulement sous Windows. Cause : les fonctions transcendantes de `std` (`sin`, `cos`, `exp`, `ln`, `powf`...) appellent la libm de la plateforme (glibc, UCRT), précise à ~1 ulp mais pas identique bit à bit ; la croissance sur 36 500 jours amplifie la moindre différence.

### Ce qui a été fait

- Crate `libm` (Rust pur, `libm = "0.2"`) et module `crates/dendro-core/src/fmath.rs` : trait `DMath` implémenté pour `f32` et `f64`, méthodes `dsin`, `dcos`, `dtan`, `dacos`, `datan2`, `dexp`, `dln`, `dpowf`, `dsin_cos`, `dpowi`. `dpowi` est une exponentiation par carrés dans un ordre fixe (`std::powi` n'est pas garanti identique d'un niveau d'optimisation ou d'une plateforme à l'autre).
- Les 66 appels de `dendro-core/src` ont été remplacés (`climate` 10, `growth` 14, `roots` 9, `mesh` 6, `flux` 4, `pruning` 3, `rng` 2 (en f64), `phenology` 2, `sim` 2, `genome`, `math`, `soil` 1 chacun).
- Restent des méthodes `std`, exactes selon IEEE 754 : `+ - * /`, `sqrt`, `abs`, `min`, `max`, `clamp`, `floor`, `ceil`, `round`, `rem_euclid`, `signum`, `to_radians`/`to_degrees` (multiplications). Aucun `mul_add`, `cbrt`, `hypot`, `tanh`... dans le cœur. Pas de FMA implicite : Rust ne contracte pas `a*b+c`, et il n'y a ni `-C target-cpu=native` ni fast-math.
- Garde-fou : `crates/dendro-core/tests/determinism.rs` échoue si `.sin(`, `.cos(`, `.exp(`, `.ln(`, `.powf(`, `.powi(`, `.atan2(`, `.tanh(`, `.mul_add(`... (ou `f32::sin(`...) apparaît dans `crates/dendro-core/src` hors `fmath.rs` (`.log(` est exclu : `SaveFile::log()` est un accesseur).
- Autres sources examinées, sans effet : `HashMap`/`HashSet` de `pruning.rs` (recherche id -> index, jamais itérés) ; tris (`sort_by` avec `total_cmp` et départage par indice, `sort_unstable` sur clés totalement ordonnées) ; pas de thread, d'horloge ni de `env` dans le cœur ; les conversions `as u32`/`as usize` portent sur des indices et des quantités bornées (les `as` flottant -> entier saturent de façon définie, NaN -> 0) ; aucune dépendance à la largeur de `usize` trouvée dans le calcul (les compteurs sont des `u32`). Le binding Godot ne renvoie à la simulation que des durées en jours (la fraction de jour par image est reportée dans le binding, et `advance_days(n)` est identique à n appels d'un jour) et des actions de taille (position de coupe, journalisées donc rejouées telles quelles) : aucune valeur calculée côté GDScript n'influence la croissance.
- Les trajectoires ont légèrement changé (autre libm) : chêne de 100 ans 24,38 m / 76,1 cm, merisier 23,27 m / 76,1 cm. Quatre tests calibrés à la limite ont été rendus robustes par une médiane sur plusieurs graines (bornes inchangées) : voir les commentaires de `limb_to_stem_ratios_and_elastic_similarity`, `soil_shapes_the_tree`, `oak_architecture` et `cherry_declines_while_oak_grows`. Aucun test ne compare un hachage d'état figé.
- Coût : environ +5 % sur les tests de performance (chêne 200 ans 3,9 -> 4,1 s, merisier 130 ans 0,49 -> 0,52 s, `state_hash` inchangé à 1,07 s), pas d'optimisation nécessaire.

### Ce qui reste incertain

La preuve est le job `determinism-compare` : il n'a pas pu être exécuté sous Windows ici (seul `cargo check --target x86_64-pc-windows-msvc` a été fait). Restent à surveiller : `f32::sqrt` est exact partout, mais un futur code utilisant `f32::sin` etc. directement réintroduirait la divergence (le garde-fou le détecte seulement dans `dendro-core/src`) ; les sauvegardes embarquent un hachage d'état, qui détectera aussi toute dérive entre plateformes ou versions.

### Règle pour les contributeurs

Dans `dendro-core/src`, jamais de `.sin()`, `.cos()`, `.exp()`, `.ln()`, `.powf()`, `.powi()`... bruts : importer `crate::fmath::DMath` et utiliser `.dsin()`, `.dcos()`, `.dexp()`, `.dln()`, `.dpowf()`, `.dpowi()`. N'ajouter une nouvelle fonction qu'en l'enveloppant dans `fmath.rs` (libm) ; ne jamais itérer sur un `HashMap`/`HashSet`, ne pas utiliser de FMA/fast-math ni `target-cpu=native`, ne jamais comparer un test à un hachage d'état figé.
