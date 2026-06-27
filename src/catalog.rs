use std::path::{Path, PathBuf};

const SCAN_DIRS: &[&str] = &[
    "/Applications",
    "/System/Applications",
    "/System/Applications/Utilities",
];

#[derive(Clone, Debug)]
pub struct AppEntry {
    pub name: String,
    pub path: PathBuf,
}

pub fn scan() -> Vec<AppEntry> {
    let mut dirs: Vec<PathBuf> = SCAN_DIRS.iter().map(PathBuf::from).collect();
    if let Some(home) = std::env::home_dir() {
        dirs.push(home.join("Applications"));
    }

    let mut entries = Vec::new();
    for dir in dirs {
        collect_apps(&dir, 1, &mut entries);
    }

    dedupe(entries)
}

fn dedupe(mut entries: Vec<AppEntry>) -> Vec<AppEntry> {
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

    match plist::Value::from_file(bundle.join("Contents/Info.plist")) {
        Ok(plist::Value::Dictionary(dict)) => name_from_dict(&dict, stem),
        _ => stem,
    }
}

fn name_from_dict(dict: &plist::Dictionary, stem: String) -> String {
    ["CFBundleDisplayName", "CFBundleName"]
        .iter()
        .find_map(|key| dict.get(key).and_then(|v| v.as_string()))
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or(stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, path: &str) -> AppEntry {
        AppEntry {
            name: name.to_owned(),
            path: PathBuf::from(path),
        }
    }

    fn dict(pairs: &[(&str, &str)]) -> plist::Dictionary {
        let mut dict = plist::Dictionary::new();
        for (key, value) in pairs {
            dict.insert((*key).to_owned(), plist::Value::String((*value).to_owned()));
        }
        dict
    }

    #[test]
    fn dedupe_duplicate_paths() {
        let entries = vec![
            entry("Terminal", "/System/Applications/Utilities/Terminal.app"),
            entry("Console", "/System/Applications/Utilities/Console.app"),
            entry("Terminal", "/System/Applications/Utilities/Terminal.app"),
        ];
        let names: Vec<String> = dedupe(entries).into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["Console", "Terminal"]);
    }

    #[test]
    fn dedupe_mixed_case_names() {
        let entries = vec![
            entry("Xcode", "/Applications/Xcode.app"),
            entry("iTerm", "/Applications/iTerm.app"),
        ];
        let names: Vec<String> = dedupe(entries).into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["iTerm", "Xcode"]);
    }

    #[test]
    fn name_from_dict_both_keys() {
        let info = dict(&[
            ("CFBundleName", "Safari"),
            ("CFBundleDisplayName", "Safari Browser"),
        ]);
        assert_eq!(name_from_dict(&info, "stem".to_owned()), "Safari Browser");
    }

    #[test]
    fn name_from_dict_empty_display_name() {
        let info = dict(&[("CFBundleDisplayName", ""), ("CFBundleName", "Safari")]);
        assert_eq!(name_from_dict(&info, "stem".to_owned()), "stem");
    }
}
