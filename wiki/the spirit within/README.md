# Doka — transcription structurée de deux schémas manuscrits

> Reconstruction interprétative. Les flèches et certains libellés des originaux sont ambigus : les liens indiqués ci-dessous doivent être validés avant implémentation.

## 1. Modèle documentaire

Source : première image, avec les mentions `units`, `items`, `chunks`, `Vespa`, `embeddings`.

- `Unit` : annotation lisible « .eml + simple meta ».
- `Items` : annotation « after split » et « item + meta + blobs + tags ».
- Deux objets illustratifs A et B, avec des métadonnées.
- `PJ1` et `PJ2` : pièces jointes probables, avec tags ; leur rattachement exact est incertain.
- Les `chunks` sont dessinés en groupes sous les objets.
- En bas : « Vespa » et « embeddings ».
- En haut : « Ex: email + 2 PJ ».

### Diagramme Mermaid

```mermaid
flowchart TB
  U["Unit = .eml + simple meta"]
  I["Items (after split)\nitem + meta + blobs + tags"]
  A["Item A"]
  B["Item B"]
  C1["Chunks A"]
  C2["Chunks B"]
  E1["Vespa"]
  E2["Embeddings"]
  PJ1["PJ1 + tags"]
  PJ2["PJ2 + tags"]
  U --> I
  I --> A
  I --> B
  A --> C1
  B --> C2
  C1 --> E1
  C2 --> E2
  A -.-> PJ1
  B -.-> PJ2
```

## 2. Architecture de traitement

Source : seconde image.

- Point d'entrée : `Doka`, mention `Client: .../api/` (lecture incomplète).
- `Units` / `LX`.
- Stockage `S3 (Garage)`.
- Transport/événements `Kafka Lite`.
- `Split` → `Items`.
- Services `Tika`, `Lexem`, `Smart Chunk`, `Embedding`, `TAG Generation`.
- Destination indiquée : `Vespa - multi node`.

### Diagramme Mermaid

```mermaid
flowchart TB
  CLIENT["Doka client / API\n(annotation partiellement illisible)"]
  UNIT["Units / LX"]
  S3["S3 (Garage)"]
  K["Kafka Lite"]
  SPLIT["Split"]
  ITEMS["Items"]
  TIKA["Tika"]
  LEX["Lexem"]
  EMB["Embedding"]
  SMART["Smart Chunk"]
  TAG["TAG Generation"]
  V["Vespa - multi node"]
  CLIENT --> UNIT
  UNIT --> S3
  UNIT --> K
  S3 --> SPLIT
  SPLIT --> ITEMS
  K --> TIKA
  K --> SMART
  K --> TAG
  TIKA --> LEX
  SMART --> EMB
  EMB --> V
  TAG --> V
```

## Incertitudes à valider

1. Les flèches précises entre Kafka Lite, S3, Split et les traitements sont partiellement superposées sur le dessin.
2. Le libellé exact de l'API et le rôle de `LX` restent à confirmer.
3. `Lexem` et `TAG Generation` sont retranscrits phonétiquement / visuellement ; noms exacts à confirmer.
4. La distinction entre stockage de chunks dans Vespa et production d'embeddings mérite une précision.
5. Les annotations `PJ1`, `PJ2` et leurs métadonnées sont difficiles à lire.

Les SVG sont des reconstructions vectorielles et non des calques fidèles des traits manuscrits.
