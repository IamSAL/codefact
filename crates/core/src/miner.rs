//! Slice selection: which files to analyze next. Priority is changed →
//! uncovered → re-deepen stale, bounded by a limit. `select_slice` is pure;
//! the git/fs helpers are thin `std::process`/walk wrappers.

use std::collections::HashSet;
use std::process::Command;

pub struct SliceInput {
    /// Files changed since last analysis (git modified + untracked).
    pub changed: Vec<String>,
    /// Tracked files never analyzed yet.
    pub uncovered: Vec<String>,
    /// Already-covered files, oldest-analyzed first (to re-deepen).
    pub stale: Vec<String>,
    pub limit: usize,
}

/// Pick up to `limit` files: changed first, then uncovered, then stale.
/// Order-preserving and de-duplicated.
pub fn select_slice(input: &SliceInput) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for f in input
        .changed
        .iter()
        .chain(input.uncovered.iter())
        .chain(input.stale.iter())
    {
        if out.len() >= input.limit {
            break;
        }
        if seen.insert(f.clone()) {
            out.push(f.clone());
        }
    }
    out
}

/// Files git considers modified or untracked (respecting .gitignore).
pub fn git_changed(repo: &str) -> Vec<String> {
    run_git(repo, &["ls-files", "-m", "-o", "--exclude-standard"])
}

/// All tracked files in the repo.
pub fn list_repo_files(repo: &str) -> Vec<String> {
    run_git(repo, &["ls-files"])
}

fn run_git(repo: &str, args: &[&str]) -> Vec<String> {
    let out = Command::new("git").arg("-C").arg(repo).args(args).output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_orders_changed_then_uncovered_then_stale_and_bounds() {
        let input = SliceInput {
            changed: vec!["a".into(), "b".into()],
            uncovered: vec!["c".into(), "a".into()], // 'a' dup should drop
            stale: vec!["d".into()],
            limit: 3,
        };
        assert_eq!(select_slice(&input), vec!["a", "b", "c"]);
    }

    #[test]
    fn slice_dedup_across_buckets() {
        let input = SliceInput {
            changed: vec!["x".into()],
            uncovered: vec!["x".into()],
            stale: vec!["x".into()],
            limit: 10,
        };
        assert_eq!(select_slice(&input), vec!["x"]);
    }
}
