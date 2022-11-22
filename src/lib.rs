#![doc = include_str!("../README.md")]

use std::ffi::{OsStr, OsString};

mod error;
mod imp;
#[cfg(test)]
mod tests;

/// Interface for allowing users to edit file in CLI applications.
///
/// The simplest usage is to construct the object using [`edit`] function and
/// then invoke the editor with [`file`](`Edit::file`) method.  For example:
///
/// ```no_run
/// let path = "/home/lex/.shellrc";
/// run_editor::edit().file(path).unwrap();
/// ```
///
/// A common usage is to use a temporary file to allow user to edit data which
/// is then kept in-memory.  This can be done manually by creating and
/// maintaining the temporary file or with help of [`buffer`](`Edit::buffer`)
/// method.  For example:
///
/// ```no_run
/// let buf = String::from("\n\n# Enter commit message above.");
/// let buf = run_editor::edit().buffer(Vec::from(buf)).unwrap();
/// ```
#[derive(Default)]
pub struct Edit<'a> {
    /// Name of an additional environment variable to read editor command from.
    editor_variable: Option<&'a OsStr>,

    /// Command to use in preference to those determined by default methods of
    /// getting user preferences.
    editor_command: Option<&'a OsStr>,
}

/// Constructs default [`Edit`] object.
///
/// Example usage (error handling omitted for brevity):
///
/// ```no_run
/// let path = "/home/lex/.shellrc";
/// run_editor::edit().file(path).unwrap();
/// ```
pub const fn edit<'a>() -> Edit<'a> {
    Edit { editor_variable: None, editor_command: None }
}

pub use error::Error;

impl<'a> Edit<'a> {
    /// Executes text editor letting user modify the file.
    ///
    /// Example usage:
    ///
    /// ```no_run
    /// let path = "/home/lex/.shellrc";
    /// if let Err(err) = run_editor::edit().file(path) {
    ///     eprintln!("{err}");
    /// }
    /// ```
    ///
    /// Note that in cases where the value to edit does not exist in a file but
    /// is kept in memory, it may be more convenient to use
    /// [`buffer`](`Self::buffer`) instead.
    pub fn file(&self, path: impl AsRef<std::path::Path>) -> Result<(), Error> {
        match self.editor_unless_nop() {
            Some(editor) => imp::edit_file(editor, path.as_ref()),
            None => Ok(()),
        }
    }

    /// Writes contents of a buffer to temporary file to let user edit it.
    ///
    /// This is a wrapper around [`file`](`Self::file`) which first writes the
    /// data into a temporary file so that user can edit it in a text editor.
    /// On success, contents of the file are then read and returned.  The
    /// temporary file is of courses deleted.
    ///
    /// Example usage (error handling omitted for brevity):
    ///
    /// ```no_run
    /// let buffer = "Some value to edit".to_string();
    /// let result = run_editor::edit()
    ///     .buffer(buffer.into_bytes())
    ///     .map(String::from_utf8);
    /// match result {
    ///     Err(err) => eprintln!("edit failed: {err}"),
    ///     Ok(Err(err)) => eprintln!("invalid UTF-8: {err}"),
    ///     Ok(Ok(message)) => println!("{message}"),
    /// }
    /// ```
    ///
    /// This requires `with_tempfile` Cargo feature to be enabled.  That feature
    /// is enabled by default.
    #[cfg(feature = "with_tempfile")]
    pub fn buffer(&self, buf: Vec<u8>) -> Result<Vec<u8>, Error> {
        match self.editor_unless_nop() {
            Some(editor) => imp::edit_buffer(editor, buf),
            None => Ok(buf),
        }
    }

    /// Copies file from `src` to `dst` letting user edit it.
    ///
    /// This is a bit like first copying the file and then running
    /// [`file`](`Self::file`) on it except that this method performs atomic
    /// write and on failure the destination location is not affected at all.
    /// That is, on failure if the destination file didn’t exist, it won’t be
    /// created and if it existed it won’t be edited.
    ///
    /// Example usage:
    ///
    /// ```no_run
    /// let template_path = "/etc/skel/.bashrc";
    /// let destination_path = "/home/lex/.bashrc";
    /// let res = run_editor::edit().file_copy(template_path, destination_path);
    /// if let Err(err) = res {
    ///     eprintln!("{err}")
    /// }
    /// ```
    ///
    /// This requires `with_tempfile` Cargo feature to be enabled.  That feature
    /// is enabled by default.
    #[cfg(feature = "with_tempfile")]
    pub fn file_copy(
        &self,
        src: impl AsRef<std::path::Path>,
        dst: impl AsRef<std::path::Path>,
    ) -> Result<(), Error> {
        let temp = imp::copy_temp(src.as_ref(), dst.as_ref())?;
        self.file(&*temp)?;
        imp::persist(temp, dst.as_ref())?;
        Ok(())
    }

    /// Returns the editor command to use to let user edit files.
    ///
    /// The resolution of the editor command is goes as follows:
    /// 1. If variable name has been provided via
    ///    [`with_editor_variable`](`Self::with_editor_variable`) and such
    ///    environment variable is set, use its value.
    /// 2. Otherwise, if editor command has been provided via
    ///    [`with`](`Self::with`) method, use that command.
    /// 3. Otherwise, use system-dependent method for determining user
    ///    preferences.  At the moment that means reading `VISUAL` and `EDITOR`
    ///    environment variables.
    /// 4. If that fails as well, use system-dependent default.  At the moment
    ///    that means `"vi"` which should be available on any Unix system.
    ///
    /// Note that returned string is a *command*.  This means that it needs to
    /// be executed through a shell with file to be edited *properly escaped*
    /// and appended to the command.  Or better still, quoted environment
    /// variable name appended and the file path passed as that variable.  For
    /// example, on Unix systems one might execute:
    ///
    /// ```
    /// // File to let user edit:
    /// let path = "/home/lex/.shellrc";
    ///
    /// // Determine command to execute; pass path as variable so we don’t need
    /// // to worry about escaping:
    /// let mut cmd = run_editor::edit().editor();
    /// cmd.push(" \"$TMP_FILE_PATH\"");
    ///
    /// let mut cmd = std::process::Command::new("sh")
    ///     // Don’t forget to pass the variable:
    ///     .env("TMP_FILE_PATH", path)
    ///     .arg("-c")
    ///     .arg(cmd);
    /// ```
    ///
    /// Normally, you just want to use [`file`](`Self::file`) method instead
    /// which handles all that for you.
    pub fn editor(&self) -> OsString {
        self.editor_variable
            .and_then(|var| std::env::var_os(var))
            .or_else(|| self.editor_command.map(OsString::from))
            .or_else(|| std::env::var_os("VISUAL"))
            .or_else(|| std::env::var_os("EDITOR"))
            .unwrap_or_else(|| OsString::from("vi"))
    }

    /// Returns the editor command or `None` if the command is a nop.
    ///
    /// Works like [`get`](`Self::get`) except that it returns `None` if editor
    /// command user specified is `":"` or `"true"`.  If the command is set to
    /// one of those, the file won’t be edited so there’s no need to execute the
    /// editor.
    fn editor_unless_nop(&self) -> Option<OsString> {
        let editor = self.editor();
        if editor == ":" || editor == "true" {
            None
        } else {
            Some(editor)
        }
    }

    /// Specifies environment variable to read user-preferred editor command
    /// from.
    ///
    /// This is useful in conjunction with [`with`](`Self::with`) to allow easy
    /// per-command overriding of the application configuration.  For example,
    /// git supports `core.editor` option but allows that to be overridden with
    /// `GIT_EDITOR` environment variable, e.g.: `GIT_EDITOR=nano git commit`.
    ///
    /// If the variable is set, its value overrides any other methods of
    /// determining the editor command.  See [`editor`](`Self::editor`) for full
    /// description of the resolution priorities.
    #[inline]
    pub fn with_editor_variable(&mut self, variable: &'a OsStr) -> &mut Self {
        self.editor_variable = Some(variable);
        self
    }

    /// Specifies editor command to edit the file with.
    ///
    /// This is useful if an application supports specifying the editor using
    /// methods other than environment variables.  For example, git allows
    /// setting the editor via `core.editor` option which takes priority over
    /// user configuration not specific to git (namely `VISUAL` and `EDITOR`
    /// environment variables).
    ///
    /// If this option is set, it overrides default system-dependent methods of
    /// determining the editor command but is shadowed by value of environment
    /// variable specified via
    /// [`with_editor_variable`](`Self::with_editor_variable`).  See
    /// [`editor`](`Self::editor`) for full description of the resolution
    /// priorities.
    #[inline]
    pub fn with(&mut self, editor_command: &'a OsStr) -> &mut Self {
        self.editor_command = Some(editor_command);
        self
    }
}
