use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(&self.0)
    }
}

impl Error {
    pub fn unreadable(path: &Path, problem: impl fmt::Display) -> Error {
        Error(format!("{} could not be read: {problem}", path.display()))
    }

    /// A key a section entry states in a shape klin does not read, named against its file.
    pub fn malformed(file: &Path, section: &str, key: &str, must_be: &str) -> Error {
        Error(format!(
            "{}: a \"{section}\" entry's \"{key}\" must be {must_be}",
            file.display()
        ))
    }
}

/// The error kinds of spec 7.3 the runners tell apart.
#[derive(Clone, Copy)]
pub enum ErrorKind {
    Invocation,
    Configuration,
    Base,
    Git,
    Internal,
}

impl ErrorKind {
    pub fn name(self) -> &'static str {
        match self {
            ErrorKind::Invocation => "invocation",
            ErrorKind::Configuration => "configuration",
            ErrorKind::Base => "base",
            ErrorKind::Git => "git",
            ErrorKind::Internal => "internal",
        }
    }
}

/// One error of spec 7.3: its kind, and what went wrong.
pub struct Fault {
    pub kind: ErrorKind,
    pub error: Error,
}

impl From<Fault> for Error {
    fn from(fault: Fault) -> Error {
        fault.error
    }
}

pub fn fault(kind: ErrorKind) -> impl Fn(Error) -> Fault {
    move |error| Fault { kind, error }
}
