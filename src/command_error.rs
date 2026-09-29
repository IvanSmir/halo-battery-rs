//! The error the settings window's commands send to the page: a stable
//! `kind` the script can branch on plus a `message` to show.

use std::fmt;
use std::io;

use serde::Serialize;
use serde::ser::SerializeStruct;

#[derive(Debug)]
pub enum CommandError {
    /// Something the command needs is not there (the tray executable, an update).
    Unavailable(String),
    /// Reading or writing a file or the registry failed.
    Io(io::Error),
    /// The command ran but did not achieve its purpose.
    Failed(String),
}

impl CommandError {
    /// The name the page matches on.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Unavailable(_) => "unavailable",
            Self::Io(_) => "io",
            Self::Failed(_) => "failed",
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(m) | Self::Failed(m) => f.write_str(m),
            Self::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for CommandError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut out = s.serialize_struct("CommandError", 2)?;
        out.serialize_field("kind", self.kind())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}
