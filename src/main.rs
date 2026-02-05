use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::io::{self, BufWriter, Write};
use std::os::windows::fs::MetadataExt;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::exit,
};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

struct Entry {
    name: String,
    size: u64,
    is_dir: bool,
}

fn main() -> io::Result<()> {
    let path = env::current_dir().map(PathBuf::from).unwrap_or_else(|_| {
        eprintln!("{}: {}", "[ERROR]".bold().bright_red(), "Path unreachable.");
        exit(1);
    });
    let mut entries = fetch_entries(&path);
    entries.sort_unstable_by(|a, b| b.size.cmp(&a.size));
    let total_size: u64 = entries.iter().map(|e| e.size).sum();
    let stdout = io::stdout();
    let mut handle = BufWriter::with_capacity(128 * 1024, stdout.lock());
    writeln!(
        handle,
        "{} {}",
        path.display().bold().bright_green(),
        human_size(total_size).bold().bright_cyan()
    )?;
    render_tree(&mut handle, &entries)?;
    handle.flush()?;
    Ok(())
}

fn fetch_entries(path: &Path) -> Vec<Entry> {
    let raw: Vec<_> = fs::read_dir(path)
        .map(|res| res.flatten().collect())
        .unwrap_or_default();

    raw.into_par_iter()
        .map(|entry| {
            let path = entry.path();
            let name = entry.file_name().into_string().unwrap_or_default();
            let ft = entry.file_type().ok();
            let is_dir = ft.map(|f| f.is_dir()).unwrap_or(false);

            let size = if is_dir {
                parallel_dir_size(&path)
            } else {
                entry.metadata().map(|m| m.file_size()).unwrap_or(0)
            };

            Entry { name, size, is_dir }
        })
        .collect()
}

fn parallel_dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    entries
        .flatten()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|entry| {
            let metadata = entry.metadata().ok();
            if metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
                parallel_dir_size(&entry.path())
            } else {
                metadata.map(|m| m.file_size()).unwrap_or(0)
            }
        })
        .sum()
}

fn render_tree<W: Write>(w: &mut W, entries: &[Entry]) -> io::Result<()> {
    let dirs: Vec<_> = entries.iter().filter(|e| e.is_dir).collect();
    let files: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();

    for (i, dir) in dirs.iter().enumerate() {
        let connector = if i == dirs.len() - 1 && files.is_empty() {
            "└──"
        } else {
            "├──"
        };
        writeln!(
            w,
            "{} {} {}",
            connector,
            dir.name.bold().red(),
            human_size(dir.size).bold().bright_cyan()
        )?;
    }

    for (i, file) in files.iter().enumerate() {
        let connector = if i == files.len() - 1 { "└" } else { "├" };
        writeln!(
            w,
            "{} {} {}",
            connector,
            file.name.bold().bright_purple(),
            human_size(file.size).bold().bright_cyan()
        )?;
    }

    Ok(())
}

#[inline(always)]
fn human_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1}{}", size, UNITS[unit])
}
