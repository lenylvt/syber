# Support des Plateformes

## Matrice de compatibilité

|                    | Client | Serveur |
|--------------------|--------|---------|
| **Windows 10+**    | ✅     | ✅      |
| **macOS (Intel)**  | ✅ (≥10.12 Sierra) | 🔜 Bientôt |
| **macOS (Apple Silicon)** | ✅ (≥11.0 Big Sur) | 🔜 Bientôt |
| **Linux (X11)**    | ✅     | ✅      |
| **Linux (Wayland)**| 🔜 Bientôt | 🔜 Bientôt |
| **Navigateur Web** | ✅ (Chromium) ⚠️ (Firefox) ❌ (Safari/WebKit) | — |
| **Android**        | 🔜 Bientôt | — |
| **iOS**            | 🔜 Bientôt | — |

---

## Windows

### Prérequis client
- Windows 10 ou ultérieur

### Prérequis serveur
- Windows 10 ou ultérieur
- **NVENC** : drivers NVIDIA à jour (recommandé — meilleure performance)
- **QSV** (Intel) : support expérimental
- **AMF** (AMD) : support expérimental

### Particularités Windows

#### Service Windows (`service.exe`)

Pour le contrôle du **Secure Desktop** (écran de connexion, UAC), Kyber nécessite un service Windows :

```
service.exe     → Session 0, LOCAL_SYSTEM
    │ CreateProcessAsUser()
    ▼
controller.exe  → Session interactive
    ├── Capture bureau normal ✅
    ├── Capture logon/UAC ✅
    └── Injection inputs sur Secure Desktop ✅
```

> Un `controller.exe` lancé manuellement peut capturer le bureau normal mais **pas** le Secure Desktop.

#### GPU multiple (iGPU + dGPU)

Sur les systèmes avec plusieurs GPU, Kyber peut sélectionner le mauvais GPU par défaut. Contournement : désactiver l'iGPU dans le BIOS.

#### Écran requis

Windows nécessite un **moniteur physique** connecté au GPU principal pour activer la capture DXGI. Solution virtuelle en cours de développement.

#### Curseur de souris invisible

Sans souris physique branchée, Windows cache le curseur. Solution : activer "Touches souris" dans les paramètres d'accessibilité Windows.

#### Gamepad (serveur)

**ViGEm** doit être installé pour le support gamepad côté serveur.

#### Sous-système Windows (CONSOLE vs WINDOWS)

- `CONSOLE` : `controller.exe` ouvre une fenêtre cmd.exe avec les logs de tous les sous-processus
- `WINDOWS` : pas de fenêtre cmd — `println!()` ne fonctionne pas

---

## macOS

### Client
- Intel (x86) : macOS 10.12 Sierra minimum
- Apple Silicon (M-series) : macOS 11.0 Big Sur minimum

### Serveur
- En cours de développement — publication prochaine

### Particularités macOS
- Cross-compilation supportée : Intel ↔ Apple Silicon
- Capture vidéo : ScreenCaptureKit (API moderne Apple)
- Décodage : VideoToolbox (hardware)

---

## Linux

### État du support
- Testé sur **Debian stable**
- X11/Xorg : pleinement fonctionnel
  - Encodage CPU (x264) ✅
  - Encodage NVIDIA (NVENC) ✅
  - Encodage VA-API (Intel/AMD) ⚠️ Expérimental
- Wayland : publication prochaine

### Build
- Build natif Linux : supporté
- **Cross-build** Linux → Windows : supporté (via MinGW)
  - Le build Windows doit être fait depuis Linux

### Dépendances système (Debian)

```bash
# À consulter dans le README de kyber-desktop pour la liste complète
apt install build-essential cmake ninja-build \
    libx11-dev libxext-dev libxrandr-dev \
    libva-dev libvdpau-dev \
    mingw-w64  # pour cross-build Windows
```

---

## Navigateur Web

### Support

| Navigateur | Statut |
|-----------|--------|
| Chrome / Chromium | ✅ Pleinement fonctionnel |
| Edge (Chromium-based) | ✅ |
| Firefox | ⚠️ Expérimental |
| Safari / WebKit | ❌ Limitations WebKit |

### Technologie

- Client compilé en **WebAssembly** via `wasm-bindgen`
- Transport : **WebTransport** (API W3C basée sur QUIC)
- Audio : Web Audio API + `kyaudioreg`
- Vidéo : décodage via WebCodecs API (Chromium)

### Limitations navigateur

- AV1 : très expérimental (limitations navigateur)
- Clipboard : uniquement Chromium avec permission explicite
- Pas de support serveur (le navigateur ne peut être que client)

---

## Android (bientôt)

- Binding : JNI (C) + API Kotlin
- QUIC natif
- Décodage : MediaCodec (hardware)

## iOS (bientôt)

- Binding C existant (utilisé dans l'app de démo)
- Couche Swift native en développement
- VideoToolbox pour le décodage
