# env-sync

Syncs `.env` with `.env.example`, in the current folder:

- order, comments and blank lines come from the template;
- values already set in `.env` are kept;
- missing keys get the template's default;
- every other line of `.env` (keys no longer in the template, repeated keys, your own comments) ends up at the
  bottom, under a warning: nothing is lost.

`export KEY=value` lines and quoted values spanning several lines are kept as they are.
A comment later removed from the template is kept at the bottom too, as it can no longer be told apart from your own.

A `KEY=# Command` default is generated on the first sync:

| Command | Value |
| --- | --- |
| `RandomHex` | 32 random bytes in hex (for local secrets) |
| `RandomBase64` | 32 random bytes in base64 |
| `Uuid` | random UUID (version 4) |
| `Cmd <command>` | output of `<command>`, run with `sh -c` (`cmd /C` on Windows); it must print a single line |

A value whose first word after `#` is one of these names is always run as that command (`# Cmd is optional` runs
`is optional`). Any other value, even one starting with `#` (a comment, `#fff`), is copied as is.

`Cmd` runs whatever `.env.example` says: sync only templates you trust.

`.env` is replaced atomically, by renaming a new file over it (a symlinked `.env` stays a link): on Unix it ends up
readable only by its owner (`0600`) and owned by whoever ran `env-sync`, and hard links to it are broken. A `.env`
that cannot be read (permissions, invalid UTF-8) stops the sync and is left untouched.

## Example

`.env.example`:

```bash
# Database
DB_HOST=localhost
DB_PASSWORD=# RandomHex
```

`.env` before:

```bash
DB_HOST=db.local
OLD_FLAG=1
```

`env-sync`, then `.env`:

```bash
# Database
DB_HOST=db.local
DB_PASSWORD=3f9c...e1a7

# ===================================================
# ⚠️  DEPRECATED OR LOCAL VARIABLES
# The following variables are in your .env
# but are no longer tracked in .env.example
# ===================================================
OLD_FLAG=1
```

## Options

```text
-e, --env <file>       env file to sync, read and rewritten [default: .env]
-t, --template <file>  template to sync from [default: .env.example]
-h, --help             print this help
```

One template can feed several env files, or each can have its own:

```bash
env-sync --env .env.prod
env-sync -e .env.test -t .env.test.example
```

## Install

Download and extract the archive for your platform from the
[releases](https://github.com/Berack96/env-sync/releases) (`env-sync-<os>-<arch>.tar.gz`, `.zip` on Windows) and put
`env-sync` on your `PATH`.

## Development

```bash
mise install && mise run setup
mise run lint
mise run test
```
