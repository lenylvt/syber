# SDK Kyber

## Vue d'ensemble

Le SDK Kyber est principalement écrit en **Rust**, avec des **bindings** pour d'autres langages afin de faciliter l'intégration sur toutes les plateformes.

> "Kyber veut être un SDK, car il pourrait y avoir d'autres cas d'usage (drones, streaming audio) auxquels on n'a pas encore pensé. Le SDK sera projeté sur d'autres langages."

---

## Architecture du SDK

```
┌──────────────────────────────────────────────────────────┐
│                    SDK Core (Rust)                       │
│                                                          │
│  ┌──────────────┐  ┌───────────────┐                    │
│  │  libclient   │  │  libkynput    │                    │
│  │  (client     │  │  (entrées     │                    │
│  │   principal) │  │   client)     │                    │
│  └──────┬───────┘  └───────┬───────┘                    │
│         └──────────────────┘                             │
│                    │                                     │
└────────────────────┼─────────────────────────────────────┘
                     │
          ┌──────────┼──────────────────┐
          │          │                  │
          ▼          ▼                  ▼
   ┌─────────────┐ ┌──────────────┐ ┌────────────┐
   │  C Binding  │ │  JS/WASM     │ │  (futur)   │
   │ (cbindgen)  │ │ (wasm-bindgen│ │  Swift     │
   └──────┬──────┘ └──────┬───────┘ │  Kotlin    │
          │               │         └────────────┘
     ┌────┴────┐     ┌────┴─────┐
     │ Windows │     │   Web    │
     │  macOS  │     │ Browser  │
     │  Linux  │     └──────────┘
     └────┬────┘
          │
     ┌────┴────────────────┐
     │  (via C binding)    │
     │  iOS (direct C)     │
     │  Android (JNI+Kotlin│
     └─────────────────────┘
```

---

## Bibliothèques SDK Client

### libclient

La **bibliothèque principale** côté client. Elle intègre :

- `kymux` : transport QUIC/WebTransport
- VLC modifié : lecture vidéo/audio 0-latence
- `libkynput` : capture des entrées

C'est le point d'entrée pour toute intégration cliente.

### libkynput (client)

Gestion des entrées côté client : capture clavier, souris, gamepad, clipboard.

---

## Bindings existants

### Binding C (Desktop — Windows, macOS, Linux)

Généré automatiquement par **[cbindgen](https://github.com/mozilla/cbindgen)** depuis le code Rust.

```c
// Exemple d'API C générée
kyber_client_t* kyber_client_create(const kyber_config_t* config);
void kyber_client_connect(kyber_client_t* client, const char* host, uint16_t port);
void kyber_client_send_input(kyber_client_t* client, const kyber_input_t* input);
void kyber_client_destroy(kyber_client_t* client);
```

**Utilisation** :
- Wrapper direct en C/C++
- Via JNI pour Android
- Via FFI pour tout langage ayant un ABI C

### Binding JavaScript/WebAssembly (Web)

Généré par **[wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/)** depuis le code Rust.

```javascript
// Exemple d'utilisation JS
import init, { KyberClient } from './kyber_wasm.js';

await init();
const client = new KyberClient({
  host: "192.168.1.10",
  port: 8080,
  certHash: "..."
});
await client.connect();
client.sendKeyEvent({ key: "a", pressed: true });
```

---

## Bindings en développement

### Swift (iOS)

- Actuellement : binding C utilisé directement dans l'app de démo iOS
- En cours : couche Swift native pour une meilleure DX (Developer Experience)

```swift
// API Swift future (exemple conceptuel)
let client = KyberClient(config: .init(host: "192.168.1.10"))
try await client.connect()
client.send(input: .keyboard(.init(key: .a, pressed: true)))
```

### Kotlin / JNI (Android)

- Utilise le binding C via JNI (Java Native Interface)
- API finale en Kotlin au-dessus du JNI

```kotlin
// API Kotlin
val client = KyberClient(host = "192.168.1.10", port = 8080)
client.connect()
client.sendInput(KyberInput.Keyboard(key = Key.A, pressed = true))
```

---

## Extensions futures

Le SDK est conçu pour être **extensible à de nouveaux cas d'usage** :

| Cas d'usage | Extension SDK nécessaire |
|------------|--------------------------|
| Contrôle de drones | SDK avec canaux de télémétrie + vidéo |
| Streaming audio seul | SDK audio-only sans vidéo |
| Agents IA visuels | SDK flux vidéo + annotations |
| Applications robotiques | SDK capteurs + actionneurs |

L'architecture modulaire (kymux, kymedia, kynput séparés) permet d'assembler uniquement les briques nécessaires.

---

## Utilisation du SDK pour construire Syber

Syber utilise directement les crates Rust de Kyber :

```toml
# Cargo.toml de Syber
[dependencies]
# Plan de données
kymux = { git = "https://gitlab.com/kyber.stream/core/kymux" }

# Entrées
libkynput = { git = "https://gitlab.com/kyber.stream/core/kynput" }

# Multimédia
kymedia = { git = "https://gitlab.com/kyber.stream/core/kymedia" }

# Utilitaires
kyutil = { git = "https://gitlab.com/kyber.stream/core/kyutil" }
```
