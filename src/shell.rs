use std::path::Path;
use std::process::Command;

/// A path spelled the way the shell that runs jobs reads it.
///
/// That shell is sh, which on windows is the one git ships: it understands
/// `C:/dir`, while the backslashes of `C:\dir` are escapes to it.
pub fn shell_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// `sh -c <script>`, with the script reaching sh as it was written.
///
/// On windows that takes doing. Windows hands a process one command line rather
/// than a list of arguments, and the sh git ships parses that line itself. Rust
/// wraps an argument in quotes only when it holds a space, while escaping the
/// quotes inside it either way, so a script quoted end to end that holds no
/// space - say `"$KROK_HOOKS_DIR/pre-commit-hooks/existing-pre-commit"` -
/// arrives escaped but unwrapped, and sh dies looking for a closing quote.
/// Quoting it here, always, is what rust already does for the scripts that do
/// hold a space.
pub fn shell_command(shell: &str, script: &str) -> Command {
    let mut command = Command::new(shell);
    command.arg("-c");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.raw_arg(quoted(script));
    }
    #[cfg(not(windows))]
    command.arg(script);

    command
}

/// One argument, quoted the way windows expects to read it back.
#[cfg(windows)]
fn quoted(argument: &str) -> String {
    let mut quoted = String::with_capacity(argument.len() + 2);
    quoted.push('"');

    let mut backslashes = 0;
    for character in argument.chars() {
        match character {
            '\\' => {
                backslashes += 1;
            }
            '"' => {
                // Doubled so they stay backslashes, plus one to escape the quote.
                quoted.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                backslashes = 0;
            }
            _ => {
                quoted.extend(std::iter::repeat_n('\\', backslashes));
                backslashes = 0;
            }
        }
        if character != '\\' {
            quoted.push(character);
        }
    }

    // A run at the very end would escape the closing quote, so it is doubled too.
    quoted.extend(std::iter::repeat_n('\\', backslashes * 2));
    quoted.push('"');
    quoted
}

#[cfg(all(test, windows))]
mod tests {
    use super::quoted;

    #[test]
    fn plain_text_is_only_wrapped() {
        assert_eq!(quoted("cargo test"), r#""cargo test""#);
    }

    // The case that sent krok down this road.
    #[test]
    fn a_script_that_is_quoted_end_to_end_keeps_its_quotes() {
        assert_eq!(
            quoted(r#""$KROK_HOOKS_DIR/hooks/existing""#),
            r#""\"$KROK_HOOKS_DIR/hooks/existing\"""#
        );
    }

    #[test]
    fn backslashes_are_left_alone_unless_they_reach_a_quote() {
        assert_eq!(quoted(r"C:\dir\file"), r#""C:\dir\file""#);
        assert_eq!(quoted(r#"say \"hi\""#), r#""say \\\"hi\\\"""#);
    }

    #[test]
    fn a_trailing_backslash_does_not_escape_the_closing_quote() {
        assert_eq!(quoted(r"cd C:\dir\"), r#""cd C:\dir\\""#);
    }
}
