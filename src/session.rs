use std::cmp::Reverse;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

use crate::catalog::AppEntry;

#[derive(Default)]
pub struct Session {
    apps: Vec<AppEntry>,
    pub query: String,
    results: Vec<usize>,
    selected: usize,
    matcher: Matcher,
}

impl Session {
    pub fn selected(&self) -> usize {
        self.selected
    }

    // ---- Catalog ----

    pub fn set_apps(&mut self, apps: Vec<AppEntry>) {
        self.apps = apps;
        self.update_results();
    }

    // ---- Query ----

    pub fn query_changed(&mut self) {
        self.selected = 0;
        self.update_results();
    }

    pub fn reset(&mut self) {
        self.query.clear();
        self.query_changed();
    }

    // ---- Results ----

    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    pub fn entry(&self, row: usize) -> Option<&AppEntry> {
        self.results.get(row).map(|&i| &self.apps[i])
    }

    // ---- Selection ----

    pub fn select_next(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    // ---- Shared ----

    fn update_results(&mut self) {
        self.results = Self::rank(&mut self.matcher, &self.query, &self.apps);
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
    }

    fn rank(matcher: &mut Matcher, query: &str, apps: &[AppEntry]) -> Vec<usize> {
        if query.trim().is_empty() {
            return (0..apps.len()).collect();
        }

        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut buf = Vec::new();
        let mut hits: Vec<(u32, usize)> = apps
            .iter()
            .enumerate()
            .filter_map(|(i, app)| {
                pattern
                    .score(Utf32Str::new(&app.name, &mut buf), matcher)
                    .map(|score| (score, i))
            })
            .collect();

        hits.sort_by_key(|&(score, i)| (Reverse(score), &apps[i].name));
        hits.into_iter().map(|(_, i)| i).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn apps(names: &[&str]) -> Vec<AppEntry> {
        names
            .iter()
            .map(|name| AppEntry {
                name: (*name).to_owned(),
                path: PathBuf::from(format!("/Applications/{name}.app")),
            })
            .collect()
    }

    fn rank(query: &str, apps: &[AppEntry]) -> Vec<usize> {
        Session::rank(&mut Matcher::default(), query, apps)
    }

    #[test]
    fn rank_blank_query() {
        let apps = apps(&["Books", "iTerm", "Xcode"]);
        assert_eq!(rank("   ", &apps), vec![0, 1, 2]);
    }

    #[test]
    fn rank_differing_scores() {
        let apps = apps(&["Chess", "System Settings"]);
        assert_eq!(rank("ss", &apps), vec![1, 0]);
    }

    #[test]
    fn rank_tied_scores() {
        let apps = apps(&["Sonoma", "Sierra"]);
        assert_eq!(rank("s", &apps), vec![1, 0]);
    }

    #[test]
    fn rank_unmatched_entries() {
        let apps = apps(&["Safari", "Notes", "Mail"]);
        assert_eq!(rank("saf", &apps), vec![0]);
    }
}
