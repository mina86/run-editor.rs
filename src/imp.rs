//! Implementations of the operations.
//!
//! This is not in `lib.rs` so that `lib.rs` is more readable by consisting of
//! mainly declaration of the types and simple function.

use std::ffi::{OsStr, OsString};

use crate::error;
#[cfg(feature = "with_tempfile")]
use crate::error::WithPathContext;


/// Runs user’s preferred editor on given file; see [`crate::Edit::file`].
pub(super) fn edit_file(
    editor: OsString,
    path: &std::path::Path,
) -> Result<(), error::Error> {
    let command =
        concat_os_str(editor.as_os_str(), OsStr::new(" \"$TMP_file_path\""));
    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .env("TMP_file_path", path)
        .status()
        .map_err(|error| error::Inner::CmdError { error })
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(error::Inner::EditorError { editor, status })
            }
        })
        .map_err(|inner| error::Error(inner).into())
}


/// Runs user’s preferred editor to edit data held in memory; see
/// [`crate::Edit::buffer`].
#[cfg(feature = "with_tempfile")]
pub(super) fn edit_buffer(
    editor: OsString,
    mut buf: Vec<u8>,
) -> Result<Vec<u8>, error::Error> {
    use std::io::{Read, Write};

    let mut temp = new_temp_file(std::env::temp_dir())?;
    temp.as_file_mut().write_all(buf.as_slice()).with_path_ctx(temp.path())?;
    let path = temp.into_temp_path();

    edit_file(editor, &path)?;

    // We need to reopen the file (rather than using file.rewind() because an
    // editor might have replaced the dentry.  This usually happens because
    // editors implement atomic write which makes the file descriptor we have
    // point to now deleted file.

    (|| {
        let mut file = std::fs::File::open(&path)?;
        buf.clear();
        file.read_to_end(&mut buf)
    })()
    .with_path_ctx(&*path)?;

    Ok(buf)
}


/// Creates a new temporary file in a given directory.
#[cfg(feature = "with_tempfile")]
pub(super) fn new_temp_file(
    tempdir: std::path::PathBuf,
) -> Result<tempfile::NamedTempFile, error::Error> {
    tempfile::NamedTempFile::new_in(&tempdir).with_path_ctx(tempdir)
}


/// Copies source file into a temporary file located next to destination.
///
/// Destination path is not touched in any way.  It’s only needed to determine
/// its parent directory to put a temporary file in.
#[cfg(feature = "with_tempfile")]
pub(super) fn copy_temp(
    src: &std::path::Path,
    dst: &std::path::Path,
) -> Result<tempfile::TempPath, error::Error> {
    let mut input = std::fs::File::open(src).with_path_ctx(src)?;

    let dir = match dst.parent() {
        None => {
            let error = if cfg!(unix) {
                const EISDIR: i32 = 21;
                std::io::Error::from_raw_os_error(EISDIR)
            } else {
                std::io::Error::new(std::io::ErrorKind::Other, "is a directory")
            };
            return Err(error.with_path_ctx(dst));
        }
        Some(path) if path == std::path::Path::new("") => {
            std::env::current_dir().with_path_ctx(path)?
        }
        Some(path) => path.to_path_buf(),
    };

    let mut temp = new_temp_file(dir)?;
    // TODO(mina86): One issue here is that on error std::io::copy does not
    // specify whether the failure happened when reading input or writing to
    // output.  This mean that we cannot reliably specify whether issue was with
    // sourec or destination file.  For now give destination file as context.
    std::io::copy(&mut input, temp.as_file_mut()).with_path_ctx(temp.path())?;
    Ok(temp.into_temp_path())
}


/// Persist a temporary file into given destination location.
#[cfg(feature = "with_tempfile")]
pub(super) fn persist(
    path: tempfile::TempPath,
    dst: &std::path::Path,
) -> Result<(), error::Error> {
    path.persist(dst).map_err(|err| {
        error::Error(error::Inner::PathError {
            path: err.path.to_path_buf(),
            error: err.error,
        })
    })
}


/// Concatenates two [`OsStr`]s into a newly allocated [`OsString`].
fn concat_os_str(x: &OsStr, y: &OsStr) -> OsString {
    #[cfg(any(target_family = "unix", target_family = "wasi"))]
    {
        #[cfg(target_family = "unix")]
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        #[cfg(target_family = "wasi")]
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        return OsString::from_vec([x.as_bytes(), y.as_bytes()].concat());
    }
    #[allow(unreachable_code)]
    {
        let mut result = OsString::from(x);
        result.push(y);
        result
    }
}

#[test]
fn test_concat_os_str() {
    let got = concat_os_str(OsStr::new("foo"), OsStr::new("bar"));
    assert_eq!("foobar", got);
}
