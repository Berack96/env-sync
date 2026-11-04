# env-sync

Syncs `.env` with `.env.example`, in the current folder:

- order, comments and blank lines come from the template;
- values already set in `.env` are kept;
- missing keys get the template's default;
- keys no longer in the template end up at the bottom, under a warning.

A `KEY=# Command` default is generated on the first sync:

| Command | Value |
| --- | --- |
| `RandomHex` | 32 random bytes in hex (for local secrets) |

## Usage with mise

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

## Development

```bash
mise install && mise run setup
mise run lint
mise run test
```

Every push to `main` creates a pre-release with the binaries (Linux x64/arm64, macOS arm64, Windows x64); `Production`,
run manually, promotes it to `vX.Y.Z`.
