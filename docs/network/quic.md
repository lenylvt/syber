# Réseau & QUIC — kymux

## Pourquoi QUIC ?

### Critères de sélection

Pour choisir le protocole de transport, Kyber a évalué les critères suivants :

| Critère | Description |
|---------|-------------|
| **Légèreté** | Ratio données utiles / overhead minimum |
| **Multi-sessions** | Plusieurs canaux de communication simultanés |
| **Sécurité native** | Chiffrement intégré sans couche supplémentaire |
| **Flexibilité de fiabilité** | Streams fiables (TCP-like) ET non fiables (UDP-like) |
| **Compatibilité firewall** | Traversée des firewalls d'entreprise |

### Comparaison des protocoles

| Protocole | Multi-stream | Chiffrement | Faible latence | Firewall friendly |
|-----------|-------------|-------------|----------------|-------------------|
| **QUIC** | ✅ | ✅ TLS 1.3 natif | ✅ | ✅ (HTTP/3) |
| RTP | ✅ | ❌ (SRTP séparé) | ✅ | ⚠️ |
| WebRTC | ✅ | ✅ DTLS | ⚠️ | ✅ |
| TCP+TLS | ❌ (monoligne) | ✅ | ❌ (head-of-line blocking) | ✅ |
| UDP brut | ❌ | ❌ | ✅ | ❌ |

> RTP était le premier choix lors de la phase de faisabilité. QUIC s'est révélé supérieur sur tous les critères.

---

## QUIC — Caractéristiques clés

**QUIC** (RFC 9000) est la base de **HTTP/3**. Il remplace la combinaison TCP + TLS sur une couche UDP.

### Avantages pour Kyber

```
┌────────────────────────────────────────────────┐
│                   QUIC (UDP)                    │
│                                                │
│  Stream 1 (vidéo, fiable)    → pas de HoLB*    │
│  Stream 2 (audio, fiable)    → indépendant     │
│  Stream 3 (entrées, fiable)  → garanti livraison│
│  Stream 4 (unreliable**)     → plus bas delai  │
│                                                │
│  TLS 1.3 intégré → chiffrement obligatoire     │
│  0-RTT reconnect → reprise rapide après perte  │
└────────────────────────────────────────────────┘

* HoLB = Head-of-Line Blocking (absent avec QUIC contrairement à TCP)
** RFC 9221 (QUIC Unreliable Datagrams) — stabilisation en cours
```

### Avantage sur TCP

Avec TCP, si un paquet est perdu, **tout** est bloqué jusqu'à réémission (head-of-line blocking). Avec QUIC, chaque stream est indépendant : un paquet perdu sur le stream vidéo ne bloque pas les entrées clavier.

### Connexion rapide

QUIC réduit considérablement le temps d'établissement de connexion :
- **0-RTT** : reprise de session sans nouveau handshake complet
- Utile lors des reconnexions après coupure réseau

---

## kymux — Architecture

`kymux` est le composant Kyber responsable de **tout ce qui touche au réseau**.

### Rôle

```
Processus avserver (vidéo)  ──┐
Processus kynput (entrées)  ──┤  IPC local  ┌──► kymux client ──► VLC
Processus USB service       ──┤ ────────► kymux ──► libkynput client
                              │  serveur    └──► audio player
                              │
                          QUIC (port 8080)
```

### Fonctionnement

1. Les processus locaux (avserver, kynput, etc.) envoient des paquets bruts à kymux via **IPC**
2. kymux **multiplexe** ces paquets sur une unique connexion QUIC
3. Côté client, kymux **démultiplexe** et redistribue vers les composants appropriés

### Transport IPC

- **Actuellement** : TCP local (loopback)
- **Évolution prévue** : pipes Unix, mémoire partagée (plus sécurisé et performant)

### Protocole de données (kyproto)

Format de paquet kymux pour le transport des InputPackets :

```
┌──────────────────────────────────────────────┐
│ type (u8) │ size (u16) │ payload (size bytes) │
└──────────────────────────────────────────────┘
```

Serialisation/désérialisation :
```rust
// Envoi (pseudocode)
fn send_packet(stream: QuicStream, pkt: InputPacket) {
    let type_: u8 = pkt.get_type() as u8;
    let payload: Bytes = pkt.serialize();
    let size: u16 = payload.len() as u16;
    stream.write_all(&[type_])
          .write_all(&size.to_be_bytes())
          .write_all(&payload);
}

// Réception
fn recv_packet(stream: QuicStream) -> InputPacket {
    let type_ = stream.read_u8();
    let size = stream.read_u16_be();
    let payload = stream.read_exact(size);
    InputPacket::deserialize(type_, payload)
}
```

---

## Streams QUIC dans Kyber

Kyber utilise plusieurs streams QUIC indépendants pour chaque type de donnée :

| Stream | Type | Fiabilité | Contenu |
|--------|------|-----------|---------|
| Stream vidéo | Unidirectionnel (serveur→client) | Fiable | Paquets NAL H264/HEVC/VP9/AV1 |
| Stream audio | Unidirectionnel (serveur→client) | Fiable | Paquets audio encodés |
| Stream entrées | Unidirectionnel (client→serveur) | Fiable | Clavier, souris, gamepad |
| Stream cursor | Unidirectionnel (serveur→client) | Fiable | Forme du curseur (bitmap) |
| Stream clipboard | Bidirectionnel | Fiable | Données presse-papiers |
| Stream USB | Bidirectionnel | Fiable | Paquets usbip |
| Stream contrôle | Bidirectionnel | Fiable | Signaling, heartbeat |

> Les entrées **ne doivent jamais être perdues** (pressions de touches surtout). Le transport fiable (ordre + réémission) est obligatoire.

---

## WebTransport (clients web)

Pour les clients navigateur, Kyber utilise **WebTransport** (W3C), qui est une API haut niveau basée sur QUIC, accessible depuis JavaScript/WASM.

```javascript
// Côté web client
const transport = new WebTransport("https://host:8080/transport", {
  serverCertificateHashes: [{
    algorithm: "sha-256",
    value: certHashFromController
  }]
});
await transport.ready;

const videoStream = await transport.incomingUnidirectionalStreams.getReader();
const inputStream = await transport.createUnidirectionalStream();
```

---

## Configuration réseau

```
Port par défaut : 8080 (TCP + UDP)
Protocoles : HTTPS (TCP), WSS (TCP), QUIC (UDP)

Règles firewall minimales :
  IN  TCP  8080  ← connexions entrantes HTTPS/WSS/QUIC
  OUT UDP  8080  ← QUIC (réponses)
```

> ⚠️ Pour exposition Internet : configurer une authentification robuste (JWT) avant d'ouvrir le port. Kyber ne supporte pas encore de solution de relai P2P.
