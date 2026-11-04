---
name: commit
description: Fa un commit git in questo repository, messaggio incluso, quando viene chiesto esplicitamente di fare un commit.
---

Si committa solo se richiesto esplicitamente, mai di propria iniziativa. Bastano due o tre comandi: non rileggere il diff delle modifiche fatte da te in questa sessione, sai già cosa contengono.

1. `git status --short` per vedere cosa c'è da committare. Leggi il diff (`git diff -- <file>`) solo dei file che non hai modificato tu; se in stage c'è già qualcosa che non hai toccato tu, segnalalo invece di committarlo.
2. Se le modifiche da committare mescolano cose logicamente distinte, fermati e segnalalo: ogni commit corrisponde a un solo cambiamento logico.
3. `git add <file>...` solo con i file toccati per questo task, per nome: mai `git add -A` o `git add .`. Mai aggiungere `.env` o altri file con segreti, anche se git non li ignorasse.
4. `git commit -m "<tipo>: <oggetto>"`: messaggio in **italiano**, sul modello Conventional Commits, minuscolo, imperativo, senza punto finale. Un corpo (secondo `-m`) solo se serve spiegare il *perché* di una scelta non ovvia dal diff, mai il *cosa*.
5. Non menzionare la conversazione, il task o un assistente AI nel messaggio: descrive il codice, non il contesto in cui è stato scritto. Fa eccezione il footer di attribuzione, se la piattaforma lo richiede.
