# env-sync

Sincronizza `.env` con `.env.example`, nella cartella corrente:

- ordine, commenti e righe vuote vengono dal modello;
- i valori già impostati in `.env` restano;
- le chiavi mancanti prendono il default del modello;
- le chiavi non più nel modello finiscono in fondo, sotto un avviso.

Un default `CHIAVE=# Comando` viene generato al primo sync:

| Comando | Valore |
| --- | --- |
| `RandomHex` | 32 byte casuali in esadecimale (per segreti locali) |

## Uso con mise

```toml
[tools]
"github:Berack96/env-sync" = "0.1.0"

[env]
_.file = ".env"

[tasks.setup]
run = [
    "git config --local include.path ../.gitconfig",
    "env-sync",
]
```

## Sviluppo

```bash
mise install && mise run setup
mise run lint
mise run test
```

Ogni push su `main` crea una pre-release con i binari (Linux x64/arm64, macOS arm64, Windows x64); `Production`, a
mano, la promuove a `vX.Y.Z`.
