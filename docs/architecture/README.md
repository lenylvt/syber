# Architecture générale de Kyber

## Vue macro

Kyber est une **architecture multi-agents** : l'ensemble des fonctionnalités est découpé en processus spécialisés, communicant entre eux via IPC (actuellement TCP, avec évolution prévue vers pipes/mémoire partagée).

```
┌─────────────────────────────────────────────────────────────────────┐
│                           SERVEUR (HOST)                            │
│                                                                     │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐  │
│  │   Streamer   │    │ Input Server │    │    USB Service       │  │
│  │  (avserver)  │    │ (libkynput)  │    │  (USB forwarding)    │  │
│  │  FFmpeg-based│    │  Inject I/O  │    │  Virtual USB driver  │  │
│  └──────┬───────┘    └──────┬───────┘    └──────────┬───────────┘  │
│         │                  │                        │              │
│         └──────────────────┴────────────────────────┘              │
│                            │ IPC (TCP → pipes)                     │
│                      ┌─────▼──────┐                                │
│                      │    Mux     │  ◄── QUIC, WebTransport        │
│                      │  (kymux)   │                                │
│                      └─────┬──────┘                                │
│                            │                                       │
│                      ┌─────▼──────┐                                │
│                      │ Controller │  ◄── HTTP/WSS (contrôle)       │
│                      │  (kyctl)   │      Auth, Orchestration       │
│                      └────────────┘                                │
└─────────────────────────────────────────────────────────────────────┘
                              │
                          RÉSEAU QUIC
                         port 8080 TCP+UDP
                              │
┌─────────────────────────────────────────────────────────────────────┐
│                           CLIENT                                    │
│                                                                     │
│  ┌──────────────┐    ┌──────────────┐                              │
│  │  VLC modifié │    │  libkynput   │                              │
│  │  (0-latency) │    │  (capture)   │                              │
│  │  Décodage +  │    │  Clavier,    │                              │
│  │  Affichage   │    │  Souris,     │                              │
│  └──────┬───────┘    │  Gamepad     │                              │
│         │            └──────┬───────┘                              │
│         └──────────────────┤                                       │
│                      ┌─────▼──────┐                                │
│                      │    Mux     │  ◄── QUIC / WebTransport       │
│                      │  (kymux)   │                                │
│                      └────────────┘                                │
│                      (libclient)                                    │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Flux de données

### Flux descendant (serveur → client)

```
Écran du serveur
    │
    ▼ (DXGI / X11 / etc.)
[Capture vidéo]  ──── [Capture audio]
    │                      │
    ▼                      ▼
[Encodage H264/HEVC/VP9/AV1]  [Encodage audio]
    │                      │
    └──────────────────────┘
                │
                ▼ IPC
            [kymux serveur]
                │
                ▼ QUIC (port 8080)
            [kymux client]
                │
                ▼ IPC
            [VLC 0-latency]
                │
                ▼
        [Décodage hardware]
                │
                ▼
           [Affichage]
```

### Flux montant (client → serveur)

```
Clavier / Souris / Gamepad
    │
    ▼
[libkynput capture]
    │
    ▼
[InputRouter client]
    │
    ▼ IPC
[kymux client]
    │
    ▼ QUIC (port 8080)
[kymux serveur]
    │
    ▼ IPC
[libkynput injection]
    │
    ▼
Système du serveur (injection OS)
```

---

## Séparation des responsabilités

| Composant | Rôle | Processus |
|-----------|------|-----------|
| **Controller** | Point d'entrée, auth, orchestration | ✅ Permanent |
| **Streamer (avserver)** | Capture + encodage vidéo/audio | ✅ Redémarrable |
| **Input Server (kynput)** | Injection des entrées clavier/souris | ✅ Séparé |
| **USB Service** | Forwarding USB client→serveur | ✅ Séparé |
| **Mux (kymux)** | Transport QUIC, multiplexage | ✅ Séparé |

### Pourquoi multi-processus ?

1. **Isolation des crashs** : un crash du streamer n'interrompt pas la connexion réseau
2. **Redémarrage à chaud** : changement de résolution/codec = redémarrer le streamer seulement
3. **Développement parallèle** : équipes indépendantes sur chaque composant
4. **Stateless** : chaque processus peut redémarrer sans état partagé complexe

---

## Plan de communication

| Canal | Protocole | Usage |
|-------|-----------|-------|
| Client ↔ Controller | HTTPS | Authentification, commandes REST |
| Client ↔ Controller | WSS (WebSocket sécurisé) | Notifications temps réel |
| Client ↔ Mux | QUIC (natif) ou WebTransport (web) | Données : vidéo, audio, entrées |
| Composants internes | IPC TCP (→ pipes/SHM) | Communication inter-processus locale |

---

## Principes de conception

- **Push-based partout** : pas de buffer, pas de synchronisation, affichage de la dernière frame reçue
- **Stateless** : facilite les redémarrages et la résistance aux pannes
- **Modulaire** : chaque brique est remplaçable (ex: remplacer le streamer FFmpeg par une autre implémentation)
- **SDK-first** : l'architecture est pensée comme un SDK réutilisable, pas seulement une application
