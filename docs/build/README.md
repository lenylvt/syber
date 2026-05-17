# Système de Build

## Vue d'ensemble

Le système de build de Kyber repose sur des **scripts shell** par plateforme (`build-<platform>.sh`) et une gestion des dépendances tierces via un dossier `contrib/`.

> ⚠️ **Les builds incrémentaux ne sont pas supportés.** Ne jamais relancer le build principal plusieurs fois sans nettoyer.

---

## Structure des dépôts

Chaque dépôt suit la même structure :

```
mon-composant/
├── build-linux.sh       ← Script de build Linux
├── build-windows.sh     ← Script de build Windows (cross depuis Linux)
├── build-macos.sh       ← Script de build macOS
├── external/            ← Git submodules (forks internes de dépendances)
│   ├── vlc/             ← Fork VLC modifié
│   ├── ffmpeg/          ← FFmpeg (si patches nécessaires)
│   └── txproto/         ← txproto
└── contrib/             ← Scripts de build des dépendances tierces
    └── build-*.sh       ← Fetch + patch + build de chaque dépendance
```

---

## Scripts de build

### Options globales

```bash
./build-linux.sh    [options]
./build-windows.sh  [options]
./build-macos.sh    [options]

Options :
  -o <path>   Répertoire de sortie du build
  -s          Skip le rebuild des contribs (dépendances tierces)
```

### Options supplémentaires (dépôts principaux uniquement)

```bash
# Pour kyber-desktop, kyber-web, kyber-android, kyber-ios

  -n          Ne pas mettre à jour le workspace (pas de git pull)
  -p          Packager l'application finale (Windows et macOS)
  -w          (Windows uniquement) Cibler le subsystem WINDOWS au lieu de CONSOLE
```

---

## Structure de sortie des contribs

```
contrib/work/
├── src/                           ← Archives sources téléchargées
│   ├── ffmpeg-7.1.tar.xz
│   ├── opus-1.3.1.tar.gz
│   ├── zlib-1.3.1.tar.gz
│   ├── x264-a8b68eb...zip
│   └── ...
│
├── x86_64-linux-gnu/              ← Build Linux
│   ├── nv-codec-headers/
│   └── x264/
│
└── x86_64-w64-mingw32/            ← Build Windows (cross)
    ├── amf/
    ├── ffmpeg/
    ├── glslang/
    ├── libplacebo/
    ├── lua/
    ├── nv-codec-headers/
    ├── opus/
    ├── SPIRV-Cross/
    ├── SPIRV-Headers/
    ├── SPIRV-Tools/
    ├── txproto/
    ├── vpl/
    ├── x264/
    └── zlib/
```

---

## Dépôts principaux (apps)

Les dépôts principaux sont responsables de :

| Responsabilité | Description |
|---------------|-------------|
| **Fetch des sous-dépôts** | Via git submodules |
| **Génération `.cargo/config.toml`** | Lie les crates Rust partagées |
| **Build des sous-dépôts** | Dans le bon ordre de dépendances |
| **Build de l'application finale** | Compilation + liaison |
| **Packaging** | Structure plate pour distribution (option `-p`) |

### kyber-desktop

Build les clients et serveurs Windows, Linux, macOS.

```bash
# Build Linux
cd kyber-desktop
./build-linux.sh

# Cross-build Windows (depuis Linux)
./build-windows.sh

# Build macOS
./build-macos.sh -p   # avec packaging
```

### kyber-web

Build le client navigateur (WebAssembly).

```bash
cd kyber-web
./build-web.sh
```

---

## Cross-compilation Windows

Le build Windows est effectué **depuis Linux** avec MinGW :

```
Linux machine
    │
    └── MinGW toolchain (x86_64-w64-mingw32-gcc)
            │
            ▼
    Binaires Windows .exe / .dll
```

**Résultat avec `-p`** :
```
kyber-windows-x86_64/     ← Structure plate pour distribution
├── controller.exe
├── service.exe
├── avserver.exe
├── inputserver.exe
├── kymux.exe
├── kyber-client.exe
└── *.dll
```

---

## Workspace Cargo

Les dépôts principaux génèrent un fichier `.cargo/config.toml` pour lier les crates Rust partagées entre sous-dépôts :

```toml
# .cargo/config.toml (généré automatiquement)
[patch.crates-io]
kymux = { path = "../kymux" }
libkynput = { path = "../libkynput" }
kyutil = { path = "../kyutil" }
```

---

## Nettoyage complet

Il n'existe pas de commande `clean` dédiée. Pour tout supprimer :

```bash
rm -rf avserver/contrib/work \
       libclient/contrib/work \
       libkynput/contrib/work \
       rootfs-x86_64-linux-gnu/* \
       rootfs-x86_64-w64-mingw32/*
```

---

## Rebuild d'un composant spécifique

Après le premier build complet, on peut rebuilder un seul composant :

```bash
# Exemple : rebuilder uniquement avserver
cd avserver
./build-linux.sh -s   # -s = skip les contribs
```

> Certains composants nécessitent des workarounds spécifiques (consulter le README du composant concerné).

---

## Dépendances système requises (Linux)

```bash
# Ubuntu / Debian
apt install \
  build-essential \
  cmake \
  ninja-build \
  pkg-config \
  git \
  curl \
  nasm \
  yasm \
  python3 \
  mingw-w64          # pour cross-build Windows
```

---

## Considérations spéciales Windows

### Subsystem CONSOLE vs WINDOWS

| Subsystem | Comportement |
|-----------|-------------|
| `CONSOLE` (défaut) | `controller.exe` ouvre une fenêtre cmd.exe — tous les sous-processus loggent ici. `println!()` fonctionne. |
| `WINDOWS` (option `-w`) | Pas de fenêtre cmd. `println!()` ne fonctionne pas. Pour distribution finale. |
