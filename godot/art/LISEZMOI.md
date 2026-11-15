# Peintures optionnelles

Le jeu charge ces images au lancement (sans import Godot) si elles existent, sinon il peint
lui-même un fond procédural. Formats acceptés : `.png`, `.jpg`, `.jpeg`, `.webp`.

| Nom (sans extension) | Usage |
|---|---|
| `title_backdrop` | Écran titre, 16:9. Sujet à gauche (~65 %), tiers droit plus sombre et calme (le menu s'y pose). Recadrage « cover », jamais étiré. |
| `setup_backdrop` | Écran de préparation de l'Arboretum, 16:9. |
| `pause_backdrop` | Menu pause, 16:9, facultatif. Absent : vue du jeu floutée et assombrie. |

Les planches botaniques (écran « Planches ») sont entièrement dessinées par le code : aucune image n'est nécessaire.
