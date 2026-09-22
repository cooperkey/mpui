/// Command definitions, syntax hints, and tab-completion for command mode (:).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDef {
    pub name: &'static str,
    pub args_syntax: &'static str,
    pub subcommands: &'static [&'static str],
    pub description: &'static str,
}

pub const COMMANDS: &[CommandDef] = &[
    CommandDef {
        name: "vol",
        args_syntax: "<0-100>",
        subcommands: &[],
        description: "Set volume (0-100)",
    },
    CommandDef {
        name: "seek",
        args_syntax: "<mm:ss | $ | end>",
        subcommands: &["0", "$", "end"],
        description: "Seek to position or end of track",
    },
    CommandDef {
        name: "eq",
        args_syntax: "<preset>",
        subcommands: &["flat", "rock", "bass_boost", "vocal_boost", "treble_boost"],
        description: "Apply equalizer preset",
    },
    CommandDef {
        name: "sort",
        args_syntax: "<artist | album | title | duration | year>",
        subcommands: &["artist", "album", "title", "duration", "year"],
        description: "Sort library tracks",
    },
    CommandDef {
        name: "toggle",
        args_syntax: "<visualizer | equalizer | album_art | metadata | lyrics | queue>",
        subcommands: &[
            "visualizer",
            "equalizer",
            "album_art",
            "metadata",
            "lyrics",
            "queue",
        ],
        description: "Toggle panel display",
    },
    CommandDef {
        name: "layout",
        args_syntax: "<minimal | full | default>",
        subcommands: &["minimal", "full", "default"],
        description: "Switch panel layout mode",
    },
    CommandDef {
        name: "w",
        args_syntax: "[filename.m3u]",
        subcommands: &[],
        description: "Save current queue to playlist file",
    },
    CommandDef {
        name: "e",
        args_syntax: "<path>",
        subcommands: &[],
        description: "Open track or playlist from file path",
    },
    CommandDef {
        name: "rescan",
        args_syntax: "",
        subcommands: &[],
        description: "Rescan library directories",
    },
    CommandDef {
        name: "help",
        args_syntax: "",
        subcommands: &[],
        description: "Open keyboard shortcuts manual",
    },
    CommandDef {
        name: "quit",
        args_syntax: "",
        subcommands: &[],
        description: "Exit player",
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandSuggestion {
    /// Ghost text immediately following cursor.
    pub ghost: String,
    /// Additional hint / candidate list.
    pub hint: String,
    /// Completion target if user presses Tab.
    pub completion: Option<String>,
}

/// Computes ghost text, hints, and tab completion for the given command buffer.
pub fn get_suggestion(buffer: &str) -> CommandSuggestion {
    let trimmed = buffer.trim_start();
    if trimmed.is_empty() {
        return CommandSuggestion {
            ghost: String::new(),
            hint: "  vol, seek, eq, sort, toggle, layout, w, e, rescan, help, quit".to_string(),
            completion: None,
        };
    }

    if let Some((cmd, rest)) = trimmed.split_once(' ') {
        // User has entered a command and is typing arguments
        let rest = rest.trim_start();
        let cmd_def = COMMANDS
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(cmd) || (c.name == "quit" && cmd == "q"));

        if let Some(def) = cmd_def {
            if !def.subcommands.is_empty() {
                let matching: Vec<&&str> = def
                    .subcommands
                    .iter()
                    .filter(|sub| sub.starts_with(rest))
                    .collect();

                if matching.len() == 1 {
                    let matched_sub = *matching[0];
                    let ghost = matched_sub[rest.len()..].to_string();
                    let completion = Some(format!("{} {}", def.name, matched_sub));
                    return CommandSuggestion {
                        ghost,
                        hint: format!(" - {}", def.description),
                        completion,
                    };
                } else if matching.len() > 1 {
                    let lcp = longest_common_prefix(&matching);
                    let ghost = if lcp.len() > rest.len() {
                        lcp[rest.len()..].to_string()
                    } else {
                        String::new()
                    };
                    let hint = format!("  [{}]", matching.iter().map(|s| **s).collect::<Vec<_>>().join(", "));
                    let completion = if lcp.len() > rest.len() {
                        Some(format!("{} {}", def.name, lcp))
                    } else {
                        None
                    };
                    return CommandSuggestion {
                        ghost,
                        hint,
                        completion,
                    };
                } else {
                    return CommandSuggestion {
                        ghost: String::new(),
                        hint: format!("  [{}]", def.subcommands.join(", ")),
                        completion: None,
                    };
                }
            } else {
                let ghost = if rest.is_empty() {
                    def.args_syntax.to_string()
                } else {
                    String::new()
                };
                return CommandSuggestion {
                    ghost,
                    hint: format!(" - {}", def.description),
                    completion: None,
                };
            }
        }

        return CommandSuggestion {
            ghost: String::new(),
            hint: "  (unknown command)".to_string(),
            completion: None,
        };
    }

    // Still typing command name
    let matching: Vec<&CommandDef> = COMMANDS
        .iter()
        .filter(|c| c.name.starts_with(trimmed) || (c.name == "quit" && trimmed == "q"))
        .collect();

    if matching.len() == 1 {
        let def = matching[0];
        let name_suffix = if def.name.starts_with(trimmed) {
            &def.name[trimmed.len()..]
        } else {
            ""
        };
        let ghost = if def.args_syntax.is_empty() {
            name_suffix.to_string()
        } else {
            format!("{name_suffix} {}", def.args_syntax)
        };
        let completion = if def.args_syntax.is_empty() {
            Some(def.name.to_string())
        } else {
            Some(format!("{} ", def.name))
        };
        CommandSuggestion {
            ghost,
            hint: format!(" - {}", def.description),
            completion,
        }
    } else if matching.len() > 1 {
        let names: Vec<&str> = matching.iter().map(|c| c.name).collect();
        let lcp = longest_common_prefix_str(&names);
        let ghost = if lcp.len() > trimmed.len() {
            lcp[trimmed.len()..].to_string()
        } else {
            String::new()
        };
        let hint = format!("  [{}]", names.join(", "));
        let completion = if lcp.len() > trimmed.len() {
            Some(lcp)
        } else {
            None
        };
        CommandSuggestion {
            ghost,
            hint,
            completion,
        }
    } else {
        CommandSuggestion {
            ghost: String::new(),
            hint: "  (unknown command)".to_string(),
            completion: None,
        }
    }
}

fn longest_common_prefix(strs: &[&&str]) -> String {
    if strs.is_empty() {
        return String::new();
    }
    let mut prefix = strs[0].to_string();
    for s in &strs[1..] {
        while !s.starts_with(&prefix) {
            prefix.pop();
            if prefix.is_empty() {
                break;
            }
        }
    }
    prefix
}

fn longest_common_prefix_str(strs: &[&str]) -> String {
    if strs.is_empty() {
        return String::new();
    }
    let mut prefix = strs[0].to_string();
    for s in &strs[1..] {
        while !s.starts_with(&prefix) {
            prefix.pop();
            if prefix.is_empty() {
                break;
            }
        }
    }
    prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_buffer_suggestion() {
        let s = get_suggestion("");
        assert!(s.ghost.is_empty());
        assert!(s.hint.contains("vol"));
        assert_eq!(s.completion, None);
    }

    #[test]
    fn test_single_command_match() {
        let s = get_suggestion("vo");
        assert_eq!(s.ghost, "l <0-100>");
        assert_eq!(s.completion, Some("vol ".to_string()));

        let s_res = get_suggestion("res");
        assert_eq!(s_res.ghost, "can");
        assert_eq!(s_res.completion, Some("rescan".to_string()));
    }

    #[test]
    fn test_multiple_command_matches() {
        let s = get_suggestion("s");
        assert!(s.hint.contains("seek"));
        assert!(s.hint.contains("sort"));
        assert_eq!(s.completion, None);
    }

    #[test]
    fn test_argument_completion() {
        let s = get_suggestion("sort a");
        assert!(s.hint.contains("artist"));
        assert!(s.hint.contains("album"));

        let s_exact = get_suggestion("sort ar");
        assert_eq!(s_exact.ghost, "tist");
        assert_eq!(s_exact.completion, Some("sort artist".to_string()));

        let s_toggle = get_suggestion("toggle eq");
        assert_eq!(s_toggle.ghost, "ualizer");
        assert_eq!(s_toggle.completion, Some("toggle equalizer".to_string()));
    }
}
