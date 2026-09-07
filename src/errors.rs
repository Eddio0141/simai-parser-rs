use std::fmt::Display;

#[derive(Debug)]
pub enum SimaiParseError {
    Nom(String),
    DuplicateField(&'static str),
    DuplicateFieldEntry(u64, &'static str),
}

impl Display for SimaiParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Couldn't parse string as Simai file: {}",
            match self {
                SimaiParseError::Nom(err) => err.to_string(),
                SimaiParseError::DuplicateField(field) => format!("duplicate field `{field}`"),
                SimaiParseError::DuplicateFieldEntry(entry, field) =>
                    format!("duplicate field `{field}` entry number `{entry}`"),
            }
        )
    }
}

impl core::error::Error for SimaiParseError {}
