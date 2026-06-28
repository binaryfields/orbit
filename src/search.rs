use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::catalog::AppEntry;

pub struct Search {
    matcher: Matcher,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
        }
    }
}

impl Search {
    pub fn filter(&mut self, query: &str, apps: &[AppEntry]) -> Vec<usize> {
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
                    .score(Utf32Str::new(&app.name, &mut buf), &mut self.matcher)
                    .map(|score| (score, i))
            })
            .collect();

        hits.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| apps[a.1].name.cmp(&apps[b.1].name))
        });
        hits.into_iter().map(|(_, i)| i).collect()
    }
}
