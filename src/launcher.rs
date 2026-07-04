use crate::catalog::AppEntry;
use crate::search::Search;

#[derive(Default)]
pub struct Launcher {
    apps: Vec<AppEntry>,
    search: Search,
    pub query: String,
    results: Vec<usize>,
    selected: usize,
}

impl Launcher {
    pub fn set_apps(&mut self, apps: Vec<AppEntry>) {
        self.apps = apps;
        self.refilter();
    }

    pub fn on_query_edited(&mut self) {
        self.selected = 0;
        self.refilter();
    }

    pub fn reset(&mut self) {
        self.query.clear();
        self.selected = 0;
        self.refilter();
    }

    pub fn select_next(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    pub fn entry(&self, row: usize) -> Option<&AppEntry> {
        self.results.get(row).map(|&i| &self.apps[i])
    }

    fn refilter(&mut self) {
        self.results = self.search.filter(&self.query, &self.apps);
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
    }
}
