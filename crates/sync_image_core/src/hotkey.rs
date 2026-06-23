use std::{fmt, str::FromStr};

use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: char,
}

impl FromStr for Hotkey {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self> {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut key = None;

        for part in input.split('+') {
            let token = part.trim();
            if token.eq_ignore_ascii_case("ctrl") || token.eq_ignore_ascii_case("control") {
                ctrl = true;
            } else if token.eq_ignore_ascii_case("alt") {
                alt = true;
            } else if token.eq_ignore_ascii_case("shift") {
                shift = true;
            } else if token.chars().count() == 1 {
                let candidate = token.chars().next().expect("checked count");
                if !candidate.is_ascii_alphanumeric() {
                    bail!("hotkey key must be an ASCII letter or digit");
                }
                key = Some(candidate.to_ascii_uppercase());
            } else {
                bail!("unsupported hotkey token '{token}'");
            }
        }

        let Some(key) = key else {
            bail!("hotkey must include a key, for example Ctrl+Alt+U");
        };

        if !(ctrl || alt || shift) {
            bail!("hotkey must include at least one modifier");
        }

        Ok(Self {
            ctrl,
            alt,
            shift,
            key,
        })
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        parts.push(self.key.to_string());
        write!(f, "{}", parts.join("+"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_hotkey() {
        let hotkey: Hotkey = "Ctrl+Alt+U".parse().expect("default hotkey");

        assert!(hotkey.ctrl);
        assert!(hotkey.alt);
        assert!(!hotkey.shift);
        assert_eq!(hotkey.key, 'U');
    }

    #[test]
    fn rejects_missing_modifier() {
        let err = "U".parse::<Hotkey>().expect_err("modifier required");
        assert!(err.to_string().contains("modifier"));
    }
}
