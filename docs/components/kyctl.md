# kyctl — Plan de contrôle

## Rôle

`kyctl` fournit les composants du **plan de contrôle** de Kyber. Il gère :

- L'initialisation de la communication
- L'authentification des clients
- L'orchestration de tous les autres composants côté serveur et client

C'est le **chef d'orchestre** de Kyber.

---

## Architecture interne

### Côté serveur : le `Controller`

Le Controller est le **seul processus permanent** sur le serveur. C'est le point d'entrée du client.

```
Client HTTP(S)
    │
    ▼
┌───────────────────────────────────────┐
│              Controller               │
│                                       │
│  ┌─────────────────────────────────┐  │
│  │     Serveur HTTP/HTTPS (actix)  │  │
│  │  POST /session/login            │  │
│  │  POST /session/renew            │  │
│  │  POST /session/logout           │  │
│  │  POST /start_mux                │  │
│  └────────────────┬────────────────┘  │
│                   │                   │
│  ┌────────────────▼────────────────┐  │
│  │    Middleware d'authentification │  │
│  │    (actix-session custom)        │  │
│  └────────────────┬────────────────┘  │
│                   │                   │
│  ┌────────────────▼────────────────┐  │
│  │        Backend Auth             │  │
│  │  Basic / JWT / HTTPS / OAuth    │  │
│  └────────────────┬────────────────┘  │
│                   │                   │
│  ┌────────────────▼────────────────┐  │
│  │       Process Supervisor        │  │
│  │  Lance, surveille, redémarre :  │  │
│  │  - avserver (streamer)          │  │
│  │  - kymux (mux réseau)           │  │
│  │  - libkynput (input server)     │  │
│  │  - USB service                  │  │
│  └─────────────────────────────────┘  │
└───────────────────────────────────────┘
```

### Côté client

Le plan de contrôle côté client est la partie de `libclient` qui :
- Initie les connexions HTTPS et WSS
- Gère le cycle de vie des sessions (login, renew, logout)
- Déclenche le démarrage du plan de données (QUIC)

---

## API REST du Controller

### Authentification

```
POST /session/login
  Headers: Authorization: Bearer <jwt>  ou  Authorization: Basic <base64>
  Response: { uid: string, websocket: string }

POST /session/renew
  Headers: Authorization: Bearer <nouveau_jwt>
  Cookies: session_cookie
  Response: { uid: string, websocket: string }

POST /session/logout
  Cookies: session_cookie
  Effect: invalide HTTPS + WSS + QUIC pour cette session
```

### Plan de données

```
POST /start_mux
  Cookies: session_cookie
  Response: {
    data_plane_address: string,
    data_plane_port: u16,
    data_plane_token: string,
    data_plane_certificate_hash: string
  }
```

---

## Gestion des sessions

Chaque session est identifiée par un **UUID v4** unique.

Cycle de vie :
```
[login] → session créée (UUID + cookie + ws_token + expiration)
    │
    ├──► [Toutes les N secondes] cleanup des sessions expirées
    │
    ├──► [/session/renew] prolongation de la session
    │
    └──► [/session/logout] invalidation immédiate
```

Le nettoyage automatique tourne toutes les **30 secondes**.

---

## Interface WebSocket (WSS)

Le WebSocket côté client sert aux **notifications serveur→client** :

```
Connexion WSS établie
    │
    ├── Premier message: ws_token → validation
    │
    ├── "Welcome" → connexion confirmée
    │
    └── Loop: heartbeat messages (keepalive)
         └── Si silence trop long : reconnexion automatique
```

---

## Supervision des processus

Le Controller maintient un **supervisor** qui :

1. **Lance** les processus au démarrage ou à la demande client
2. **Surveille** l'état de chaque processus
3. **Redémarre** automatiquement un processus crashé
4. **Configure** chaque processus avec les bonnes autorisations utilisateur

```rust
// Exemple conceptuel de supervision
struct ProcessSupervisor {
    streamer: Option<Child>,
    input_server: Option<Child>,
    mux: Option<Child>,
}

impl ProcessSupervisor {
    fn start_stream_session(&mut self, config: StreamConfig) {
        self.mux = Some(spawn_process("kymux", &config));
        self.streamer = Some(spawn_process("avserver", &config));
        self.input_server = Some(spawn_process("inputserver", &config));
    }

    fn on_process_exit(&mut self, pid: u32) {
        // Redémarrage automatique
    }
}
```

---

## Design flexible pour l'intégration

Le Controller est conçu pour être :
- **Embarqué dans des applications** (ex: Parsec-like pour accès personnel depuis Internet)
- **Intégré dans des service meshes** (architecture DevOps)
- **Extensible** avec n'importe quel backend d'authentification
