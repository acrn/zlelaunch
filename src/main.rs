//! yaml-configured command launcher for zsh
//!
//! Install and add to .zshrc:
//!
//! ```bash
//! ctrl_e_menu() { zle -U "$(read -ek | zlelaunch .ctrl_e.yml)
//! " }
//! zle -N ctrl_e_menu
//! bindkey '^e' ctrl_e_menu
//! ```
//!
//! Press `<ctrl>+e` to bring up the list of commands, press indicated keys to execute.
//!
//! ```bash
//! a cargo test --examples --frozen
//! c cargo clippy --no-deps
//! z vim .ctrl_e.yml
//! ```
//!
//! The configuration file should be a list where each entry is either a string or a map
//! with the keys `command` and optionally `key`
//!
//! ```yaml
//! - cargo test --examples --frozen
//! - key: c
//!   command: cargo clippy --no-deps
//! ```
#![warn(rust_2018_idioms)]

use std::io::{self, Read};
use std::process::{ExitCode, Termination};
use std::{env, fs};
use yaml_rust::{Yaml, YamlLoader};

/// An entry in the launcher menu
struct LauncherEntry<'a> {
    /// The key for executing the command
    character: Option<char>,
    /// A shell command
    command: &'a str,
}

impl<'a> LauncherEntry<'a> {
    fn new(command: &'a str) -> Self {
        LauncherEntry {
            character: None,
            command,
        }
    }
}

/// Display menu on stderr
fn output(entries: &[LauncherEntry<'_>]) {
    // hide cursor
    eprint!("\x1b[?25l");
    // count number of newlines, these will be erased after a key is pressed
    let mut linecount = 0;
    entries.iter().for_each(|e| {
        linecount += 1;
        let mut command = String::new();
        e.command.chars().for_each(|ch| {
            match ch {
                c if c == '\n' => {
                    linecount += 1;
                    // indent each line to match the first one
                    command.push_str("\n    ");
                }
                c => command.push(c),
            }
        });
        eprint!(
            "\n \x1b[33m\x1b[1m{}\x1b[0m {}",
            e.character.unwrap(),
            command,
        )
    });
    let mut buffer = [0u8; 1];
    let k = match io::stdin().read_exact(&mut buffer) {
        Ok(_) => buffer[0] as char,
        Err(_) => panic!("Could not read key from stdin"),
    };
    // erase the menu
    (0..linecount - 1).for_each(|_| {
        eprint!("\x1b[2K\x1b[F");
    });
    // erase last line, move to column 1 and show cursor
    eprint!("\x1b[2K\x1b[G\x1b[?25h");
    entries.iter().for_each(|e| {
        if k == e.character.unwrap() {
            println!("{}", e.command);
        }
    });
}

/// Parse yaml documents
///
/// returns a vec of entries and a vec of assigned keys
fn parse_yaml(docs: &[Yaml]) -> (Vec<LauncherEntry<'_>>, Vec<char>) {
    let mut launcher_entries: Vec<_> = Vec::new();
    let mut reserved_keys: Vec<_> = vec!['z'];

    let key_command = &Yaml::String("command".to_string());
    let key_key = &Yaml::String("key".to_string());

    docs.iter().for_each(|doc| {
        if let Yaml::Array(e) = doc {
            e.iter().enumerate().for_each(|(idx, entry)| match entry {
                Yaml::Hash(hash_entry) => {
                    match hash_entry.get(key_command) {
                        Some(value) => {
                            let command = value.as_str();
                            let mut entry = LauncherEntry::new(command.unwrap());
                            if let Some(key) = hash_entry.get(key_key) {
                                let c = key.as_str().unwrap().chars().next().unwrap();
                                if !reserved_keys.contains(&c) {
                                    entry.character = Some(c);
                                    reserved_keys.push(c);
                                }
                            }
                            launcher_entries.push(entry);
                        }
                        None => {
                            eprintln!(
                                "Missing required key \"command\" at index {idx}, found {entry:?}"
                            )
                        }
                    };
                    // let command = hash_entry[key_command].as_str();
                }
                Yaml::String(string_entry) => {
                    let command = string_entry.as_str();
                    launcher_entries.push(LauncherEntry::new(command));
                }
                _ => panic!("Expected string or mapping at index {idx}, found: {entry:?}"),
            });
        } else {
            panic!("Expected array, found {doc:?}");
        }
    });
    (launcher_entries, reserved_keys)
}

/// Assign keys to each command
fn assign_keys(entries: &mut [LauncherEntry<'_>], reserved_keys: &[char]) {
    let mut chars = "aoeuhtnsidpyfgcrlqjkxbmwvz".chars();
    entries.iter_mut().for_each(|entry| {
        // set key
        let mut c = chars.next().unwrap();
        if entry.character.is_none() {
            while reserved_keys.contains(&c) {
                c = chars.next().unwrap();
            }
            entry.character = Some(c);
        }
    });
}

/// Program exit status
enum Exit<'life> {
    Ok,
    ErrorMessage(&'life str),
}

impl<'life> Termination for Exit<'life> {
    fn report(self) -> ExitCode {
        ExitCode::from(match self {
            Exit::Ok => 0,
            Exit::ErrorMessage(m) => {
                eprintln!("{m}");
                1
            }
        })
    }
}

/// Entrypoint
fn main() -> Exit<'static> {
    let (filename, print0) = {
        let mut filename = None;
        let mut print0 = false;
        env::args().for_each(|arg| {
            if arg == "--print0" {
                print0 = true;
            } else {
                filename = Some(arg);
            }
        });
        if filename.is_none() {
            return Exit::ErrorMessage("missing filename argument");
        }
        (filename.unwrap(), print0)
    };
    let file_result = fs::read_to_string(&filename);
    let yaml = match file_result {
        Ok(y) => YamlLoader::load_from_str(&y).unwrap(),
        Err(_) => {
            eprintln!("Failed to read file, create?");
            Vec::new()
        }
    };
    let (mut entries, keys) = parse_yaml(&yaml);
    let editor = match env::var("EDITOR") {
        Ok(s) => s,
        _ => "vim".to_string(),
    };
    let edit_command = format!("{editor} {filename}");
    entries.push(LauncherEntry {
        character: Some('z'),
        command: &edit_command,
    });
    if print0 {
        // Ignore keys and just print every command null-separated
        entries
            .into_iter()
            .for_each(|entry| print!("{}\0", entry.command));
    } else {
        assign_keys(&mut entries, &keys);
        output(&entries);
    }
    Exit::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = "
- python test.py
- key: a
  command: pytest -s
- key: h
  command: echo hej
";

    #[test]
    fn test_string_entry() {
        let yaml = YamlLoader::load_from_str(YAML).unwrap();
        let (entries, _) = parse_yaml(&yaml);
        // test string entry
        assert_eq!(entries[0].command, "python test.py");
        assert_eq!(entries[0].character, None);
    }

    #[test]
    fn test_hash_entry() {
        let yaml = YamlLoader::load_from_str(YAML).unwrap();
        let (entries, _) = parse_yaml(&yaml);
        // test hash entry
        assert_eq!(entries[1].command, "pytest -s");
        assert_eq!(entries[1].character, Some('a'));
    }

    #[test]
    fn test_reserved_key() {
        let yaml = YamlLoader::load_from_str(YAML).unwrap();
        let (_, keys) = parse_yaml(&yaml);
        assert!(keys.contains(&'a'));
        assert!(!keys.contains(&'b'));
    }

    #[test]
    fn test_reserve_keys() {
        let yaml = YamlLoader::load_from_str(YAML).unwrap();
        let (mut entries, keys) = parse_yaml(&yaml);
        assign_keys(&mut entries, &keys);
        assert_eq!(entries[0].character, Some('o'));
        assert_eq!(entries[1].character, Some('a'));
    }
}
