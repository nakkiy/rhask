use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::sync::OnceLock;

use crate::task::{ListItemKind, ListMessageLevel, ListOutput};

const RESET: &str = "\x1b[0m";
const FG_CYAN: &str = "\x1b[36m";
const FG_BRIGHT_BLACK: &str = "\x1b[90m";
const FG_WHITE: &str = "\x1b[97m";
const BG_GROUP: &str = "\x1b[48;5;24m";
const FG_GROUP_DESC: &str = "\x1b[97m";
const ERASE_TO_END: &str = "\x1b[K";

pub fn info(message: impl AsRef<str>) {
    write_line(io::stdout(), message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    write_line(io::stderr(), message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    write_line(io::stderr(), message.as_ref());
}

fn write_line(mut target: impl Write, message: &str) {
    let _ = writeln!(target, "{}", message);
}

pub fn print_list(output: &ListOutput) {
    for message in &output.messages {
        match message.level {
            ListMessageLevel::Info => info(&message.text),
            ListMessageLevel::Warn => warn(&message.text),
            ListMessageLevel::Error => error(&message.text),
        }
    }

    let mut width_per_depth: HashMap<usize, usize> = HashMap::new();
    for item in &output.items {
        let name_width = item.name.chars().count();
        width_per_depth
            .entry(item.depth)
            .and_modify(|width| *width = (*width).max(name_width))
            .or_insert(name_width);
    }

    let use_color = colors_enabled();

    for item in &output.items {
        let indent = "  ".repeat(item.depth);
        let name_width = *width_per_depth
            .get(&item.depth)
            .unwrap_or(&item.name.chars().count());
        let padded_name = format!("{:width$}", item.name, width = name_width);
        let symbol = match item.kind {
            ListItemKind::Group => '>',
            ListItemKind::Task => '-',
        };

        let base = format!("{}{} {}", indent, symbol, padded_name);
        let desc_plain = item.description.as_ref().map(|d| format!(" : {}", d));

        if use_color {
            info(format_colored_line(item.kind, &base, desc_plain.as_deref()));
        } else if let Some(desc) = desc_plain {
            info(format!("{}{}", base, desc));
        } else {
            info(base);
        }
    }
}

fn format_colored_line(kind: ListItemKind, base: &str, desc: Option<&str>) -> String {
    match kind {
        ListItemKind::Group => {
            if let Some(desc) = desc {
                format!(
                    "{BG_GROUP}{FG_WHITE}{base}{FG_GROUP_DESC}{desc}{ERASE_TO_END}{RESET}",
                    base = base,
                    desc = desc,
                    ERASE_TO_END = ERASE_TO_END
                )
            } else {
                format!(
                    "{BG_GROUP}{FG_WHITE}{base}{ERASE_TO_END}{RESET}",
                    base = base,
                    ERASE_TO_END = ERASE_TO_END
                )
            }
        }
        ListItemKind::Task => {
            if let Some(desc) = desc {
                format!(
                    "{FG_CYAN}{base}{RESET}{FG_BRIGHT_BLACK}{desc}{RESET}",
                    base = base,
                    desc = desc
                )
            } else {
                format!("{FG_CYAN}{base}{RESET}", base = base)
            }
        }
    }
}

fn colors_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| io::stdout().is_terminal())
}
