# Composants Core de Kyber

## Structure du dépôt

Kyber est organisé en **plusieurs dépôts GitLab** répartis en groupes :

```
gitlab.com/kyber.stream/
├── kyber/           ← dépôt principal (documentation, point d'entrée)
│
├── core/            ← composants spécifiques à Kyber
│   ├── kyctl        ← plan de contrôle
│   ├── kymux        ← plan de données / réseau
│   ├── kynput       ← gestion des entrées/sorties
│   ├── kymedia      ← composants multimédia
│   └── kyutil       ← utilitaires partagés
│
├── deps/            ← forks de dépendances avec patches
│   ├── vlc          ← VLC modifié (--0latency)
│   ├── ffmpeg       ← FFmpeg si patches nécessaires
│   └── txproto      ← streaming solution basée sur FFmpeg
│
└── apps/            ← applications finales
    ├── kyber-desktop   ← client/serveur Windows, macOS, Linux
    ├── kyber-web       ← client navigateur
    ├── kyber-android   ← client Android (bientôt)
    └── kyber-ios       ← client iOS (bientôt)
```

---

## Vue d'ensemble des composants core

| Composant | Dépôt | Rôle | Langage |
|-----------|-------|------|---------|
| [kyctl](./kyctl.md) | `core/kyctl` | Plan de contrôle — auth, orchestration | Rust |
| [kymux](./kymux.md) | `core/kymux` | Plan de données — QUIC, WebTransport, IPC | Rust |
| [kynput](./kynput.md) | `core/kynput` (libkynput) | Capture et injection d'entrées | Rust |
| [kymedia](./kymedia.md) | `core/kymedia` | Capture vidéo/audio, encodage/décodage | Rust + FFmpeg |
| [kyutil](./kyutil.md) | `core/kyutil` | IPC, utilitaires partagés | Rust |

---

## Dépendances entre composants

```
kyber-desktop (app)
    │
    ├── kyctl (controller)
    │       ├── kymux
    │       ├── avserver ──── kymedia ──── ffmpeg/txproto
    │       └── libkynput ─── kyutil
    │
    └── libclient
            ├── kymux
            ├── vlc (modifié)
            ├── libkynput
            └── kyaudioreg
```

---

## Composants intermédiaires notables

### txproto

Solution de streaming open source basée sur FFmpeg, intégrée dans Kyber via `avserver`.
- `txproto` : pipeline d'encodage/décodage avec filtres via libavcodec + libavfilter
- `avserver` : intègre txproto dans l'environnement Kyber

### libkypc

Bibliothèque pour **spawner des processus** et établir la communication IPC (Inter-Process Communication) entre eux.

### kyaudioreg

Registre audio pour :
- Buffering audio faible latence
- Correction de la dérive audio (clock drift)
- Utilisé par VLC et le lecteur audio web

### kywasmtime

Implémentation de `tokio::time` pour les plateformes WebAssembly où les APIs standard de temps ne sont pas disponibles.

---

## Détail par composant

- [kyctl — Plan de contrôle](./kyctl.md)
- [kymux — Plan de données](./kymux.md)
- [kynput — Gestion des entrées](./kynput.md)
- [kymedia — Multimédia](./kymedia.md)
- [kyutil — Utilitaires](./kyutil.md)
