use std::ffi::{OsStr, OsString};

/// Wrapper for setting environment variables and restoring them to old
/// state once the object is dropped.
#[derive(Default)]
struct TestEnv(std::collections::HashMap<&'static OsStr, Option<OsString>>);

impl TestEnv {
    fn set(&mut self, var: &'static str, value: &str) {
        let var = OsStr::new(var);
        self.0.entry(var).or_insert_with(|| std::env::var_os(var));
        std::env::set_var(var, value);
    }

    fn del(&mut self, var: &'static str) {
        let var = OsStr::new(var);
        self.0.entry(var).or_insert_with(|| std::env::var_os(var));
        std::env::remove_var(var);
    }
}

impl std::ops::Drop for TestEnv {
    fn drop(&mut self) {
        for (var, value) in self.0.drain() {
            match value {
                Some(value) => std::env::set_var(var, value),
                None => std::env::remove_var(var),
            }
        }
    }
}

/// Tests whether `Edit::editor` resolves editor command correctly.
#[test]
fn test_get_editor() {
    fn test<'a>(edit: &super::Edit<'a>, want: [&'static str; 4]) {
        let mut env = TestEnv::default();
        env.del("FOO_EDITOR");
        env.del("VISUAL");
        env.del("EDITOR");

        let a = edit.editor().into_string().unwrap();
        env.set("EDITOR", "editor");
        let b = edit.editor().into_string().unwrap();
        env.set("VISUAL", "visual");
        let c = edit.editor().into_string().unwrap();
        env.set("FOO_EDITOR", "foo");
        let d = edit.editor().into_string().unwrap();

        let got = [a.as_str(), b.as_str(), c.as_str(), d.as_str()];
        assert_eq!(want, got);
    }

    let var = OsStr::new("FOO_EDITOR");
    let command = OsStr::new("command");

    test(&super::edit(), ["vi", "editor", "visual", "visual"]);
    test(super::edit().with_editor_variable(var), [
        "vi", "editor", "visual", "foo",
    ]);
    test(super::edit().with(command), [
        "command", "command", "command", "command",
    ]);
    test(super::edit().with_editor_variable(var).with(command), [
        "command", "command", "command", "foo",
    ]);
}


/// Constructs an `Edit` object which changes `foo` on each line in the file
/// with `bar`.
// TODO(mina86): Do something more portable than `sed -i`.  However, keep in
// mind this needs to be something that does atomic write so that we test
// whether we correctly handle that.
fn substitute_foo_bar<'a>() -> super::Edit<'a> {
    let mut edit = super::edit();
    edit.with(OsStr::new("sed -i -e s/foo/bar/"));
    edit
}

#[test]
fn test_edit_file() {
    use std::io::Write;

    let tmpdir = tempfile::tempdir().unwrap();
    let path = tmpdir.path().join("temp-file");

    // Using ":" or "true" as editor should result in a no-op.  It’s
    // actually not observable whether something happened or not but at
    // least we’ll detect if handling of those special commands is broken.
    super::edit().with(OsStr::new(":")).file(&path).unwrap();
    assert!(!path.exists());

    std::fs::File::create(&path).unwrap().write_all(b"foo\n").unwrap();
    substitute_foo_bar().file(&path).unwrap();
    assert_eq!(b"bar\n", std::fs::read(&path).unwrap().as_slice());
}

#[test]
fn test_edit_buffer() {
    let got =
        super::edit().with(OsStr::new(":")).buffer(b"foo\n".to_vec()).unwrap();
    assert_eq!(b"foo\n", got.as_slice());

    let got = substitute_foo_bar().buffer(b"foo\n".to_vec()).unwrap();
    assert_eq!(b"bar\n", got.as_slice());
}

#[test]
fn test_edit_file_copy() {
    use std::io::Write;

    let tmpdir = tempfile::tempdir().unwrap();
    let src = tmpdir.path().join("src");
    let dst = tmpdir.path().join("dst");
    std::fs::File::create(&src).unwrap().write_all(b"foo\n").unwrap();
    std::fs::File::create(&dst).unwrap().write_all(b"oof\n").unwrap();

    // Test destination is not changed on failure.
    let res = super::edit().with(OsStr::new("false")).file_copy(&src, &dst);
    assert_eq!(
        "false: terminated with exit status: 1",
        res.unwrap_err().to_string()
    );
    assert_eq!(b"foo\n", std::fs::read(&src).unwrap().as_slice());
    assert_eq!(b"oof\n", std::fs::read(&dst).unwrap().as_slice());

    substitute_foo_bar().file_copy(&src, &dst).unwrap();
    assert_eq!(b"foo\n", std::fs::read(&src).unwrap().as_slice());
    assert_eq!(b"bar\n", std::fs::read(&dst).unwrap().as_slice());
}
