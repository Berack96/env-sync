---
name: pr
description: Gestisce il flusso completo di una feature/fix a commit in questo repository, dalla creazione del branch alla pull request pronta per la review. Usala quando viene chiesto esplicitamente un lavoro con commit.
---

Questa skill si applica solo se un lavoro con commit è stato chiesto esplicitamente — non aprire branch, fare commit o PR di propria iniziativa.

1. **Branch**: crea un branch da `origin/main` (o dal branch da cui si parte, se diverso) con un nome che rifletta cosa fa la feature/fix.
2. **Checklist e primo commit**: definisci la checklist degli elementi da implementare, realizza il primo e fanne un commit dedicato, poi pusha il branch — serve almeno un commit di differenza dalla base, GitHub non permette di aprire una PR fra due branch identici. Ogni commit si fa invocando la skill `commit` (Skill tool), mai con un `git commit` diretto.
3. **Apertura PR draft**: apri subito la PR in **draft** dal branch verso il branch di partenza.
   - Titolo: Conventional Commits con scope obbligatorio, breve, in italiano; tipi e scope ammessi sono nella sezione Git di `CLAUDE.md`.
     Il titolo diventa una riga delle note di release, letta da chi non ha seguito il lavoro:
     - dice l'effetto, cioè cosa fa ora il progetto o cosa cambia per chi lo usa, non i pezzi toccati: moduli, file e librerie solo se sono il soggetto della modifica (`feat(ci): note di release più leggibili, con il link alla PR di ogni modifica`, non `feat(ci): emoji, link alle PR e una sola lista in cliff.toml`);
     - un cambiamento solo, niente liste con «e»: se ne servono due, il titolo dice il principale e il resto va nella descrizione, o le PR erano due;
     - al massimo circa 60 caratteri dopo lo scope, senza termini nati durante il lavoro (nomi di opzioni scartate, numeri di passi o di PR).
   - Corpo, in italiano: **Sommario** (2-3 righe, lo stato attuale della PR, non la cronologia di come ci si è arrivati); se il lavoro nasce da una issue (o viene chiesto di collegarla), una riga isolata con la parola chiave di chiusura (es. `Closes #<numero>`) — mai solo una menzione testuale — e solo se il branch di partenza di questa PR è quello di default del repository (altrimenti l'issue non si chiude da sola al merge: dillo esplicitamente in quel caso); **Modifiche**, la checklist completa con il primo elemento già spuntato; **Nota** (opzionale) per cose esplicitamente lasciate fuori scope.
4. **Implementazione**: implementa gli elementi restanti della checklist uno alla volta (anche a metà, se ha senso spezzarli): per ognuno un commit dedicato, con la skill `commit`, e la spunta nella descrizione della PR.
5. **Richieste che ampliano il lavoro**: se durante il lavoro arriva una richiesta di modificare la feature o aggiungere altro — piccola deviazione rispetto a quanto già in corso → implementala nella stessa PR; un refactor vero e proprio o qualcosa di troppo distante dal lavoro in corso → chiedi conferma su come procedere invece di deciderlo da solo. Quando la modifica entra nella PR, aggiorna la descrizione in modo che rifletta lo stato attuale, non il percorso per arrivarci (niente "prima era così, poi abbiamo aggiunto...").
6. **Revisione pre-ready**: prima di segnare la PR come pronta, delega la revisione al sotto-agente `revisore` (Sonnet, effort high), passandogli solo il branch base (e la cartella del repository, se non è quella corrente) — mai in-process: chi ha scritto le modifiche ha già il contesto e rischia di non notare bug, duplicazioni, refusi o problemi di logica che un lettore fresco troverebbe. I findings piccoli correggili nella PR stessa; per quelli grandi chiedi conferma su come procedere.
7. **Ready**: rimuovi dalla descrizione i riferimenti al futuro ("si farà", "verrà fatto"...) e segna la PR come pronta per la review (togli lo stato draft).

Se la piattaforma richiede un footer di attribuzione nel corpo della PR, aggiungilo in fondo.
