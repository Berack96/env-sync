---
name: minimal-diff
description: Self-check of scope and minimality on your own diff, to do before declaring a code change task in this repository done.
---

Before declaring a code change task in this repository done, reread your `git diff` (staged and unstaged) line by line.

1. For every changed line, ask yourself which part of the request it corresponds to. If you cannot answer, it must go.
2. **Scope**: no file touched that the task did not need; no line reformatted, renamed, or import reordered without being asked; nothing "fixed in passing" in unrelated nearby code.
3. **Minimality**: no generalizations, abstractions, optional parameters, edge-case handling, logging or configuration nobody asked for; nothing added "because it will be needed sooner or later".
4. **Leftovers**: no debug code, unused import, temporary file, accidental change.
5. If you find an unrequested bug in code you touched for other reasons, do not fix it on your own initiative: report it at the end of the work (file, line, why it is a bug) and let the user decide.
6. Outcome: either the diff is already minimal, or a precise list of lines/files to remove before closing the task.
