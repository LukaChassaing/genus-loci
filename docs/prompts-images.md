# Prompts d'images (Gemini, ChatGPT Images…)

Les fonds peints du jeu sont générés hors du projet puis déposés dans `godot/art/`.
Le jeu charge un fichier s'il existe et retombe sur un rendu par code sinon.
Formats acceptés : `.png`, `.jpg`, `.webp`. Le nom du fichier (sans extension) doit être exactement celui du tableau.

Les prompts sont en anglais : les modèles d'image y répondent mieux.
**Jamais de texte dans l'image** : les titres et menus sont posés par le jeu.

Les planches botaniques ne sont pas concernées : elles sont dessinées par le code à partir des données de l'espèce.

## Bloc de style commun

À coller au début de chaque prompt de **peinture** :

> Loose oil painting concept art, visible broad brush strokes and palette-knife marks, a magnificent stylised sky with large soft cumulus clouds, canvas texture, bright and gentle spring palette of soft sky blue, cream white, fresh leaf green and pale gold with small blossom-pink accents, clear morning light, calm and cheerful cosy mood, soft edges, painterly not photographic, no text, no letters, no watermark, no signature, no frame.

## Fonds peints (16:9, au moins 2560×1440)

| Fichier | Écran | Prompt (après le bloc peinture) |
|---|---|---|
| `title_backdrop` | Écran-titre | A lone old pedunculate oak in fresh spring leaf on a gentle green rise in a flowering meadow, seen from far, broad rounded crown placed in the left half of the image, large luminous stylised clouds in a soft blue morning sky, warm sunlight on the grass, a few birds. The right third of the image is calmer and almost empty (pale sky and soft meadow) to leave room for a menu. Wide 16:9 composition. |
| `setup_backdrop` | Préparation de l'arboretum | A sunny arboretum in spring, a winding gravel path between widely spaced trees of different species and ages, a wild cherry in full white blossom on the left, a young staked sapling in the foreground, dappled light on fresh grass with daisies. Lighter and simpler on the right third. Wide 16:9 composition. |
| `pause_backdrop` | Pause (facultatif) | A naturalist's wooden desk by an open window on a bright spring morning: pressed leaves, a slice of trunk showing growth rings, a hand lens, acorns and cherry stones, a sprig of cherry blossom in a glass, soft daylight. Wide 16:9 composition. |

Pistes pour plus tard (mode narratif, une image par lieu ou par commande) : la pépinière du commanditaire, une lande au soleil, une vallée inondable, un verger en fleurs, un versant calcaire sec, une cour de ville.

## Conseils

- Générer 3 ou 4 variantes et garder celle où l'arbre est botaniquement juste (port large et branches sinueuses du chêne).
- Si le modèle ajoute du texte malgré tout, relancer avec « absolutely no writing of any kind ».
- Garder la même session ou la même image de référence pour tous les fonds, afin que la touche reste homogène.
- Vérifier les conditions d'utilisation du service pour un jeu diffusé sur Steam avant la première publication.
