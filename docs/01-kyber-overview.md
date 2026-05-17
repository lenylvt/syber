# Vue d'ensemble de Kyber

## Qu'est-ce que Kyber ?

Kyber est une **solution open source cross-platform à très faible latence** pour le contrôle à distance de tout type de machine : du bureau distant aux drones et robots, en passant par le rendu cloud et le gaming, ou les agents visuels IA.

Il est construit sur **VLC**, **FFmpeg** et le protocole **QUIC**.

> **Objectif principal** : atteindre la latence absolument la plus basse possible, car *la latence compte plus que tout lorsqu'on contrôle une machine*.

### Performances mesurées

| Métrique | Valeur |
|----------|--------|
| Latence glass-to-glass (LAN optimisé) | **~8 ms** |
| Latence glass-to-glass (conditions normales) | **20–25 ms** |
| Port réseau utilisé | `8080` TCP+UDP (QUIC) |

---

## Cas d'usage

| Domaine | Exemple |
|---------|---------|
| Remote Desktop | Remplacement de TeamViewer, VNC, RDP |
| Cloud Gaming | Streaming de jeux depuis un serveur GPU |
| Cloud Rendering | Rendu 3D interactif à distance |
| Robotique | Contrôle/observation de robots en temps réel |
| Drones | Télépilotage avec retour vidéo ultra-faible latence |
| Agents IA visuels | Flux vidéo pour entraînement et inférence de modèles |
| Télémédecine | Contrôle d'instruments médicaux à distance |
| Application Streaming | Virtualisation d'applications |

---

## Pourquoi Kyber plutôt que les alternatives ?

| Solution | Latence typique | Open Source | Cross-platform | Multi-codec | QUIC |
|----------|----------------|-------------|----------------|-------------|------|
| **Kyber** | **8–25 ms** | ✅ AGPL | ✅ Win/Mac/Linux/Web | ✅ H264/HEVC/VP9/AV1 | ✅ |
| Moonlight/Sunshine | ~15–30 ms | ✅ | Partiel | H264/HEVC | ❌ |
| WebRTC | 50–150 ms | ✅ | ✅ | Limité | Partiel |
| RDP | 50–200 ms | ❌ | Partiel | Propriétaire | ❌ |
| Parsec | ~20 ms | ❌ | ✅ | Propriétaire | ❌ |

---

## Principes techniques fondamentaux

### 1. Pipeline entièrement push-based

Contrairement aux lecteurs multimédias traditionnels (pull-based avec buffers), Kyber force chaque image à être envoyée au décodeur **dès réception réseau**, sans attendre la synchronisation. Résultat : zéro buffer intermédiaire = latence minimale.

### 2. Un seul port, un seul protocole

Tout (vidéo, audio, entrées clavier/souris, gamepad, clipboard) transite sur **une unique connexion QUIC chiffrée** via le port `8080`. Cela simplifie les configurations firewall/NAT et réduit la surface d'attaque.

### 3. Architecture multi-processus stateless

Chaque composant (streamer, serveur d'entrées, mux réseau) est un **processus séparé et stateless**. Un crash d'un composant n'affecte pas les autres. La reconnexion est transparente.

### 4. Réutilisation de VLC et FFmpeg

- **FFmpeg** (libavcodec + libavfilter) : capture, encodage, filtres vidéo côté serveur
- **VLC modifié** (`--0latency`) : décodage et affichage côté client, toutes plateformes

### 5. Rust partout (sauf VLC/FFmpeg)

Tout le nouveau code est en Rust pour la sécurité mémoire, les performances et la portabilité.

---

## Licence

Kyber utilise un **double modèle de licence** :

| Usage | Licence |
|-------|---------|
| Open source / non commercial | GNU AGPL v3.0 ou ultérieur |
| Intégration commerciale propriétaire | Licence commerciale (payante) |

---

## Fondateurs et contexte

- **Jean-Baptiste Kempf** — Fondateur de Kyber, lead developer de VLC (6 milliards de téléchargements), contributeur FFmpeg
- Annoncé à **KDE Akademy 2023**, présenté à **Demuxed '23** (San Francisco) et **Mile High Video 2025**
- Code source publié sur [GitLab](https://gitlab.com/kyber.stream/kyber)

---

## Limites actuelles connues

- **Pas de peer-to-peer** : QUIC ne supporte pas le NAT traversal natif → nécessite VPN ou exposition du port réseau
- **AV1** : support hautement expérimental (limitations navigateur)
- **Wayland** : support Linux Wayland "bientôt disponible"
- **Interface graphique** : pas encore d'UI graphique pour le client desktop
- **macOS serveur** : "bientôt disponible"
