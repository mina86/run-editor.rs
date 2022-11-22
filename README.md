Routines for executing user-preferred text editor on file.  This is meant
for CLI applications which need to let user edit a file or in-memory
message.  For example, think of `git commit` command which runs the editor
to let user write commit message or `fc` bash command which lets user edit
shell command to be executed.

The crate takes into account user’s preferences thus the editor is not
hard-coded and can be customised.  The preferences are read in accordance to
Unix custom from VISUAL and EDITOR environment variables.  If those
variables are not set, `vi` is used as the default.

# Example usage

Most basic usage of the crate is to run [`Edit::file`] method on the file to
edit.  For example (error handling omitted for brevity):

```no_run
let path = "/home/lex/.shellrc";
run_editor::edit().file(path).unwrap();
```

If the value to edit is in memory [`Edit`] supports using a temporary file
to let user modify it there.  This is done with [`Edit::buffer`] method.
For example (error handling omitted for brevity):

```no_run
let message = "Some value to edit".to_string();
let buffer = run_editor::edit().buffer(message.into_bytes()).unwrap();
let message = String::from_utf8(buffer).unwrap();
```

# Features

The crate has `with_tempfile` feature which is enabled by default.  It
enables [`Edit::buffer`] and [`Edit::file_copy`] methods.  If those methods
are not necessary, the feature may be disabled and then the crate will not
pull in `tempfile` dependency.
