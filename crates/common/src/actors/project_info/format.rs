//! Text formatting for search results and statistics replies.

use crate::actors::{SearchHit, StatsData};

/// Formats search hits as a numbered list; `None` distances are omitted.
pub(crate) fn format_matches(found: &[SearchHit]) -> String {
    if found.is_empty() {
        return "🔍 No matches found.".to_string();
    }
    let mut out = format!("🔍 Found {} match(es):\n", found.len());
    for (i, m) in found.iter().enumerate() {
        let distance = m
            .distance
            .map(|d| format!(" (distance: {d:.4})"))
            .unwrap_or_default();
        out.push_str(&format!(
            "{}. [{}:{}] {}{}\n   {}\n",
            i + 1,
            m.project,
            m.category,
            m.id,
            distance,
            m.content
        ));
    }
    out
}

/// Formats the non-empty text statistics reply for a project scope.
///
/// `all_projects` mirrors the `project == "*"` scope, which adds a
/// per-project breakdown section.
pub(crate) fn format_stats(project: &str, data: &StatsData, all_projects: bool) -> String {
    let mut out = format!(
        "📊 Memory stats for project '{project}': {total} record(s), ~{content_chars} content chars (avg {avg_chars}/record)\n",
        total = data.total,
        content_chars = data.content_chars,
        avg_chars = data.avg_chars
    );
    if all_projects {
        out.push_str("Projects:\n");
        for (name, count) in &data.per_project {
            out.push_str(&format!("- {name}: {count}\n"));
        }
    }
    out.push_str("Categories:\n");
    for (category, count) in &data.per_category {
        out.push_str(&format!("- {category}: {count}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::SearchHit;

    fn hits() -> Vec<SearchHit> {
        vec![
            SearchHit {
                id: "src/auth.rs".to_string(),
                project: "my-proj".to_string(),
                category: "file".to_string(),
                content: "JWT validation helpers".to_string(),
                distance: Some(0.125),
            },
            SearchHit {
                id: "src/main.rs".to_string(),
                project: "my-proj".to_string(),
                category: "file".to_string(),
                content: "stdio wiring and actor spawn".to_string(),
                distance: Some(0.5),
            },
        ]
    }

    #[test]
    fn formats_matches_and_empty_result() {
        assert_eq!(format_matches(&[]), "🔍 No matches found.");

        let text = format_matches(&hits());

        assert!(text.contains("Found 2 match(es)"));
        assert!(text.contains("1. [my-proj:file] src/auth.rs (distance: 0.1250)"));
        assert!(text.contains("JWT validation helpers"));
        assert!(text.contains("2. [my-proj:file] src/main.rs (distance: 0.5000)"));
    }

    #[test]
    fn omits_distance_when_unavailable() {
        let mut hits = hits();
        hits[0].distance = None;
        let text = format_matches(&hits);
        assert!(text.contains("1. [my-proj:file] src/auth.rs\n"));
        assert!(!text.contains("1. [my-proj:file] src/auth.rs (distance"));
    }
}
