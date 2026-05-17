# Syber — Documentation

> **Syber** est un projet Rust inspiré de [Kyber](https://gitlab.com/kyber.stream/kyber), visant à construire une alternative à Moonlight/Sunshine — plus simple, plus rapide, et fondée sur les technologies de Kyber.

---

## Table des matières

| # | Document | Description |
|---|----------|-------------|
| 1 | [Vue d'ensemble de Kyber](./01-kyber-overview.md) | Qu'est-ce que Kyber, ses objectifs et ses cas d'usage |
| 2 | [Architecture générale](./architecture/README.md) | Vue macro de l'architecture client/serveur |
| 3 | [Architecture serveur](./architecture/server.md) | Pipeline capture → encodage → envoi |
| 4 | [Architecture client](./architecture/client.md) | VLC modifié, décodage, affichage zéro-latence |
| 5 | [Réseau & QUIC](./network/quic.md) | Protocole QUIC, kymux, multiplexage |
| 6 | [Composants core](./components/README.md) | kyctl, kymux, kynput, kymedia, kyutil |
| 7 | [Système d'entrées/sorties](./components/kynput.md) | Clavier, souris, gamepad, USB forwarding |
| 8 | [Authentification](./architecture/authentication.md) | TLS, sessions, backends JWT/Basic/HTTPS |
| 9 | [Support des plateformes](./platform/compatibility.md) | Windows, macOS, Linux, Web, Android, iOS |
| 10 | [SDK & Bindings](./sdk/README.md) | C, JavaScript/WASM, Kotlin/JNI, Swift |
| 11 | [Système de build](./build/README.md) | Scripts, contrib, workspace, cross-compilation |
| 12 | [Plan projet Syber](./syber-project-plan.md) | Notre implémentation Rust — architecture & roadmap |

---

## À propos de Kyber

**Kyber** est un SDK open source (AGPL v3 / licence commerciale) créé par **Jean-Baptiste Kempf** (fondateur de VLC / VideoLAN), qui atteint **8 ms de latence glass-to-glass** — la meilleure performance connue dans sa catégorie.

- Site officiel : [kyber.media](https://kyber.media)
- Dépôt principal : [gitlab.com/kyber.stream/kyber](https://gitlab.com/kyber.stream/kyber)
- Langages : **Rust** (code neuf), **C** (VLC/FFmpeg)

## À propos de Syber

**Syber** réutilise les briques de Kyber pour offrir une expérience de streaming de bureau similaire à Moonlight/Sunshine, avec :

- Une architecture **plus simple** (moins de processus, configuration minimale)
- Une **latence ultra-basse** grâce à QUIC + FFmpeg push-mode + VLC 0-latency
- Un code entièrement en **Rust**
- Un focus sur le **remote desktop gaming / productivité**
