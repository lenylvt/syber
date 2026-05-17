# kynput — Gestion des entrées/sorties

## Rôle

`libkynput` (kynput) fournit les composants **client et serveur** responsables de :

- La **capture** de tout type d'événement d'entrée (côté client)
- L'**injection** de ces événements dans l'OS (côté serveur)
- La gestion **clavier, souris, gamepad, clipboard, fichiers, USB**

---

## Types d'entrées (InputType)

### Entrées à faible taille, haute fréquence

Ces entrées peuvent être envoyées plusieurs fois par seconde :

| Type | Description |
|------|-------------|
| `Keyboard` | Pression / relâchement de touche |
| `MouseButton` | Pression / relâchement bouton souris |
| `MouseWheel` | Rotation molette |
| `MouseMove` | Mouvement relatif (delta X, delta Y) |
| `MousePosition` | Position absolue (X, Y) sur l'écran du serveur |
| `Gamepad` | Pression bouton, mouvement stick, rumble |

### Entrées à grande taille, basse fréquence

| Type | Description |
|------|-------------|
| `Cursor` | Forme du curseur (bitmap) — envoyé par le serveur |
| `Clipboard` | Chaîne de caractères à copier/coller |
| `File` | Fichier à transférer |

---

## Architecture — InputPacket

Chaque entrée est encapsulée dans un `InputPacket` générique :

```rust
pub enum InputType {
    Keyboard,
    MouseButton,
    MouseWheel,
    MouseMove,
    MousePosition,
    Gamepad,
    Cursor,
    Clipboard,
    File,
}

pub struct InputPacket {
    // type et données spécifiques à ce type
}

impl InputPacket {
    pub fn get_type(&self) -> InputType;
    pub fn serialize(&self) -> Vec<u8>;
    pub fn deserialize(type_: u8, target: InputTarget, buf: &[u8]) -> Result<Self>;
}

impl InputType {
    pub const fn from_u8(t: u8) -> Option<Self>;
}
```

Le format sur le réseau (kycom/kyproto) :

```
┌───────────┬────────────┬────────────────────────┐
│ type (u8) │ size (u16) │    payload (size bytes) │
└───────────┴────────────┴────────────────────────┘
```

---

## InputPacketHandler

Chaque type d'entrée a son propre handler. Un handler est une implémentation spécifique à l'OS qui peut **produire** et/ou **consommer** des InputPackets.

```
Un seul InputPacketHandler par type d'entrée
    │
    ├── Gère N périphériques physiques (ex: 4 gamepads branchés)
    │     └── Chaque paquet contient un DeviceId
    │
    ├── Thread dédié ou Tokio task
    │
    └── API de notifications/actions :
          GamepadHandler.on_gamepad_plugged()
          GamepadHandler.on_gamepad_unplugged()
          FileHandler.send_file(path)
          FileHandler.on_transfer_progress(percent)
```

---

## InputRouter

Le composant central de routage. Il reçoit **tous les événements** des producteurs et les distribue aux consommateurs appropriés selon la politique de routage.

### Schéma de routage (client)

```
[KeyboardProducer]  ──┐
[MouseProducer]     ──┤──► [InputRouter] ──► [Network (kymux)]
[GamepadProducer]   ──┘           │       ──► [UI locale]
                                  │
[Network (curseur)] ──────────────┤──► [CursorConsumer]
                                  │
                                  └──► [ShortcutHook] (intercept)
```

### Politiques de routage

| Situation | Comportement |
|-----------|-------------|
| Fenêtre de stream en focus | Entrées envoyées au serveur |
| Focus perdu | Entrées NON envoyées au serveur |
| UI locale ouverte | Entrées vont à l'UI, pas au serveur |
| Guest mode (2 hosts) | Routing vers le host sélectionné |
| Shortcut fullscreen | Shortcut consommé localement, non transmis |

### ShortcutHook

Un hook peut être branché sur l'InputRouter pour :
- **Intercepter** certains raccourcis avant transmission
- **Modifier** la politique de routage (ex: basculer fullscreen)
- **Bloquer** certains types d'entrées selon le contexte

### Thread safety

L'API `send` de l'InputRouter est **thread-safe** : les handlers peuvent envoyer depuis n'importe quel thread.

---

## Implémentations par plateforme

Pour chaque `InputType`, il existe **deux implémentations** : une pour le Client (capture), une pour le Serveur (injection).

### Capture — Côté Client

| OS | Mécanisme |
|----|-----------|
| Windows | Raw Input API, XInput (gamepad) |
| macOS | Core Graphics events, Game Controller framework |
| Linux | evdev, uinput, X11 events |
| Web | Browser Keyboard/Mouse/Gamepad APIs |

### Injection — Côté Serveur

| OS | Mécanisme |
|----|-----------|
| Windows | `SendInput()`, `ViGEm` (gamepad virtuel) |
| macOS | Core Graphics `CGEvent` |
| Linux | `uinput`, XTest extension |

> ⚠️ **Windows** : ViGEm doit être installé séparément pour le support gamepad sur le serveur.

---

## USB Device Forwarding

En complément de la capture clavier/souris/gamepad, kynput gère le **forwarding USB** :

```
Client                               Serveur
   │                                    │
[Périphérique USB]                      │
   │                                    │
[Driver USB client]                     │
  (intercepte le device)               │
   │                                    │
   │──── Paquets USB (protocole usbip) ─►│
   │                                    │
                               [Driver USB virtuel]
                               (présente le device
                                à l'OS serveur)
                                        │
                               [OS voit le device
                                comme physique]
```

**Protocole** : basé sur [usbip](https://docs.kernel.org/usb/usbip_protocol.html) (Linux kernel)

**Cas d'usage** : tablettes graphiques, clés USB, périphériques spécialisés

---

## Clipboard

Le partage de presse-papiers est actuellement limité :

| Composant | Support clipboard |
|-----------|------------------|
| Serveur Windows | ✅ |
| Client Windows (natif) | ✅ |
| Client Web (Chromium) | ✅ |
| Client macOS/Linux | ⚠️ Non encore supporté |

**Configuration requise** côté serveur (`kyber_config.toml`) :
```toml
[controller.clipboard]
enabled = true
```

---

## Protocole de normalisation

Pour assurer l'interopérabilité entre OS, les événements sont **normalisés** :
- Chaque touche a un code universel indépendant de l'OS
- Chaque bouton gamepad a un identifiant universel
- Les coordonnées souris sont normalisées à l'espace écran du serveur

Cela permet à un client Windows de contrôler un serveur Linux sans conversion supplémentaire.
