use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AppEntry {
    pub name: String,
    pub path: PathBuf,
}

const SCAN_DIRS: &[&str] = &[
    "/Applications",
    "/System/Applications",
    "/System/Applications/Utilities",
];

pub fn scan() -> Vec<AppEntry> {
    let mut dirs: Vec<PathBuf> = SCAN_DIRS.iter().map(PathBuf::from).collect();
    if let Some(home) = std::env::home_dir() {
        dirs.push(home.join("Applications"));
    }

    let mut entries = Vec::new();
    for dir in dirs {
        collect_apps(&dir, 1, &mut entries);
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.dedup_by(|a, b| a.path == b.path);
    entries.sort_by_key(|a| a.name.to_lowercase());
    entries
}

fn collect_apps(dir: &Path, depth: u32, out: &mut Vec<AppEntry>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "app") {
            out.push(AppEntry {
                name: display_name(&path),
                path,
            });
        } else if depth > 0 {
            collect_apps(&path, depth - 1, out);
        }
    }
}

fn display_name(bundle: &Path) -> String {
    let stem = bundle
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let Ok(info) = plist::Value::from_file(bundle.join("Contents/Info.plist")) else {
        return stem;
    };
    let Some(dict) = info.as_dictionary() else {
        return stem;
    };
    ["CFBundleDisplayName", "CFBundleName"]
        .iter()
        .find_map(|key| dict.get(key).and_then(|v| v.as_string()))
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or(stem)
}
