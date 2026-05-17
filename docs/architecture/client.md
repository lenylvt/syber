# Architecture Client

## Rôle du client

Le client (Guest) a pour mission :

1. **S'authentifier** auprès du serveur
2. **Recevoir** les flux vidéo et audio via QUIC
3. **Décoder et afficher** les frames le plus vite possible (zéro buffer)
4. **Capturer** les entrées locales (clavier, souris, gamepad)
5. **Transmettre** les entrées vers le serveur via QUIC

---

## Composants client

### libclient

Bibliothèque principale côté client. Elle intègre et orchestre :

- `kymux` (transport QUIC)
- VLC modifié (lecture vidéo/audio)
- `libkynput` (capture d'entrées)

C'est le SDK client. Toutes les plateformes (desktop, web, mobile) l'utilisent via leurs bindings respectifs.

### VLC modifié (mode `--0latency`)

VLC a été **profondément modifié** pour l'usage Kyber. Normalement, VLC optimise la fluidité et la synchronisation A/V, ce qui introduit des buffers et donc de la latence. Ces mécanismes ont été retirés ou renversés :

#### Modifications apportées à VLC

| Mécanisme original | Comportement Kyber |
|-------------------|--------------------|
| Buffer de jitter pour fluidité | **Supprimé** |
| Synchronisation A/V (sync clock) | **Supprimée** — chaque stream décode indépendamment |
| File d'attente des frames décodées | **Vidée** — on affiche toujours la dernière frame reçue |
| Horloge interne de présentation | **Modifiée** — mode push, pas pull |
| Décodeur software first | **Hardware decoder prioritaire** pour vitesse maximale |

#### Résultat

```
Frame reçue réseau
    │
    ▼ (immédiatement, sans attendre sync)
[Décodeur hardware (DXVA2 / VideoToolbox / VA-API / etc.)]
    │
    ▼ (dès décodage terminé)
[Affichage — dernière frame disponible]
```

**Paramètre d'activation** : `--0latency`

### libkynput (client)

Côté client, `libkynput` **capture** les événements :

- Clavier (presses / relâchements)
- Souris (mouvements relatifs, absolus, boutons, molette)
- Gamepad (boutons, sticks, rumble)
- Curseur (forme du curseur reçu depuis le serveur → rendu local)
- Clipboard (presse-papiers bidirectionnel)
- Fichiers (transfert)

Ces événements sont transmis à l'`InputRouter` client, qui les route selon le contexte (focus de fenêtre, mode UI, etc.).

### InputRouter (client)

Composant central de routage des entrées :

```
[KeyboardProducer]
[MouseProducer]       ──►  [InputRouter]  ──► [kymux (network consumer)]
[GamepadProducer]                         ──► [UI consumer]
                                          ──► [ShortcutHook] (ex: basculer fullscreen)
[Network (cursor)]    ──►  [InputRouter]  ──► [CursorConsumer]
```

**Comportement selon contexte** :

| Contexte | Comportement |
|----------|-------------|
| Focus sur la fenêtre de stream | Les entrées sont envoyées au serveur |
| Focus perdu | Les entrées ne sont PAS envoyées au serveur |
| UI ouverte | Les entrées vont à l'UI locale, pas au serveur |
| Multi-host (guest) | Routing vers le host sélectionné |

---

## Plateformes client et leurs spécificités

### Desktop (Windows / macOS / Linux)

- Utilise `libclient` via un **binding C** (généré par `cbindgen`)
- VLC modifié compilé nativement
- QUIC natif via `kymux`
- Vérification certificat : TLS standard OS

### Web (Chromium / Firefox)

- Utilise `libclient` compilé en **WebAssembly** via `wasm-bindgen`
- Lecteur vidéo web via `libclient` web audio player + `kyaudioreg`
- Transport via **WebTransport** (au lieu de QUIC natif)
- Vérification certificat : `serverCertificateHashes` (paramètre WebTransport)

> ⚠️ Firefox : support expérimental  
> ⚠️ Webkit (Safari) : non fonctionnel (limitations WebKit)  
> ✅ Chromium/Chrome : pleinement fonctionnel

### Android (bientôt)

- Binding **JNI** (C binding) + API **Kotlin** au-dessus
- QUIC natif

### iOS (bientôt)

- Utilise le binding C directement dans l'app de démonstration
- Couche **Swift** native en développement

---

## Flux d'initialisation client

```
1. Client ouvre connexion HTTPS vers le controller
   └── TLS handshake → vérification certificat serveur

2. POST /session/login  (Authorization: Bearer <jwt> ou Basic <base64>)
   └── Réponse: { session_cookie, websocket_token }

3. Client ouvre connexion WSS (WebSocket sécurisé)
   └── TLS handshake → vérification certificat serveur
   └── Premier message: websocket_token
   └── Controller valide → "Welcome"

4. Client demande démarrage du mux
   POST /start_mux (avec session_cookie)
   └── Réponse: { data_plane_address, data_plane_port,
                  data_plane_token, data_plane_certificate_hash }

5. Client ouvre connexion QUIC (natif) ou WebTransport (web)
   └── Vérification: hash certificat == data_plane_certificate_hash
   └── Premier message: data_plane_token
   └── Controller valide → session data plane établie

6. Streaming démarre
   ├── Flux vidéo → QUIC stream #1 → VLC 0-latency
   ├── Flux audio → QUIC stream #2 → lecteur audio
   └── Entrées clavier/souris → QUIC stream #3 (inverse)
```

---

## Gestion de la latence côté client

### `kyaudioreg`

Module de buffering audio pour :
- Maintenir une **faible latence audio** malgré les variations réseau (jitter)
- Corriger la **dérive audio** (clock drift entre serveur et client)
- Utilisé par VLC et le lecteur audio web

### `kywasmtime`

Implémente `tokio::time` pour les **plateformes WebAssembly** (navigateur), où les APIs de temps standard ne sont pas disponibles.
