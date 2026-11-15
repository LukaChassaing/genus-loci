# Genus Loci: Dendrologia

Prototype de *Genus Loci: Seed Master* (nom du jeu complet à venir).

Simulateur d'arbres éducatif : un arbre pousse selon ses gènes (traits), son milieu et la météo,
au-dessus et au-dessous du sol. Prototype en cours : le chêne pédonculé (*Quercus robur*).

## Structure

| Dossier | Rôle |
|---|---|
| `crates/dendro-core` | Cœur de simulation en Rust pur (génome, climat, croissance, cernes, maillages). Aucune dépendance à Godot. |
| `crates/dendro-godot` | Liaison Godot (gdext) : expose la classe `TreeSim`. |
| `godot/` | Projet Godot 4.7 : scène 3D, interface, vue des cernes. |
| `data/species/` | Fiches de traits par espèce. |

## Lancer

Prérequis : Rust (stable) et Godot 4.7.

```bash
cargo build -p dendro-godot
```

```bash
godot --path godot
```

## Tester le cœur seul

```bash
cargo test -p dendro-core
```

```bash
cargo run -p dendro-core --release --example grow -- --years 200
```
