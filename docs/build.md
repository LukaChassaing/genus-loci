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
godot --headless --path godot --import          # une fois, pour créer .godot/
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

Objectif (roadmap M0) : même graine, même génome, mêmes appels, même état sur Linux et Windows. `examples/state_hash.rs` fait pousser le chêne et le merisier 100 ans (graine 42) et imprime, par arbre, `hash` (FNV-1a sur les bits des statistiques, cernes et quatre maillages) et `state` (`Simulation::state_hash()` de `save.rs`, sur l'état interne). En local : deux exécutions successives donnent des sorties identiques.

### Sources possibles de divergence dans `dendro-core/src`

| Source | Statut |
|---|---|
| **Fonctions transcendantes de la libm de la plateforme** : `std` f32/f64 `sin`, `cos`, `tan`, `acos`, `atan2`, `exp`, `ln`, `powf` appellent glibc sous Linux et l'UCRT/compiler-rt sous Windows ; les deux sont précises à ~1 ulp mais **pas identiques bit à bit**. | **Risque réel, probablement fatal sur 100 ans.** Comptage (hors tests non triés) : `climate.rs` 8 `sin`, 8 `cos`, 2 `tan`, 2 `acos` ; `growth.rs` 6 `sin`, 6 `cos`, 4 `powf`, 1 `tan`, 1 `exp`, 1 `atan2`, 1 `acos` ; `roots.rs` 3 `powi`, 2 `atan2`, 1 chacun `sin`/`cos`/`powf`/`exp` ; `rng.rs` 2 `ln`, 1 `cos` (en f64) ; `mesh.rs` 3 `sin`/`cos`, 1 `atan2` ; `flux.rs` 2 `powf`, 1 `sin`/`cos`/`exp` ; `pruning.rs` 1 `sin`, 1 `powf` ; `phenology.rs` 1 `powf`, 1 `powi` ; `sim.rs` 1 `powf`, 1 `exp` ; `soil.rs` 1 `exp` ; `genome.rs` 1 `exp`. |
| `sqrt`, `+ - * /`, `abs`, `min`, `max`, `floor` | Exacts selon IEEE 754, sans risque. |
| `powi` | Appelle `__powisf2` (multiplications en chaîne) ; déterministe tant que le compilateur ne le replie pas différemment ; à surveiller, pas à remplacer en priorité. |
| FMA / `fast-math` | Rust ne contracte pas `a*b+c` et n'a pas de fast-math en stable ; pas de `-C target-cpu=native` (aucun `.cargo/config`). Rien à faire, à ne pas introduire. |
| `HashMap` / `HashSet` (`pruning.rs`, 8 occurrences) | Utilisés uniquement en recherche (`id -> index`), aucune itération sur la table (vérifié par recherche). L'ordre aléatoire ne joue donc pas. À maintenir : ne jamais itérer dessus (préférer `BTreeMap`/`Vec`). |
| Threads | Aucun dans le cœur (`rayon`/`std::thread` absents). |
| Tri | 7 `sort*`; `sort_unstable` de Rust est déterministe pour une entrée donnée. Un tri sur des `f32` avec égalités doit rester total (`total_cmp`). |
| Sérialisation `Debug` des `f32` (hachage de `save.rs`) | Forme la plus courte aller-retour, identique sur toutes les plateformes. |

**Prédiction** : les hachages Linux et Windows divergent probablement dès que la CI tournera (au premier ulp différent d'un `sin`/`powf`, la divergence est amplifiée par la croissance sur 36 500 jours). Si le job `determinism-compare` est rouge, c'est ce point, pas un bogue de la CI.

### Correctif proposé (non appliqué : `sim.rs`, `growth.rs`, `mesh.rs`, `rings.rs` sont en cours de modification par un autre chantier)

1. `crates/dendro-core/Cargo.toml` : ajouter `libm = "0.2"` (Rust pur, mêmes résultats partout).
2. Créer un module `crates/dendro-core/src/fmath.rs` (déclaré `mod fmath;` dans `lib.rs`) :

```rust
//! Fonctions transcendantes déterministes (libm pure Rust), pour un résultat identique sur tous les OS.
#[inline] pub fn sin(x: f32) -> f32 { libm::sinf(x) }
#[inline] pub fn cos(x: f32) -> f32 { libm::cosf(x) }
#[inline] pub fn tan(x: f32) -> f32 { libm::tanf(x) }
#[inline] pub fn acos(x: f32) -> f32 { libm::acosf(x) }
#[inline] pub fn atan2(y: f32, x: f32) -> f32 { libm::atan2f(y, x) }
#[inline] pub fn exp(x: f32) -> f32 { libm::expf(x) }
#[inline] pub fn powf(x: f32, y: f32) -> f32 { libm::powf(x, y) }
#[inline] pub fn ln64(x: f64) -> f64 { libm::log(x) }
#[inline] pub fn cos64(x: f64) -> f64 { libm::cos(x) }
```

3. Remplacer chaque `x.sin()` par `fmath::sin(x)`, etc., dans les fichiers listés ci-dessus (une centaine de sites, mécanique ; `rng.rs` : `ln` et `cos` en f64). Le plus sûr : remplacement guidé par le compilateur, puis `cargo test` ; la calibration bouge au dernier bit, il faudra relancer les tests de réalisme.
4. Garde-fou CI : étendre `no-species-code` ou ajouter un `grep -rnE '\.(sin|cos|tan|acos|atan2|exp|ln|powf)\(' crates/dendro-core/src` hors `fmath.rs` qui échoue.
5. En complément (ou en secours), les sauvegardes embarquent déjà un hachage d'état : un instantané d'état plutôt qu'un rejeu éviterait la dépendance au déterminisme pour les parties sauvegardées.
