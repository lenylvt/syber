# kymux — Plan de données

## Rôle

`kymux` est le **composant réseau central** de Kyber. Toute donnée transitant entre le client et le serveur passe par kymux.

> "kymux est le point unique qui gère le réseau : il implémente un protocole adapté à la faible latence, chiffré, capable de traverser les firewalls d'entreprise."

---

## Responsabilités

| Responsabilité | Description |
|---------------|-------------|
| **Transport QUIC** | Gère la connexion QUIC (chiffrement TLS 1.3, multiplexage) |
| **WebTransport** | Variante pour les clients web navigateur |
| **Multiplexage** | Combine vidéo, audio, entrées sur une seule connexion |
| **IPC local** | Interface avec les processus locaux (avserver, kynput, etc.) |
| **Gestion des pertes** | Fiabilité configurable par stream (fiable vs. non-fiable) |
| **Isolation de processus** | Crash du streamer ≠ perte de connexion réseau |

---

## Positionnement dans l'architecture

```
                    SERVEUR
┌───────────────────────────────────────────────────────┐
│                                                       │
│  avserver ──────────────────────────────────────┐    │
│  (paquets vidéo/audio encodés)                  │    │
│                                                 │    │
│  inputserver ───────────────────────────────────┤    │
│  (paquets entrées injectés)                     │    │
│                                                 │    │
│  USB service ───────────────────────────────────┤    │
│  (paquets USB)                                  ▼    │
│                                          ┌──────────┐│
│                                          │  kymux   ││
│                                          │  serveur ││
│                                          └────┬─────┘│
└───────────────────────────────────────────────│──────┘
                                               │
                          QUIC (port 8080 UDP)
                               │
┌──────────────────────────────│──────────────────────────┐
│                         ┌────▼─────┐   CLIENT           │
│                         │  kymux   │                    │
│                         │  client  │                    │
│                         └──┬──┬──┬─┘                    │
│                            │  │  │                      │
│              ┌─────────────┘  │  └──────────────────┐   │
│              ▼                ▼                      ▼   │
│           [VLC]         [audio player]         [kynput]  │
│          vidéo              audio              entrées   │
└──────────────────────────────────────────────────────────┘
```

---

## IPC — Communication inter-processus

### État actuel

Les processus locaux communiquent avec kymux via **TCP loopback** :

```
avserver:127.0.0.1:PORT ──TCP──► kymux
kynput:127.0.0.1:PORT   ──TCP──► kymux
```

### Évolution prévue

TCP local sera remplacé par des mécanismes plus efficaces et sécurisés :
- **Pipes Unix** (POSIX)
- **Mémoire partagée** (shared memory)

---

## Protocoles de transport

### QUIC (clients natifs)

Kyber utilise QUIC (RFC 9000) via une implémentation Rust. Les caractéristiques utilisées :

```
QUIC connection
├── Streams bidirectionnels fiables (comme TCP mais sans HoLB)
├── Streams unidirectionnels fiables
└── Datagrams non fiables (RFC 9221 — expérimental)
     └── Pour les données où la fraîcheur > fiabilité
```

**Initialisation** :
```
1. Client → Init QUIC connection (TLS 1.3 handshake)
2. Client vérifie: hash cert TLS == cert_hash (fourni par /start_mux)
3. Client → Premier message: data_plane_token
4. kymux serveur valide token → session établie
```

### WebTransport (clients web)

WebTransport est une API W3C qui expose QUIC aux navigateurs :

```javascript
new WebTransport(url, {
  serverCertificateHashes: [{
    algorithm: "sha-256",
    value: certHash
  }]
})
```

kymux supporte les deux protocoles de manière transparente pour les composants en amont.

---

## Format de paquet (kyproto)

```
┌───────────┬────────────┬────────────────────────┐
│ type (u8) │ size (u16) │    payload (size bytes) │
└───────────┴────────────┴────────────────────────┘
```

**Types de paquets** :

| Type | Direction | Fiabilité | Description |
|------|-----------|-----------|-------------|
| VideoPacket | S→C | Fiable | Frame vidéo encodée |
| AudioPacket | S→C | Fiable | Frame audio encodée |
| KeyboardPacket | C→S | Fiable | Pression/relâchement touche |
| MouseMovePacket | C→S | (Unreliable possible) | Mouvement souris |
| MouseButtonPacket | C→S | Fiable | Clic souris |
| MouseWheelPacket | C→S | Fiable | Molette |
| GamepadPacket | C→S | Fiable | Entrée gamepad |
| CursorPacket | S→C | Fiable | Forme curseur (bitmap) |
| ClipboardPacket | Bidir | Fiable | Presse-papiers |
| ControlPacket | Bidir | Fiable | Signaling, heartbeat |

---

## Extensibilité

kymux est conçu pour supporter facilement **de nouveaux protocoles** :
- Protocol plugin system : un client peut implémenter un protocole réseau additionnel
- Exemple : intégration WebRTC pour certains cas d'usage navigateur

La responsabilité du protocole réseau est **concentrée en un seul endroit**, permettant à des experts réseau de travailler dessus sans impacter les autres composants.
