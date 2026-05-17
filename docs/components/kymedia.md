# kymedia — Composants Multimédia

## Rôle

`kymedia` fournit toutes les fonctionnalités liées à la **capture, l'encodage, le décodage, le muxing/démuxing** audio et vidéo utilisées par la solution Kyber.

Il s'appuie principalement sur **FFmpeg** (via txproto) côté serveur et **VLC modifié** côté client.

---

## Côté Serveur : avserver (basé sur txproto)

### txproto

`txproto` est une solution de streaming open source basée sur FFmpeg, intégrée dans Kyber. Elle fournit :
- Un pipeline de capture/encodage **dynamiquement configurable**
- L'abstraction des différents encodeurs hardware et software
- La gestion des filtres vidéo (libavfilter)

### Pipeline de capture vidéo

```
┌─────────────────────────────────────────────────────────────────┐
│                     avserver (serveur)                          │
│                                                                 │
│  ┌──────────────────┐                                          │
│  │  Module capture  │  ← Spécifique à l'OS + API               │
│  │  (OS-spécifique) │    Windows: DXGI Desktop Duplication      │
│  └────────┬─────────┘    Linux: X11/XShm, Pipewire (WL)       │
│           │              macOS: ScreenCaptureKit                │
│           ▼                                                     │
│  ┌──────────────────┐                                          │
│  │  Filtre vidéo    │  ← Optionnel (resize, colorimétrie, etc.) │
│  │  (libavfilter)   │    Ex: scale, format, fps                 │
│  └────────┬─────────┘                                          │
│           │                                                     │
│           ▼                                                     │
│  ┌──────────────────┐                                          │
│  │    Encodeur      │  ← Hardware (NVENC/QSV/AMF/VA-API)       │
│  │  (libavcodec)    │    ou Software (x264, x265, VP9, AV1)    │
│  └────────┬─────────┘                                          │
│           │                                                     │
│           ▼                                                     │
│  ┌──────────────────┐                                          │
│  │  Couche réseau   │  ← Vers kymux via IPC                    │
│  │  (→ kymux IPC)   │                                          │
│  └──────────────────┘                                          │
└─────────────────────────────────────────────────────────────────┘
```

### Topologies CPU/GPU

Kyber gère toutes les combinaisons CPU/GPU :

| Capture | Encodage | Transfert | Performance |
|---------|----------|-----------|-------------|
| GPU (Direct3D) | NVENC (GPU) | Aucun | ⭐⭐⭐ Optimal |
| GPU (Direct3D) | x264 (CPU) | GPU→CPU | ⭐⭐ Bon |
| CPU (XShm) | VA-API (Intel GPU) | CPU→GPU | ⭐⭐ Bon |
| CPU (XShm) | x264 (CPU) | Aucun | ⭐ Basique |

Le pipeline gère automatiquement les **transferts CPU↔GPU** selon la topologie détectée.

### Pipeline audio

```
[API audio OS] ← WASAPI (Win) / PulseAudio (Lin) / CoreAudio (Mac)
     │
     ▼
[Encodeur audio CPU] ← Opus / AAC / MP3
     │
     ▼
[kymux IPC]
```

> L'audio est toujours traité **CPU** : la charge CPU de l'encodage audio est négligeable et le flux audio est construit par le CPU de toute façon.

### Thread model

Chaque étape du pipeline est un **thread indépendant** :

```
Thread Capture ──FIFO──► Thread Filtre ──FIFO──► Thread Encodage ──FIFO──► Thread IPC
```

Cela garantit que chaque composant opère à son rythme optimal sans blocage global.

---

## Côté Client : VLC modifié

### Problème avec le VLC original

VLC est conçu pour la **fluidité** et la **synchronisation A/V**. Pour y parvenir, il introduit :
- Des buffers de jitter (plusieurs secondes de buffer)
- Des points de synchronisation entre streams
- Un horloge de présentation (PTS-based)

**Tout cela est incompatible** avec l'objectif de latence minimale de Kyber.

### Modifications Kyber (`--0latency`)

Le fork de VLC de Kyber inverse cette logique :

| Mécanisme VLC original | Comportement avec `--0latency` |
|----------------------|-------------------------------|
| Buffer de jitter | **Supprimé** |
| Sync A/V | **Supprimée** — chaque stream décode indépendamment |
| Horloge PTS | **Mode push** — pas de wait sur horloge |
| File d'attente decoder | **Vidée** — always display latest frame |
| Sélection décodeur | **Hardware first** (DXVA2, VideoToolbox, VA-API, VDPAU) |

### Stratégie d'affichage

```
                 ┌─────────────────────────────────┐
Paquet NAL reçu ─► Décodeur hardware (DXVA2/VT/VA)  │
                 └──────────────┬──────────────────┘
                                │ Frame décodée
                                ▼ (immédiatement)
                         [Affichage direct]
                         (dernière frame = frame affichée)
```

**Si une frame arrive alors qu'une autre est en cours d'affichage** : on remplace par la nouvelle. Pas de queue, pas d'interpolation.

### Décodeurs hardware supportés

| API | Plateforme | GPU |
|-----|-----------|-----|
| DXVA2 / D3D11VA | Windows | Tout GPU DirectX 9/11+ |
| VideoToolbox | macOS | Apple Silicon + Intel |
| VA-API | Linux | Intel, AMD, NVIDIA |
| VDPAU | Linux | NVIDIA legacy |
| MediaCodec | Android | SoC ARM |

---

## kyaudioreg — Gestion de l'audio basse latence

Module de buffering audio utilisé par VLC et le lecteur audio web.

### Problème résolu

Même avec un pipeline push, la latence réseau varie légèrement (jitter). Sans buffering, cela cause des **glitches audio**. Mais trop de buffering = trop de latence.

### Solution

`kyaudioreg` implémente un buffer adaptatif minimal :
- Absorbe le jitter réseau sans introduire de latence perceptible
- Corrige la **dérive d'horloge** (clock drift) entre serveur et client
- Fonctionne sur desktop et dans le navigateur (WebAssembly)

---

## Codecs supportés

| Codec | Encodage serveur | Décodage client | Notes |
|-------|-----------------|-----------------|-------|
| **H.264** | ✅ x264, NVENC, QSV, AMF, VA-API | ✅ HW/SW | Production |
| **H.265/HEVC** | ✅ NVENC, QSV, AMF, VA-API | ✅ HW/SW | Production |
| **VP9** | ✅ logiciel | ✅ HW/SW | Production |
| **AV1** | ⚠️ Expérimental | ⚠️ Expérimental | Limitations navigateur |
| **Opus** | ✅ | ✅ | Audio recommandé |
| **AAC** | ✅ | ✅ | Audio compatible |
