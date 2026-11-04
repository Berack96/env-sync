---
name: documentation
description: Routes a documentation update in this repository to the right file. Use this skill when asked to update the documentation, or when a change just made seems to require it.
---

Before writing any line of documentation in this repository, decide where it goes: writing in the wrong place creates duplicates that diverge over time.

1. **"Product" information** about a project (stack, structure, API, commands specific to that project) → goes in that project's README, not in the agent's general instructions file for the repository.
2. **"Operational" information** for whoever works in the repo (where things are, constraints, workflow) → may go in the agent's general instructions file, but only if: the project went through a big structural change (entrypoints added/moved/removed, new or renamed commands, a new top-level library or module, a change in the main stack or tools); or a statement already written in that file no longer matches reality (verified in the code, not by hearsay). Outside these two cases, the general instructions file is not touched, not even for a local fix or refactor.
3. Before adding any section, check whether the information already exists elsewhere in the repository. If it does, point there instead of repeating it — do not create a second copy of the same information.
4. Write the current state, not the history: never "it used to be like this, now it is like that", only "the function does X". A document that contradicts itself or narrates its own history is worse than an incomplete one.
