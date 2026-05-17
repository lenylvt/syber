# Authentification Kyber

## Vue d'ensemble

Kyber utilise une approche d'authentification basée sur **TLS** pour sécuriser toutes les connexions, avec une **authentification mutuelle** pour établir la confiance des deux côtés avant toute communication réelle.

### Les trois canaux de communication

| Canal | Protocole | Rôle |
|-------|-----------|------|
| Plan de contrôle (client→serveur) | **HTTPS** | Commandes, authentification |
| Plan de contrôle (serveur→client) | **WSS** (WebSocket sécurisé) | Notifications |
| Plan de données | **QUIC** (natif) / **WebTransport** (web) | Vidéo, audio, entrées |

---

## Établissement d'une connexion sécurisée

### Étape 1 — TLS handshake HTTPS

```
Client                              Controller (serveur)
  │                                       │
  │──── ClientHello ─────────────────────►│
  │◄─── ServerHello + certificat serveur ─│
  │                                       │
  │── Valide certificat serveur ──────────┤
  │   (CA OS ou exception manuelle)       │
  │── Vérifie Common Name / Alt Names ────┤
  │                                       │
  │──── POST /session/login ─────────────►│
  │     Authorization: Bearer <jwt>       │
  │◄─── session_cookie + ws_token ────────│
```

### Étape 2 — Authentification WebSocket (WSS)

```
Client                              Controller
  │                                       │
  │──── TLS handshake WSS ───────────────►│
  │── Valide certificat serveur ──────────│
  │                                       │
  │──── Premier message: ws_token ───────►│
  │                    ◄─── "Welcome" ────│  (si valide)
  │                    ◄─── [fermeture] ──│  (si invalide)
```

### Étape 3 — Authentification plan de données (QUIC)

```
Client                              Mux / kymux (serveur)
  │                                       │
  │  GET /start_mux ─────────────────────►│ (via HTTPS + session_cookie)
  │◄─ { address, port,                    │
  │     data_plane_token,                 │
  │     cert_hash }  ─────────────────────│
  │                                       │
  │──── Init QUIC / WebTransport ────────►│
  │── Vérifie: hash cert == cert_hash ────│
  │                                       │
  │──── Premier message: data_plane_token►│
  │                  ◄─── connexion établie│
```

---

## Backends d'authentification

L'architecture d'auth est **modulaire** : chaque backend implémente le même trait Rust :

```rust
pub struct Session {
    pub username: String,
    pub expiration: Option<OffsetDateTime>,
}

fn authenticate(credentials: &str) -> Result<Session, Error> { ... }
```

### Basic Authentication (développement uniquement)

- Accepte **n'importe quel username sans mot de passe**
- Format : `Authorization: Basic dXNlcm5hbWU6==` (base64 de `username:`)
- ⚠️ **À désactiver impérativement en production**

### JWT Authentication (recommandé)

```toml
[controller.auth.jwt]
algorithm = "HS256"   # ou "RS256"
key = { plain = "my-secret-key" }
```

Format : `Authorization: Bearer <jwt_token>`

**Claims supportés** :
- `exp` : expiration automatique de la session
- `sub` : identifiant du sujet (username)
- `aud` : audience (default: `"kyber"`)

**Algorithmes** : HS256, RS256

**Outil de génération pour le développement** :
```bash
jwt-gen --subject "alice" --expiration "8h" --algorithm HS256 --key-plain "mon-secret"
```

**Cycle de vie des tokens** :
- Les JWT viennent d'une source externe (l'application cliente)
- `/session/renew` : renouvellement manuel avec un nouveau JWT + session cookie valide
- Kyber ne renouvelle pas automatiquement les JWT (responsabilité de l'application)

### HTTPS Authentication

Middleware Actix personnalisé (basé sur `actix-session`). Gestion de sessions avec :
- Expiration configurable par backend
- Cookie de session UUID v4
- Nettoyage automatique des sessions expirées toutes les 30 secondes

---

## Endpoints d'authentification

### `POST /session/login`

**Corps** : Header `Authorization`

**Réponse** :
```json
{
  "uid": "9c5b94b1-35ad-49bb-b118-8e8fc24abf80",
  "websocket": "7vaag596mfrs...k4hqpk61i5bsmclq07kd..."
}
```

- `uid` : identifiant UUID v4 de la session
- `websocket` : token one-time pour l'authentification WebSocket

### `POST /session/renew`

- Nécessite : `session_cookie` valide + nouveau `Authorization` header
- Si le cookie est expiré, recommencer depuis `/session/login`

### `POST /session/logout`

- Invalide la session sur tous les canaux (HTTPS, WSS, QUIC/WebTransport)

---

## Authentification WebSocket

### Pourquoi une approche spéciale ?

Les navigateurs imposent des restrictions sur le partage de cookies lors des WebSocket upgrades cross-domain. Deux approches existent :

| Approche | Sécurité | Implémentation |
|----------|----------|----------------|
| Token dans les query params | ⚠️ Risque (visible dans logs URL) | Simple |
| **Token comme premier message** | ✅ Sécurisé | Choix Kyber |

### Flux WebSocket

```
Client ──► POST /session/login ──► reçoit ws_token (one-time)
Client ──► WebSocket upgrade
Client ──► [premier message] ws_token
Controller valide ws_token
    ├── Valide → "Welcome" + heartbeat loop
    └── Invalide → fermeture connexion
```

---

## Vérification du certificat côté client

### Clients natifs (desktop)

Vérifie :
1. Certificat valide selon la politique CA de l'OS (ou exception manuelle)
2. Common Name / Subject Alt Names correspondent au domaine

Pour le plan de données QUIC : compare le hash du certificat TLS avec celui fourni par `/start_mux`.

### Clients web (WebTransport)

```javascript
const transport = new WebTransport(url, {
  serverCertificateHashes: [{
    algorithm: "sha-256",
    value: cert_hash  // fourni par /start_mux
  }]
});
```

---

## Considérations de sécurité

- Kyber est actuellement conçu pour un **réseau local ou VPN**
- Pas de peer-to-peer QUIC natif → nécessite exposition de port ou VPN pour accès Internet
- Le backend auth est **substituable** : intégration avec OAuth2, LDAP, ou tout système externe
- L'architecture est pensée pour les **service meshes** et les déploiements distribués
