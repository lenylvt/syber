# Syber

> Alternative à Moonlight/Sunshine — ultra-low latency remote desktop, construit sur les technologies **Kyber** (QUIC, H.264, kynput).

## Architecture

```
┌──────────────────────────────────────┐
│         syber-server                 │  Windows + Linux
│  xcap → H.264 → kyproto QUIC        │
│  kynput injection des entrées        │
└──────────────────────────────────────┘
              │ QUIC :8080 (TLS 1.3)
┌──────────────────────────────────────┐
│         syber-client                 │  Windows + Linux + macOS
│  kyproto QUIC → H.264 → egui        │
│  kynput capture des entrées          │
└──────────────────────────────────────┘
```

**Stack Kyber utilisée :**
- `kyproto` / `kynet` — transport QUIC chiffré (TLS 1.3)
- `kymux-types` — types de paquets (AVPacket, InputPacket)
- `kynput` — capture/injection d'entrées (clavier, souris, gamepad)

**Stack vidéo :**
- Capture : `xcap` (cross-platform)
- Encodage : `openh264` (H.264 logiciel, zero dépendance système)
- Décodage : `openh264` 

**UI :** `egui` / `eframe` — immédiat, minimaliste, tout paramétrable sans éditer de fichier.

## Build

```bash
# Prérequis : Rust stable, xcap dépendances système

# Serveur (Windows/Linux)
cargo build --bin syber-server --release

# Client (Windows/Linux/macOS)
cargo build --bin syber-client --release
```

## Utilisation

### Serveur

```bash
./syber-server
```

1. L'empreinte TLS s'affiche dans le panneau gauche → la copier
2. Paramètres simples : port, mot de passe, qualité
3. Paramètres avancés : codec, encodeur, bitrate, FPS, résolution, protocole
4. Cliquer **▶ Démarrer**

### Client

```bash
./syber-client
```

1. Entrer : Hôte, Port, Mot de passe, Empreinte TLS
2. Cliquer **Connecter**
3. **Ctrl+Shift+Q** pour se déconnecter

## Paramètres

### Serveur — Simple
| Paramètre | Description |
|-----------|-------------|
| Port | Port d'écoute (défaut: 8080) |
| Mot de passe | Token d'authentification |
| Qualité | Rapide / Équilibré / Qualité max |
| Écran | Moniteur à streamer |

### Serveur — Avancé
| Paramètre | Description |
|-----------|-------------|
| Codec | H.264 / H.265 |
| Encodeur | Auto / Logiciel / NVENC / QSV / AMF / VA-API |
| Débit | Bitrate en kbps |
| FPS | Fréquence d'images |
| Résolution | Scale 25%-100% |
| Protocole vidéo | Reliable / GOP Stream / Unreliable+FEC |

### Client — Paramètres
| Paramètre | Description |
|-----------|-------------|
| Empreinte TLS | SHA-256 du cert serveur (sécurité) |
| Capture clavier/souris | Activer/désactiver la transmission des entrées |
| Overlay stats | FPS, bitrate, RTT en overlay |
| Plein écran auto | Passer en plein écran à la connexion |

## Sécurité

- Connexion chiffrée **TLS 1.3** via QUIC
- Authentification par **token** (mot de passe)
- Validation du serveur par **empreinte SHA-256** (pas de CA requis)
- Idéal pour LAN / VPN

## Dépendances Kyber (vendor/)

```bash
# Mise à jour des sources Kyber
./update_vendor.sh

# Sauvegarde complète
./update_vendor.sh --snapshot

# État des repos
./update_vendor.sh --status
```

## Structure du projet

```
Syber/
├── Cargo.toml                    # Workspace Rust
├── .cargo/config.toml            # Patches → vendor/
├── crates/
│   ├── syber-common/             # Config partagée
│   ├── syber-server/             # Binaire serveur
│   └── syber-client/             # Binaire client
├── vendor/                       # Sources Kyber (clonées localement)
│   ├── kymux/    kynet/kyproto  # Transport QUIC
│   ├── kynput/                   # Entrées
│   ├── kyutil/   libkypc        # IPC
│   └── deps/     vlc txproto…  # Dépendances Kyber
├── docs/                         # Documentation complète
└── update_vendor.sh              # Script de mise à jour
```
