# kyutil — Utilitaires partagés

## Rôle

`kyutil` regroupe les **composants utilitaires** utilisés par la plupart des autres composants core de Kyber.

---

## Composants principaux

### IPC (Inter-Process Communication)

L'IPC est la brique fondamentale permettant aux différents processus Kyber de se parler localement.

**État actuel** : TCP loopback  
**Évolution prévue** : Unix pipes, mémoire partagée

```rust
// Interface conceptuelle IPC
trait IpcChannel {
    fn send(&self, data: &[u8]) -> Result<()>;
    fn recv(&self) -> Result<Vec<u8>>;
}

// Implémentation actuelle
struct TcpIpcChannel { ... }

// Implémentation future
struct UnixPipeChannel { ... }
struct SharedMemoryChannel { ... }
```

### libkypc — Process spawning

`libkypc` fournit des fonctions pour **spawner des processus** et établir l'IPC entre eux.

Utilisé par le `Controller` pour lancer `avserver`, `kymux`, `inputserver`, etc.

```rust
// Exemple conceptuel
fn spawn_with_ipc(binary: &str, args: &[&str]) -> Result<(Child, IpcChannel)> {
    // Démarre le processus
    // Établit le canal IPC
    // Retourne le handle du processus + le canal
}
```

---

## Utilisation dans les autres composants

| Composant | Utilise kyutil pour |
|-----------|---------------------|
| `kyctl` (Controller) | Spawner avserver, kymux, kynput |
| `kymux` | Recevoir/envoyer via IPC depuis/vers les processus locaux |
| `avserver` | Envoyer les paquets encodés vers kymux via IPC |
| `libkynput` | Recevoir/injecter les entrées via IPC |

---

## Principes de conception

- **Stateless** : les utilitaires ne maintiennent pas d'état global
- **Testable** : chaque utilitaire peut être testé indépendamment
- **Léger** : pas de dépendances lourdes
- **Cross-platform** : compilable sur Windows, macOS, Linux, WebAssembly
