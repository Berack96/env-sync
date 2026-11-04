---
name: codice-minimale
description: Autoverifica di scope e minimalità sul proprio diff, da fare prima di dichiarare concluso un task di modifica al codice in questo repository.
---

Prima di dichiarare concluso un task di modifica al codice in questo repository, rileggi il tuo `git diff` (staged e non) riga per riga.

1. Per ogni riga cambiata, chiediti a quale parte della richiesta corrisponde. Se non sai rispondere, va tolta.
2. **Scope**: nessun file toccato che non serviva al task; nessuna riga riformattata, rinominata, o import riordinato senza che fosse richiesto; niente "sistemato di passaggio" in codice vicino non pertinente.
3. **Minimalità**: niente generalizzazioni, astrazioni, parametri opzionali, gestione di casi limite, logging o configurazioni che nessuno ha chiesto; niente aggiunto "perché tanto prima o poi serve".
4. **Residui**: nessun codice di debug, import inutilizzato, file temporaneo, modifica accidentale.
5. Se trovi un bug non richiesto nel codice che hai toccato per altri motivi, non correggerlo di iniziativa: segnalalo a fine lavoro (file, riga, perché è un bug) e lascia decidere.
6. Esito: o il diff è già minimale, o una lista puntuale di righe/file da togliere prima di chiudere il task.
