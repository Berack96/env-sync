//! Sincronizza `.env` con `.env.example`: mantiene ordine, commenti e righe vuote del modello,
//! conserva i valori già impostati, genera i default mancanti e sposta in fondo le chiavi non più
//! presenti nel modello.
//!
//! Uso: `env-sync`, nella cartella con `.env.example`.

use std::collections::HashSet;
use std::fs;

const ENV_FILE: &str = ".env";
const EXAMPLE_FILE: &str = ".env.example";

/// Se la riga è un'assegnazione `CHIAVE=valore` (spazi attorno all'uguale tollerati) ritorna (chiave, valore).
fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    let mut chars = key.chars();
    let valid = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    valid.then_some((key, value.trim()))
}

/// Comandi che generano il default di una chiave, indicati in `.env.example` come `CHIAVE=# Comando`.
enum CommandEnvs {
    /// 32 byte casuali in esadecimale.
    RandomHex,
}

impl CommandEnvs {
    fn parse(name: &str) -> Result<Self, String> {
        match name {
            "RandomHex" => Ok(Self::RandomHex),
            _ => Err(format!("comando sconosciuto in {EXAMPLE_FILE}: {name}")),
        }
    }

    fn eval(&self) -> String {
        match self {
            Self::RandomHex => {
                let mut bytes = [0u8; 32];
                getrandom::fill(&mut bytes)
                    .expect("generatore casuale del sistema non disponibile");
                bytes.iter().map(|b| format!("{b:02x}")).collect()
            }
        }
    }
}

/// Valuta il default: `# Comando` esegue il comando, qualsiasi altro valore resta com'è.
fn eval_default(value: &str) -> Result<String, String> {
    match value.strip_prefix('#') {
        Some(name) => CommandEnvs::parse(name.trim()).map(|c| c.eval()),
        None => Ok(value.to_string()),
    }
}

/// Contenuto del nuovo `.env`, dal modello e dal `.env` attuale (vuoto se non c'è).
fn sync(example: &str, current: &str) -> Result<String, String> {
    let mut out: Vec<String> = Vec::new();
    let mut known = HashSet::new();
    for line in example.lines() {
        let Some((key, default)) = parse_assignment(line) else {
            out.push(line.to_string());
            continue;
        };
        known.insert(key);
        let existing = current
            .lines()
            .find(|l| parse_assignment(l).is_some_and(|(k, _)| k == key));
        out.push(match existing {
            Some(l) => l.to_string(),
            None => format!("{key}={}", eval_default(default)?),
        });
    }

    let deprecated: Vec<&str> = current
        .lines()
        .filter(|l| parse_assignment(l).is_some_and(|(k, _)| !known.contains(k)))
        .collect();
    if !deprecated.is_empty() {
        out.push(String::new());
        out.push("# ===================================================".to_string());
        out.push("# ⚠️  VARIABILI DEPRECATE O LOCALI".to_string());
        out.push("# Le seguenti variabili sono presenti nel tuo .env".to_string());
        out.push(format!("# ma non sono più tracciate in {EXAMPLE_FILE}"));
        out.push("# ===================================================".to_string());
        out.extend(deprecated.iter().map(|l| l.to_string()));
    }

    out.push(String::new());
    Ok(out.join("\n"))
}

fn main() -> Result<(), String> {
    if std::env::args().len() > 1 {
        return Err(format!(
            "nessun argomento: sincronizza {ENV_FILE} con {EXAMPLE_FILE} nella cartella corrente"
        ));
    }
    let example = fs::read_to_string(EXAMPLE_FILE)
        .map_err(|e| format!("lettura di {EXAMPLE_FILE} fallita: {e}"))?;
    let current = fs::read_to_string(ENV_FILE).unwrap_or_default();
    fs::write(ENV_FILE, sync(&example, &current)?)
        .map_err(|e| format!("scrittura di {ENV_FILE} fallita: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mantiene_modello_e_valori() {
        let example = "# commento\nA=1\n\nB = 2\n";
        let out = sync(example, "B=x\nA=y\n").unwrap();
        assert_eq!(out, "# commento\nA=y\n\nB=x\n");
    }

    #[test]
    fn default_dei_mancanti() {
        assert_eq!(sync("A=1\n", "").unwrap(), "A=1\n");
    }

    #[test]
    fn deprecate_in_fondo() {
        let out = sync("A=1\n", "A=2\nVECCHIA=3\n").unwrap();
        assert!(out.starts_with("A=2\n\n# ==="));
        assert!(out.ends_with("VECCHIA=3\n"));
    }

    #[test]
    fn comando_random_hex() {
        let out = sync("S=# RandomHex\n", "").unwrap();
        let value = out.trim_end().strip_prefix("S=").unwrap();
        assert_eq!(value.len(), 64);
        assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(sync("S=# RandomHex\n", &out).unwrap(), out);
    }

    #[test]
    fn comando_sconosciuto() {
        assert!(sync("S=# Boh\n", "").is_err());
    }

    #[test]
    fn chiavi_non_valide() {
        assert_eq!(parse_assignment("1A=x"), None);
        assert_eq!(parse_assignment("# A=x"), None);
        assert_eq!(parse_assignment(" A = x "), Some(("A", "x")));
    }
}
