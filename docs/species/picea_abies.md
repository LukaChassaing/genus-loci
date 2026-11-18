# Picea abies (épicéa commun) : fiche de traits pour le génome

Rédigée le 2026-11-18. Entrées chiffrées : `data/validation/picea_abies.ron` (identifiants cités entre crochets).
Statut : **mesuré** (valeur publiée dans une source primaire ou de synthèse lue), **estimé** (déduit ou connaissance générale non vérifiée ici), **calibrer** (à ajuster par le moteur). Les sources Wikipédia sont signalées comme telles.

## Port et architecture

- Conifère persistant, monoïque, tige monopodiale très excurrente, branches en **verticilles** (un par an sur le tronc, plus des branches intercalaires), cime conique à colonnaire, étages supérieurs ascendants, inférieurs retombants (Atlas : « whorled, short and stout branches, the upper level ascending and the lower drooping »). [atlas_*]
- Modèle architectural de Hallé-Oldeman : **modèle de Rauh** (tronc monopodial à croissance rythmique, branches plagiotropes ramifiées). **Estimé** (connaissance générale ; non confirmé par une source lue dans cette recherche).
- Contrôle apical fort toute la vie (pas de bascule décurrente), contrairement au chêne du moteur : `apical_dominance_mature` reste élevé. **Estimé.** La flèche terminale peut être perdue (gel, insecte) et relayée par une branche verticillée redressée (« chandelle » ; à ne pas confondre avec une mort).
- Pousse annuelle monocyclique : un seul flush de printemps (débourrement tardif selon Wikipédia FR, sans date). Jusqu'à 1 m/an pour la flèche pendant 25 ans en bonnes conditions [wen_leader_growth], plus souvent 0,3-0,8 m ; **la longueur de pousse et l'espacement des verticilles sont une mesure directe de l'accroissement** (Wikipédia FR : de quelques cm à près d'un mètre par an).
- Élagage naturel : les branches basses ombragées meurent et persistent longtemps (branches mortes aciculées). Pas de valeur chiffrée trouvée. Le houppier vivant est long en peuplement ouvert, court en peuplement dense ; l'épicéa isolé garde ses branches jusqu'au sol.

## Feuillage

| Trait | Valeur proposée | Source | Statut |
|---|---|---|---|
| Type de feuille | aiguille 4-angulaire, rigide, 1-2,5 cm (Atlas), 15-25 mm (WP fr/en) | [atlas_needle_length] | mesuré (flore) |
| Persistance | persistant ; aiguilles gardées 5-7 ans (WP fr, pl), 4-7 ans en bonne santé et 1-3 ans sur arbre dépéri (WP de) ; davantage en altitude | [wfr_needle_longevity] [wde_needle_longevity] | Wikipédia |
| Phyllotaxie | spiralée, aiguilles insérées sur un pulvinus (décurrentes) | non sourcé | estimé |
| LMA | environ 200 g/m2 (SLA 50 ± 17 cm2/g, peuplements de 17 ans ; base projetée/totale inconnue). Le LMA augmente avec l'âge de l'aiguille | [sla85_lma] | dérivé d'un résumé |
| LAI | non trouvé de façon fiable (les résumés lus donnent des valeurs incohérentes entre elles) | n/d | manquant |
| Surface totale / projetée | rapport 2,2 à 4,0 selon la morphologie, d'après un résumé de recherche (non vérifié) | n/d | estimé |

Le feuillage alimente tous les calculs de lumière et de bois du moteur. Il faut remplacer « feuille caduque » par un **cohorte d'aiguilles par année de pousse** (voir « Ce que le moteur ne sait pas faire »).

## Phénologie

- Floraison : mai-juin (WP de) [wde_flowering]. Fleurs mâles à la base de la pousse de l'année précédente, femelles au bout de la pousse. Les cônes mûrissent la même année, septembre-octobre (WP fr) [wfr_cone_maturity] ; 5-7 mois après la pollinisation (WP en).
- Débourrement : « tardif » (WP fr), **aucune date en jour de l'année trouvée**. Assé et al. 2018 (Alpes françaises) : pas de retard significatif du débourrement de l'épicéa avec l'altitude (résumé de recherche, non lu). À calibrer sur Rennes ; l'épicéa n'est pas natif là et y dépend des gelées tardives.
- Senescence : sans objet (persistant).
- Mastings : floraison tous les 3-4 ans, environ 7 ans en montagne (WP de) ; forte production de graines tous les 3-4 ans. Au Royaume-Uni : bonnes récoltes tous les 5-10 ans [fr_coning].

## Reproduction

- Première floraison : 20-30 ans, jusqu'à 40 ans en peuplement dense (Atlas) [atlas_first_flowering]. Divergences : 30-40 ans (WP de, sv) ; fructification à partir de ~40 ans (Forest Research, Royaume-Uni). On garde les trois, médiane ~30 ans en plein découvert.
- Cône pendant, cylindrique, 12-15 cm (Atlas), 9-20 cm (WP en) ; graines ailées de 4-5 mm. Dispersion par le vent, plus de 300 m (WP de).

## Dimensions et croissance

| Trait | Valeur proposée | Source | Statut |
|---|---|---|---|
| Hauteur usuelle adulte | 35-40 m (WP fr) ; 50-60 m maximum (Atlas) ; record mesuré 62,71 m (Slovénie) | [wfr_height] [atlas_height_max] [wen_height_record] | mesuré / Wikipédia |
| Hauteur en peuplement, 100 ans | hauteur moyenne ~34 m pour une hauteur dominante de 36 m à 100 ans (Allemagne du Sud, table Assmann-Franz) | [utschig_h100_ass] | table de production |
| DBH | rarement > 1,5 m (WP fr, Atlas 150 cm) ; > 1,7 m à 2 m (WP de) | [atlas_dbh_max] [wfr_dbh] | mesuré / Wikipédia |
| Pousse de la flèche | jusqu'à 1 m/an pendant 25 ans | [wen_leader_growth] | Wikipédia |
| Accroissement du peuplement | 15 m3/ha/an en moyenne en Allemagne ; 17-22 m3/ha/an sur l'essai de Wolfratshausen ; pic de l'accroissement courant vers 40 ans (Wiedemann) | [wde_mai_germany] [utschig_incr] [utschig_cai_peak] | table / essai |
| Stock à 100 ans | ~1000 m3/ha (Bavière du Sud) | [utschig_vol100] | mesuré |
| Rotation | 50-70 ans en Grande-Bretagne ; 80-120 ans en Allemagne | [fr_rotation] [wde_longevity] | manuel |
| Arbre isolé (plein découvert) | **non trouvé** : aucune table d'âge-hauteur-DBH pour épicéa isolé. À ne pas confondre avec les tables de production, conçues pour des peuplements éclaircis | n/d | manquant |

Hauteurs d'échantillons finlandais (arbres excavés, forêt boréale mélangée) : 4,5 m à 16 ans et 22,6 m à 54 ans [kall_h16] [kall_h54].

## Longévité

200-300 ans normalement (Atlas), jusqu'à 500 ans (WP fr), 600 ans (WP de) ; individu daté par cernes : 532 ans (Norvège) ; clone racinaire d'environ 9 500 ans (Old Tjikko), sans équivalent pour une tige. En forêt gérée, exploité à 60-120 ans. [atlas_longevity_normal] [wen_oldest_ring_dated] [atlas_clonal_age]
Pour le génome : sénescence programmée **estimée** à partir de ~200 ans en sylviculture libre ; le vrai facteur de mortalité est le vent, le scolyte (Ips typographus) et la pourriture (Heterobasidion, Armillaria), pas l'âge.

## Racines (l'épicéa est traçant)

- Système **superficiel à pivot précoce vite remplacé par des traçantes et des sinkers** (Atlas : « Spruce has a shallow root system »). Les sinkers profonds apparaissent surtout entre 60 et 80 ans (WP de).
- Mesures finlandaises sur arbres excavés : profondeur maximale **1,62 m**, extension radiale maximale **10,1 m**, 80 % de la biomasse de grosses racines dans les 30 premiers cm, environ 70 % de la biomasse de racines fines dans l'humus et les 10 premiers cm du sol minéral [kall_depth] [kall_spread] [kall_crb30] [kall_frb10]. À comparer au pin sylvestre dans la même étude (2,91 m, 9,5 m).
- Pas de racines profondes sur sol engorgé ou compact : la profondeur est fixée par le sol. Tolère mal l'engorgement et la sécheresse estivale [atlas_waterlogging] [atlas_drought].
- Mycorhizes : champignons ectomycorhiziens ; l'Atlas n'indique pas le type, l'ectomycorhize des Pinacées est une connaissance générale **non sourcée ici**.
- Rejets : ne rejette pas de souche ; peut se marcotter (branches basses au sol, clone d'altitude) ; ne rejette pas après taille de la cime ou des branches basales (**estimé**).

## Bois (pour la mécanique et le poids de branche)

| Trait | Valeur | Source | Statut |
|---|---|---|---|
| Densité verte | 800 (700-850) kg/m3 | [wdeholz_green] (DIN 68364 via WP de) | Wikipédia |
| Densité à 12-15 % | 470 (330-680) kg/m3 | [wdeholz_dry12] | Wikipédia |
| MOR | 80 MPa (humidité non précisée) ; **pas de MOR vert trouvé**, ni de MOE vert | [wdeholz_bending] | manque |
| Densité verte moyenne du fût pour ForestGALES | 850-1000 kg/m3 typique des conifères | Hale et al. 2015 | manuel |

Bois clair, à cernes distincts (bois initial/final), canaux résinifères, non durable. L'épicéa n'est pas à pores différenciés (bois homoxylé de conifère) : `ring_porous` n'a pas de sens, il faut une structure trachéidique.

## Tolérances

- Ombre : tolérant, survit des décennies sous couvert et pousse vite après 5-10 ans de libération (Atlas) [atlas_shade] ; classé « tolerant » au Royaume-Uni.
- Sécheresse : peu tolérant (racines superficielles), très touché par les étés secs et la chaleur [atlas_drought].
- Engorgement : mal toléré, mais les racines restent superficielles, d'où le chablis [atlas_waterlogging].
- Gel : très rustique (environ -20 °C aux premiers gels, sous -60 °C en plein hiver, WP de) [wde_frost] ; les jeunes pousses sont sensibles aux gelées tardives (débourrement).
- Feu : très faible tolérance (Atlas). Sel : sensible aux embruns.
- Climat de Rennes (océanique, ~700 mm/an, étés secs de plus en plus chauds) : l'épicéa est **hors de sa niche** (Atlas : sensibilité à la chaleur et à la sécheresse hors de l'aire). Une plantation à Rennes doit exprimer ce stress : cime clairsemée, attaques de scolytes, chablis.

## Mécanique et tempêtes

L'épicéa est l'essence la plus étudiée sur ce sujet, mais les mesures directes de **fréquence propre** et de **coefficient d'ancrage Creg** pour *Picea abies* n'ont pas été obtenues par cette recherche : les accès aux textes de Jonsson et al. 2007 (Trees, fréquences propres sur pentes subalpines), Peltola et al. 2000 (traction, Finlande) et Nicoll et al. 2006 (Creg) étaient bloqués ou illisibles. À récupérer à la main.

Ce qui est établi :
- Épicéa de Sitka (proxy) : 368 essais de traction, moment de rupture 16-53 kNm, inclinaison critique 10-25° (Fraser & Gardiner 1967, rapporté par Andreozzi et al. 2025) [andreozzi_sitka] [andreozzi_tilt]. Le moteur utilise déjà Creg = 178,1 N m/kg (Sitka, sol drainé, enracinement profond) ; pour *Picea abies*, ForestGALES a un jeu de paramètres propre (non lu).
- Amortissement interne des conifères < 0,05 ; épicéa plus haute fréquence propre que les pins à taille égale (Moore & Maguire 2004, d'après résumé) [moore_damping].
- ForestGALES 2.3 en Écosse : vitesses critiques de dégâts de l'ordre de 20-40 m/s toutes essences [hale_cws].
- Lothar 1999, forêts pures de Suisse : sensibilité de l'épicéa environ trois fois celle du hêtre ; temps de retour du dommage 86-113 ans contre 357-408 ans ; un mélange de 10 % ou plus de feuillus ou de douglas diminue la vulnérabilité de l'épicéa (Schütz et al., résumé uniquement) [schutz_return]. Une autre synthèse lue rapporte des déracinements de 18,7 % pour le hêtre et 14,8 % pour l'épicéa (placettes permanentes suisses) : **divergence non résolue**, source primaire non lue, donc non reprise dans le fichier.
- Finlande, tempêtes Pyry/Janika/Mielikki : l'épicéa fait 14,8 % des arbres endommagés (le pin sylvestre dominait la zone) ; 33 % (Pyry) et 60 % (Mielikki) des épicéas atteints ont été déracinés, les autres cassés [zub_uproot].
- Klaus 2009 (Landes) : pas de donnée par essence trouvée (aucune source lue).
- Atlas : le système racinaire superficiel rend l'épicéa « moins résistant au chablis et aux chutes de pierres », surtout en peuplement pur et dense.

Mode de rupture : déracinement sur sol humide/superficiel, rupture de tige quand l'arbre est élancé et le sol sec (Zubizarreta, Hale) ; élancement H/D élevé en peuplement dense (> 80-100 : valeur non trouvée pour l'épicéa dans cette recherche).

## Réponse à la taille

Pas de rejets de souche. Les branches coupées ne se régénèrent pas ; une cime coupée est relayée par une branche verticillée redressée. Les blessures exsudent de la résine ; l'épicéa se compartimente mal et la pourriture (Heterobasidion, Armillaria) entre par les blessures et les racines. **Estimé** (aucune source lue dans cette recherche). Ne pas tailler de façon intensive : valeurs `epicormic_potential` et `stump_sprouting` proches de 0.

## Particularités et ce que le moteur ne sait pas faire

1. **Persistant** : le moteur suppose `deciduous: true` ; il faut une cohorte d'aiguilles (âge 1 à 5-7 ans, perte progressive de la plus âgée), pas de chute annuelle, photosynthèse hivernale limitée.
2. **Verticilles** : une branche principale par verticille (3-7, plus des intercalaires) et non une phyllotaxie 2/5 ; `lateral_buds_per_shoot` doit devenir un nombre de branches par verticille.
3. **Pousse monocyclique** à débourrement tardif, croissance déterminée (l'ébauche est préformée dans le bourgeon d'automne).
4. **Bois de conifère** (pas d'anneaux poreux), résine (blessures, scolytes).
5. **Racines traçantes** avec pivot précoce disparaissant : le modèle racinaire du chêne (cœur + pivots) ne convient pas.
6. **Mécanique** : Creg épicéa, densité verte 800 kg/m3, MOR/MOE verts manquants.
7. **Stress hors de l'aire** à Rennes : sécheresse estivale, scolytes (Ips typographus, arrivé au Royaume-Uni en 2018 et 2021 selon Forest Research), pourritures.
8. Cône femelle pendant de 12-15 cm : prévoir un affichage de cône si le rendu doit montrer les fruits.

## Ce qui n'a pas été trouvé

Hauteur et DBH de l'épicéa **isolé** par âge ; tables de production chiffrées (Wiedemann, Schober) : seulement la hauteur à 100 ans (Assmann-Franz) et des accroissements ; date de débourrement en jour de l'année ; LAI fiable ; MOR/MOE verts ; Creg et fréquence propre pour *Picea abies* ; dégâts de Lothar en France et de Klaus 2009 par essence ; masse sèche des aiguilles par unité de surface sur mesure directe.
