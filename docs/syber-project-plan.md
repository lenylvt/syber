# Plan Projet — Syber

> **Syber** est une alternative à Moonlight/Sunshine, construite entièrement en **Rust**, utilisant les technologies de **Kyber** (QUIC, VLC modifié, FFmpeg/txproto) pour atteindre une latence ultra-basse avec une architecture plus simple.

---

## Objectif

Créer un système de **remote desktop gaming / productivité** qui soit :

| Critère | Objectif Syber |
|---------|---------------|
| **Latence** | < 20ms glass-to-glass sur LAN |
| **Simplicité** | Un seul binaire serveur, un seul binaire client |
| **Configuration** | Zéro config pour le cas nominal (LAN, 1 utilisateur) |
| **Qualité vidéo** | H.264/H.265 hardware (NVENC, QSV, AMF) |
| **Entrées** | Clavier, souris, gamepad (XInput/ViGEm) |
| **Plateformes** | Windows first, Linux second |
| **Langage** | 100% Rust |

---

## Comparaison avec l'existant

| | Moonlight | Sunshine | **Syber** |
|--|-----------|----------|-----------|
| Protocole | NV propriétaire (RTSP+RTP) | NV propriétaire | **QUIC** |
| Chiffrement | TLS | TLS | **TLS 1.3 natif QUIC** |
| Multi-stream sur 1 connexion | ❌ | ❌ | **✅** |
| Latence | ~15ms LAN | ~15ms LAN | **< 10ms LAN** (objectif) |
| Langage | C++ | C++ | **Rust** |
| Dépendance GeForce | ✅ (NV only) | ✅ | **❌ (tout GPU)** |
| Architecture | Monolithique | Monolithique | **Multi-process** |

---

## Architecture Syber

Syber est une version simplifiée de l'architecture Kyber, adaptée au cas d'usage remote desktop :

```
                    SYBER SERVER
┌──────────────────────────────────────────────────────┐
│                                                      │
│  syber-server (binaire unique)                       │
│  ┌────────────────────────────────────────────────┐  │
│  │                                                │  │
│  │  ┌─────────────┐    ┌──────────────────────┐   │  │
│  │  │   Capture   │    │   Input Injector     │   │  │
│  │  │  (DXGI/X11) │    │   (kynput server)    │   │  │
│  │  └──────┬──────┘    └──────────┬───────────┘   │  │
│  │         │                      │               │  │
│  │         ▼                      ▲               │  │
│  │  ┌─────────────┐    ┌──────────┴───────────┐   │  │
│  │  │   Encoder   │    │     kymux serveur    │   │  │
│  │  │ (avserver/  │    │     (QUIC :8080)     │   │  │
│  │  │  txproto)   │───►│                      │   │  │
│  │  └─────────────┘    └──────────────────────┘   │  │
│  │                                                │  │
│  │  Auth: JWT ou "no-auth" (LAN simple)           │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
                         │ QUIC :8080
┌──────────────────────────────────────────────────────┐
│                  SYBER CLIENT                        │
│                                                      │
│  syber-client (binaire unique)                       │
│  ┌────────────────────────────────────────────────┐  │
│  │                                                │  │
│  │  ┌─────────────┐    ┌──────────────────────┐   │  │
│  │  │  VLC 0-lat  │    │   Input Capture      │   │  │
│  │  │  + decoder  │    │   (kynput client)    │   │  │
│  │  └──────┬──────┘    └──────────┬───────────┘   │  │
│  │         │                      │               │  │
│  │         ▲                      ▼               │  │
│  │  ┌──────┴───────────────────────────────────┐  │  │
│  │  │           kymux client (QUIC)            │  │  │
│  │  └──────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

---

## Structure du projet Rust

```
syber/
├── Cargo.toml                  ← Workspace Rust
├── Cargo.lock
├── docs/                       ← Cette documentation
│
├── syber-server/               ← Binaire serveur
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs           ← Chargement config TOML
│       ├── capture/            ← Capture écran (DXGI, X11)
│       │   ├── mod.rs
│       │   ├── dxgi.rs         ← Windows
│       │   └── x11.rs          ← Linux
│       ├── encoder/            ← Intégration avserver/FFmpeg
│       │   ├── mod.rs
│       │   └── ffmpeg.rs
│       ├── input/              ← Injection entrées (libkynput)
│       │   ├── mod.rs
│       │   ├── windows.rs
│       │   └── linux.rs
│       └── mux/                ← Intégration kymux
│           └── mod.rs
│
├── syber-client/               ← Binaire client
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── player/             ← VLC 0-latency
│       │   └── mod.rs
│       ├── input/              ← Capture entrées (libkynput)
│       │   ├── mod.rs
│       │   ├── windows.rs
│       │   └── linux.rs
│       └── mux/                ← Intégration kymux
│           └── mod.rs
│
└── syber-common/               ← Types partagés client/serveur
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── config.rs
        └── protocol.rs         ← Types de paquets, messages
```

---

## Roadmap

### Phase 1 — Foundation (MVP)

- [ ] Setup du workspace Rust
- [ ] Intégration kymux (connexion QUIC basique)
- [ ] Capture DXGI (Windows) + encodage NVENC via FFmpeg
- [ ] Décodage côté client via VLC 0-latency
- [ ] Transmission clavier + souris basique
- [ ] Authentification "no-auth" (LAN only)
- [ ] **Objectif** : stream fonctionnel sur LAN Windows → Windows

### Phase 2 — Stabilité & Qualité

- [ ] Support encodeurs AMD (AMF) et Intel (QSV)
- [ ] Support Linux X11 (capture + injection)
- [ ] Support gamepad (kynput + ViGEm)
- [ ] Authentification JWT
- [ ] Configuration via fichier TOML
- [ ] Reconnexion automatique
- [ ] Logs structurés (tracing)

### Phase 3 — Fonctionnalités avancées

- [ ] Client Web (WebAssembly + WebTransport)
- [ ] Support macOS (client + serveur)
- [ ] Partage clipboard
- [ ] Multi-moniteurs
- [ ] Interface graphique (optionnelle)
- [ ] Transfert de fichiers

---

## Dépendances Rust clés

```toml
[workspace.dependencies]
# Transport réseau (Kyber)
kymux = { git = "https://gitlab.com/kyber.stream/core/kymux" }

# Entrées/sorties (Kyber)
libkynput = { git = "https://gitlab.com/kyber.stream/core/kynput" }

# FFmpeg bindings (pour avserver)
ffmpeg-next = "7"           # ou txproto bindings

# Async runtime
tokio = { version = "1", features = ["full"] }

# Configuration
serde = { version = "1", features = ["derive"] }
toml = "0.8"

# Logs
tracing = "0.1"
tracing-subscriber = "0.3"

# CLI
clap = { version = "4", features = ["derive"] }
```

---

## Configuration serveur (`syber_server.toml`)

```toml
[server]
port = 8080
host = "0.0.0.0"

[video]
codec = "h264"        # h264 | h265 | vp9
encoder = "nvenc"     # nvenc | qsv | amf | vaapi | x264
bitrate_kbps = 10000
fps = 60
width = 1920
height = 1080

[audio]
codec = "opus"
bitrate_kbps = 128

[auth]
mode = "none"         # none (LAN) | jwt
# jwt_key = "secret"  # si mode = "jwt"
```

---

## Configuration client (`syber_client.toml`)

```toml
[server]
host = "192.168.1.10"
port = 8080

[display]
fullscreen = false
vsync = false

[input]
gamepad = true
clipboard = false
```

---

## Points techniques critiques

### 1. VLC 0-latency

VLC doit être compilé avec le patch Kyber (`--0latency`). Utiliser le fork officiel de Kyber :

```
gitlab.com/kyber.stream/deps/vlc
```

### 2. FFmpeg push-mode

avserver/txproto est configuré en **push mode** : les frames sont poussées vers le réseau dès qu'elles sont encodées, sans attendre de synchronisation d'horloge.

### 3. QUIC sans P2P

Pour un accès depuis Internet :
- Option A : **Tailscale / WireGuard VPN** (recommandé, zero-config)
- Option B : Port forwarding + authentification JWT obligatoire
- Option C (future) : Relai QUIC (serveur de médiation)

### 4. Windows Service (production)

Pour la capture du Secure Desktop (UAC/logon), implémenter `syber-service.exe` sur le modèle de `service.exe` de Kyber.
