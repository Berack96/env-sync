---
name: documenti
description: Instrada un aggiornamento di documentazione in questo repository verso il file giusto. Usa questa skill quando viene chiesto di aggiornare la documentazione, o quando una modifica appena fatta sembra richiederlo.
---

Prima di scrivere qualunque riga di documentazione in questo repository, stabilisci dove va scritta: scrivere nel posto sbagliato crea duplicazioni che divergono nel tempo.

1. **Informazione "di prodotto"** di un progetto (stack, struttura, API, comandi specifici di quel progetto) → va nel README di quel progetto, non nel file di istruzioni generali dell'agente per il repository.
2. **Informazione "operativa"** per chi lavora nel repo (dove stanno le cose, vincoli, workflow) → può andare nel file di istruzioni generali dell'agente, ma solo se: il progetto ha subito una modifica strutturale grande (entrypoint aggiunti/spostati/rimossi, comandi nuovi o rinominati, una nuova libreria o modulo di primo livello, un cambio nello stack o negli strumenti principali); oppure un'affermazione già scritta in quel file non corrisponde più alla realtà (verificato nel codice, non per sentito dire). Fuori da questi due casi, il file di istruzioni generali non si tocca, nemmeno per un fix o un refactor locale.
3. Prima di aggiungere qualunque sezione, controlla se l'informazione esiste già altrove nel repository. Se esiste, punta lì invece di ripeterla — non creare una seconda copia della stessa informazione.
4. Scrivi lo stato attuale, non la cronologia: mai "prima era così, ora è così", solo "la funzione fa X". Un documento che si contraddice o narra la propria storia è peggio di uno incompleto.
