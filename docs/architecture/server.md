# Architecture Serveur

## Rôle du serveur

Le serveur (Host) a pour mission :

1. **Capturer** l'écran et l'audio de la machine
2. **Encoder** les flux vidéo et audio aussi vite que possible
3. **Injecter** les événements d'entrée envoyés par le client
4. **Transmettre** tous les flux vers le client via kymux/QUIC

---

## Composants serveur

### Controller (`kyctl`)

**Processus permanent** — point d'entrée du serveur.

- Expose un **serveur HTTP/HTTPS** pour recevoir les commandes clients
- Gère l'**authentification** (JWT, Basic, HTTPS)
- **Lance et supervise** les autres processus (streamer, input server, USB service, mux)
- Redémarre un processus crashé automatiquement
- Propage les autorisations utilisateurs à chaque service
- Gère les **sessions multi-clients** (plusieurs utilisateurs sur la même machine)

```
Client REST: POST /session/login
Client REST: POST /start_mux
Client REST: POST /start_stream
Controller → lance avserver, kynput, kymux
```

### Streamer (`avserver` basé sur `txproto`/FFmpeg)

**Processus redémarrable** — capture et encodage.

- Basé sur **FFmpeg** (libavcodec + libavfilter) via `txproto`
- Décrit le pipeline de capture/encodage de façon **dynamique** (configuration à chaud)
- Supporte tous les encodeurs hardware et software disponibles

#### Pipeline de capture

```
[Module de capture OS-spécifique]
        │
        ├── GPU path (Direct3D / DXGI sur Windows)
        │       │
        │       ▼
        │   [Texture Direct3D/GPU]
        │       │
        │       ▼
        │   [NVENC / AMF / QSV] ──────► [Paquet encodé]
        │
        └── CPU path
                │
                ▼
            [Frame CPU]
                │
                ▼
            [Filtre optionnel: resize, colorimetrie, etc.]
                │
                ▼
            [x264 / VA-API / autre encodeur] ──► [Paquet encodé]
```

#### Modules de capture par plateforme

| Plateforme | API de capture | GPU direct |
|-----------|---------------|------------|
| Windows | DXGI Desktop Duplication | ✅ Direct3D texture |
| Linux (X11) | X11/XShm | ❌ CPU (VAAPI expérimental) |
| Linux (Wayland) | Pipewire (bientôt) | ❌ |
| macOS | ScreenCaptureKit | ✅ (bientôt) |

#### Encodeurs supportés

| Encodeur | Type | GPU | Plateforme |
|----------|------|-----|-----------|
| x264 | H.264 | ❌ CPU | Toutes |
| NVENC | H.264 / HEVC | ✅ NVIDIA | Win/Linux |
| QSV | H.264 / HEVC | ✅ Intel | Win/Linux (expérimental) |
| AMF | H.264 / HEVC | ✅ AMD | Win (expérimental) |
| VA-API | H.264 / HEVC | ✅ | Linux (expérimental) |
| VideoToolbox | H.264 / HEVC | ✅ Apple | macOS |

#### Codecs vidéo supportés

| Codec | Statut |
|-------|--------|
| H.264 | ✅ Production |
| H.265 / HEVC | ✅ Production |
| VP9 | ✅ Production |
| AV1 | ⚠️ Expérimental |

### Input Server (`libkynput` — serveur)

- **Reçoit** les événements d'entrée depuis kymux (via IPC)
- **Injecte** clavier, souris, gamepad dans l'OS serveur
- Gère les **desktop virtuels** (Secure Desktop, UAC) sur Windows via le service Windows

#### Sur Windows : `service.exe` + `controller.exe`

```
service.exe  (Session 0, LOCAL_SYSTEM)
    │
    │ CreateProcessAsUser()
    ▼
controller.exe  (session interactive)
    │
    ├── Capture DXGI normale ✅
    ├── Capture Secure Desktop (UAC/logon) ✅
    └── Injection inputs sur bureaux protégés ✅
```

> Un `controller.exe` lancé manuellement (sans service) peut capturer le bureau normal, mais **pas** le Secure Desktop / UAC.

### USB Service

- Crée des **drivers USB virtuels** côté serveur
- Redirige les paquets USB depuis le client → appareil virtuel côté serveur
- Protocole basé sur **usbip** (Linux kernel)
- Le serveur "voit" le périphérique comme s'il était branché physiquement

### Mux serveur (`kymux`)

- Reçoit les paquets encodés de avserver, kynput, USB via IPC
- **Multiplexe** tout sur une unique connexion QUIC chiffrée
- Gère la connexion réseau entrante du client
- Démultiplexe les entrées reçues et les route vers les services appropriés

---

## Exemple de pipeline complet (Windows, GPU NVIDIA)

```
Écran Windows
    │
    ▼ DXGI Desktop Duplication
[Texture Direct3D sur GPU]
    │
    ▼ NVENC (H.264, 60 fps, 10 Mbps)
[Paquets NAL H.264]
    │
    ▼ IPC (TCP local → pipes)
[kymux serveur]
    │
    ▼ QUIC stream #1 (vidéo)
    ...
```

## Exemple de pipeline complet (Linux, CPU/VA-API)

```
Écran X11
    │
    ▼ X11/XShm (CPU)
[Frame YUV sur CPU]
    │
    ▼ VA-API (encodage Intel GPU) ou x264 (CPU)
[Paquets H.264]
    │
    ▼ IPC
[kymux serveur]
    │
    ▼ QUIC
    ...
```

---

## Gestion des threads

Chaque étape du pipeline a son **propre thread d'exécution**. Les données passent entre les étapes via des **FIFOs** dédiées :

```
Thread Capture → FIFO → Thread Encodage → FIFO → Thread IPC → kymux
```

Cela garantit que chaque composant opère à son rythme optimal sans blocage.

---

## Configuration serveur (`kyber_config.toml`)

```toml
[controller.auth.jwt]
algorithm = "HS256"
key = { plain = "my-secret-key" }

[controller.clipboard]
enabled = true   # Partage presse-papiers (Windows uniquement)

[stream]
codec = "h264"
encoder = "nvenc"   # ou "x264", "qsv", "amf", "vaapi"
bitrate = 10000     # kbps
fps = 60
```
