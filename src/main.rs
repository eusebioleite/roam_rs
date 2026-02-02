use std::{
    collections::VecDeque,
    fs,
    process::exit,
    env,
    path::{ Path, PathBuf },
    sync::{ Arc, atomic::{ AtomicU64, Ordering }, Mutex },
    thread,
};

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const ORANGE: &str = "\x1b[38;5;208m";
const CYAN: &str = "\x1b[36m";
const DEFAULT: &str = "\x1b[39m";

fn main() {
    let args: Vec<String> = env::args().collect();
    let switch: &str = args.get(1).unwrap_or_else(|| {
        display_help();
        exit(0);
    });
    match switch {
        "-h" => {
            display_help();
        }
        "-d" => {
            let path = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    eprintln!("ERROR: Please provide a path.");
                    exit(1);
                });
            println!(
                "{}{}{} {}{}{}",
                GREEN,
                path.display(),
                RESET,
                CYAN,
                human_size(dir_size(&path)),
                RESET
            );
        }
        "-t" => {
            let path = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    eprintln!("ERROR: Please provide a path.");
                    exit(1);
                });
            println!(
                "{}{}{} {}{}{}",
                GREEN,
                path.display(),
                RESET,
                CYAN,
                human_size(dir_size(&path)),
                RESET
            );
            dir_tree(&path);
        }
        _ => {
            eprintln!("Error: Unknown switch '{}'.", switch);
            display_help();
            exit(0);
        }
    }
}
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
fn dir_size(root: &Path) -> u64 {
    let num_threads = std::thread
        ::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let queue = Arc::new(Mutex::new(VecDeque::from([root.to_path_buf()])));
    let total = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::with_capacity(num_threads);
    for _ in 0..num_threads {
        let queue = Arc::clone(&queue);
        let total = Arc::clone(&total);
        let handle = thread::spawn(move || {
            while
                let Some(path) = ({
                    let mut q = queue.lock().unwrap();
                    q.pop_front()
                })
            {
                if let Ok(entries) = fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        if let Ok(ft) = entry.file_type() {
                            let entry_path = entry.path();
                            if ft.is_file() {
                                if let Ok(meta) = entry.metadata() {
                                    total.fetch_add(meta.len(), Ordering::Relaxed);
                                }
                            } else if ft.is_dir() {
                                let mut q = queue.lock().unwrap();
                                q.push_back(entry_path);
                            }
                        }
                    }
                }
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }
    total.load(Ordering::Relaxed)
}
fn dir_tree(path: &Path) {
    if let Ok(entries) = fs::read_dir(path) {
        let mut dirs: Vec<(String, u64)> = Vec::new();
        let mut files: Vec<(String, u64)> = Vec::new();

        for entry in entries.flatten() {
            let path_entry = entry.path();
            let name = entry.file_name().into_string().unwrap_or_default();

            if path_entry.is_dir() {
                let size = dir_size(&path_entry);
                dirs.push((name, size));
            } else if path_entry.is_file() {
                let size = path_entry
                    .metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);
                files.push((name, size));
            }
        }
        dirs.sort_by(|a, b| b.1.cmp(&a.1));
        files.sort_by(|a, b| b.1.cmp(&a.1));
        for (i, (name, size)) in dirs.iter().enumerate() {
            let connector = if i == dirs.len() - 1 && files.is_empty() { "└──" } else { "├──" };
            println!(
                "{}{} {}{}{} {}{}",
                DEFAULT,
                connector,
                YELLOW,
                name,
                RESET,
                CYAN,
                human_size(*size)
            );
        }
        for (i, (name, size)) in files.iter().enumerate() {
            let connector = if i == files.len() - 1 { "└" } else { "├" };
            println!(
                "{}{} {}{}{} {}{}",
                DEFAULT,
                connector,
                ORANGE,
                name,
                RESET,
                CYAN,
                human_size(*size)
            );
        }
    }
}
fn display_help() {
    println!("{}roam_rs{}", ORANGE, RESET);
    println!();
    println!("Usage:");
    println!("  {}rr -h            {}{}Show this help message", CYAN, RESET, YELLOW);
    println!("  {}rr -d [PATH]     {}{}Show directory size for PATH", CYAN, RESET, YELLOW);
    println!(
        "  {}rr -t [PATH]     {}{}Show directory size tree for PATH and subdirectories",
        CYAN,
        RESET,
        YELLOW
    );
    println!();
}
