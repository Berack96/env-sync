# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Language

Code, comments, docstrings, CLI help, commit messages, PR titles and descriptions, and documentation are **in English**. Keep this convention in everything you write.

## How to work in this repo

**Rule number one: do not touch anything that was not asked for.** Everything else in this section follows from it.

- **Closed scope**: change only the files and lines needed to satisfy the request. Do not rewrite working code, rename, reorder imports, reformat lines you are not already changing, add docstrings/comments/type hints to unrelated code, or "tidy up" nearby files. If a file is not needed for the task, leave it as it is.
- **Short comments**: if a comment is really needed (it explains a non-obvious why), 1-2 lines at most. No long comment blocks. No references to issues, PRs, tasks or the history of the change ("issue #NN", "added for...", "it used to be like this, now..."): the comment describes the current state of the code, not how it got there — those references rot as soon as the code changes again and belong in the commit message or the PR, not in the code. No file paths in comments: name the thing (function, type, command), not the file it lives in, which moves.
- **Bare minimum**: of what was asked, implement the smallest version that works; bugs found along the way are reported at the end of the work, not fixed on your own initiative. Before declaring done, self-check the diff with the `minimal-diff` skill.
- **Do not be minimal on**: input validation at trust boundaries, error handling that prevents data loss, security measures, basic accessibility, anything explicitly requested.
- **Autonomy**: proceed without asking for confirmation on code changes; ask only if two readings of the task lead to substantially different work.
- **Git**: do not commit or push unless explicitly asked. The main branch is `main` and work happens on dedicated branches merged via PR. Commits and PR titles follow Conventional Commits: types `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`; scope optional.

### When a new feature is requested

When committed work is explicitly requested (a feature, a fix, or other work structured in several steps — the **Git** bullet above still applies: only if asked), the flow — branch, checklist, draft PR, one commit per step, handling requests that widen the work, pre-ready code review, PR ready for review — is in the `pr` skill; for the format of individual commits see the `commit` skill.

## Project documentation

The [README.md](../README.md) at the root is the source of general knowledge about the repository (structure, setup, workflow), while the product info of each project lives **only** in its README. When you need to understand *what* a project does or *why*, read its README (and the code), not this file.

Projects in the repository:
- **`env-sync`**: Rust CLI that syncs `.env` with `.env.example` in the current folder.

This CLAUDE.md is only an operational map to find your way around: where things are, how to run them, which constraints to respect.

## Maintaining this file

**By default this file is NOT touched**: it is not a changelog. When and how to update it (and when a project's README should be updated instead) is in the `documentation` skill.

## Commands

```bash
mise run setup                    # configure git (rules and hooks from .gitconfig)
mise run lint                     # cargo fmt + clippy
```

**Tests**: `mise run test`. Do not invent test commands beyond these.

## Architecture

A single binary, all in `src/main.rs`, with the unit tests at the bottom of the file; end-to-end tests that run the binary are in `tests/cli.rs`. Release binaries are built by `staging.yml` for Linux, macOS and Windows: the archive names (`env-sync-<os>-<arch>`) let mise's `github:` backend pick the right one.
