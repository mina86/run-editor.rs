use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;


fn main() -> ExitCode {
    // Parse arguments
    let mut args = std::env::args_os();
    let arg0 = PathBuf::from(args.next().unwrap());
    let opts = match parse_args(args) {
        Ok(opts) => opts,
        Err(()) => {
            usage(&arg0);
            return ExitCode::from(2);
        }
    };

    // Get the editor
    let mut edit = run_editor::edit();
    if let Some(variable) = &opts.variable {
        edit.with_editor_variable(variable);
    }
    if let Some(editor) = &opts.editor {
        edit.with(editor);
    }

    // Execute action
    let result = match opts.action {
        Action::EditFile(path) => edit.file(PathBuf::from(path)),
        Action::EditMessage(msg) => {
            use std::os::unix::ffi::OsStringExt;
            run_editor::edit().buffer(msg.into_vec()).map(|msg| {
                let msg = OsString::from_vec(msg);
                print!("{}", PathBuf::from(msg).display())
            })
        }
        Action::Copy(src, dst) => {
            run_editor::edit().file_copy(PathBuf::from(src), PathBuf::from(dst))
        }
    };

    // Display result
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}: {}", arg0.display(), err);
            ExitCode::FAILURE
        }
    }
}


struct Opts {
    action: Action,
    variable: Option<OsString>,
    editor: Option<OsString>,
}

enum Action {
    EditFile(OsString),
    EditMessage(OsString),
    Copy(OsString, OsString),
}


fn usage(arg0: &Path) {
    eprint!(
        concat!(
            "usage: {} [ <options> ] <action>...\n",
            "Edits file in user-preferred editor.\n",
            "The tool is demonstration of features of the ‘editor’ crate.\n",
            "\n",
            "<action> is one of:\n",
            "  [ --file ] <path>    -- edit the given file\n",
            "  --echo <message>...  -- edit the message and then print it\n",
            "  --copy <src> <dst>   -- copy file <src> to <dst> but run \
             editor on the file’s\n",
            "                          contents before copying is done; both \
             paths must point\n",
            "                          to files\n",
            "\n",
            "<options> is an optional list of options:\n",
            "  --var <var-name>     -- read the editor from given environment \
             variable\n",
            "  --editor <editor>    -- use given editor command\n"
        ),
        arg0.display()
    )
}


fn parse_args(mut args: std::env::ArgsOs) -> Result<Opts, ()> {
    let (mut variable, mut editor) = (None, None);

    let arg = loop {
        let arg = args.next().ok_or(())?;
        if arg == "--var" {
            variable = Some(args.next().ok_or(())?);
        } else if arg == "--editor" {
            editor = Some(args.next().ok_or(())?);
        } else {
            break arg;
        }
    };

    let action = if arg == "--help" || arg == "-h" {
        return Err(());
    } else if arg == "--file" {
        let path = args.next().ok_or(())?;
        if args.next().is_some() {
            return Err(());
        }
        Action::EditFile(path)
    } else if arg == "--echo" {
        let mut message = OsString::new();
        let mut first = true;
        for arg in args {
            if !first {
                message.push(" ");
            }
            message.push(arg);
            first = false;
        }
        Action::EditMessage(message)
    } else if arg == "--copy" {
        let src_path = args.next().ok_or(())?;
        let dst_path = args.next().ok_or(())?;
        if args.next().is_some() {
            return Err(());
        }
        Action::Copy(src_path, dst_path)
    } else if args.next().is_none() {
        Action::EditFile(arg)
    } else {
        return Err(());
    };

    Ok(Opts { action, variable, editor })
}
