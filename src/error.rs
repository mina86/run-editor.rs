//! Error definitions.

/// Failure when running user editor.
pub struct Error(pub(super) Inner);

/// Actual error enum.  Create as separate type so that [`Error`] can be made
/// opaque to the user.
pub(super) enum Inner {
    /// Error spawning shell to execute editor.
    CmdError { error: std::io::Error },
    /// Failure returned from the editor command.
    EditorError { editor: std::ffi::OsString, status: std::process::ExitStatus },
    /// IO error with path context.
    PathError { path: std::path::PathBuf, error: std::io::Error },
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Inner::CmdError { error } => {
                write!(fmt, "sh: {}", error)
            }
            Inner::EditorError { editor, status } => {
                debug_assert!(!status.success());
                let editor = std::path::Path::new(editor).display();
                let preposition =
                    if status.code().is_some() { "with" } else { "by" };
                write!(fmt, "{}: terminated {} {}", editor, preposition, status)
            }
            Inner::PathError { path, error } => {
                write!(fmt, "{}: {}", path.display(), error)
            }
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, fmt)
    }
}

impl std::error::Error for Error {}


/// Converts `std::io::Error` into an `Error(Inner::PathError)` adding specified
/// path.
pub(super) trait WithPathContext<P> {
    type Output;
    fn with_path_ctx(self, path: P) -> Self::Output;
}

impl<P: Into<std::path::PathBuf>> WithPathContext<P> for std::io::Error {
    type Output = Error;
    fn with_path_ctx(self, path: P) -> Error {
        Error(Inner::PathError { path: path.into(), error: self })
    }
}

impl<T, P, E: WithPathContext<P>> WithPathContext<P> for Result<T, E> {
    type Output = Result<T, E::Output>;
    fn with_path_ctx(self, path: P) -> Self::Output {
        self.map_err(|error| error.with_path_ctx(path))
    }
}


#[test]
fn test_editor_failure() {
    fn check(
        editor: &str,
        want: Result<(), &'static str>,
        status: std::process::ExitStatus,
    ) {
        let got = if status.success() {
            Ok(())
        } else {
            Err(Error(Inner::EditorError {
                editor: std::ffi::OsString::from(editor),
                status,
            })
            .to_string())
        };
        let got = if let Err(err) = &got { Err(err.as_str()) } else { Ok(()) };
        assert_eq!(want, got)
    }

    fn test(prog: &str, want: Result<(), &'static str>) {
        let got = std::process::Command::new(std::ffi::OsStr::new(prog))
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        check(prog, want, got)
    }

    test("true", Ok(()));
    test("false", Err("false: terminated with exit status: 1"));

    let mut child =
        std::process::Command::new("sleep").arg("100").spawn().unwrap();
    child.kill().unwrap();
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        if let Some(status) = child.try_wait().unwrap() {
            check(
                "sleep",
                Err("sleep: terminated by signal: 9 (SIGKILL)"),
                status,
            );
            return;
        }
    }
    panic!("Child didn’t die after kill");
}
