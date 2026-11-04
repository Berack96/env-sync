---
name: commit
description: Makes a git commit in this repository, message included, when a commit is explicitly requested.
---

Commit only when explicitly asked, never on your own initiative. Two or three commands are enough: do not reread the diff of changes you made in this session, you already know what they contain.

1. `git status --short` to see what there is to commit. Read the diff (`git diff -- <file>`) only of files you did not change yourself; if something you did not touch is already staged, report it instead of committing it.
2. If the changes to commit mix logically distinct things, stop and report it: each commit corresponds to a single logical change.
3. `git add <file>...` only with the files touched for this task, by name: never `git add -A` or `git add .`. Never add `.env` or other files with secrets, even if git did not ignore them.
4. `git commit -m "<type>: <subject>"`: message in **English**, following Conventional Commits, lowercase, imperative, no trailing period. A body (second `-m`) only if it is needed to explain the *why* of a choice that is not obvious from the diff, never the *what*.
5. Do not mention the conversation, the task or an AI assistant in the message: it describes the code, not the context it was written in. The exception is the attribution footer, if the platform requires it.
