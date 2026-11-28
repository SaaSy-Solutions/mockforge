//! What the tests that start the real `mockforge serve` share.

use std::path::Path;
use std::process::Command;

/// Take everything of the machine's own from a command's environment: no
/// environment file, no other configuration, and a temporary directory of
/// its own. Windows needs its system root to open a socket, so that is kept:
/// without it the server stopped before it listened. It looks for its
/// temporary directory in `TEMP` and `TMP`, where other systems read
/// `TMPDIR`, so all three name the directory of the test.
pub fn isolated<'a>(command: &'a mut Command, tmpdir: &Path) -> &'a mut Command {
    command.env_clear();
    for name in ["SystemRoot", "WINDIR", "SystemDrive"] {
        if let Ok(value) = std::env::var(name) {
            command.env(name, value);
        }
    }
    command.env("TMPDIR", tmpdir).env("TEMP", tmpdir).env("TMP", tmpdir)
}
