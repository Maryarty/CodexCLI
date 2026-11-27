//! Best-effort branch and PR metadata over the selected app-server workspace executor.
//! The git and GitHub lookups follow `codex-rs/tui/src/branch_summary.rs`.

use std::path::Path;

use codex_tui_core::screens::chat::facts::{BranchChanges, GitSummary, PullRequest};
use codex_tui_core::screens::transcript::diff::{Call, Limits};
use serde::Deserialize;

use super::shell::WorkspaceExec;

/// The two fields are independent. A denied command or an unavailable tool can leave either
/// empty while the other still supplies a status item.
pub async fn read(exec: &dyn WorkspaceExec, cwd: &Path) -> GitSummary {
    let (pull_request, changes) =
        tokio::join!(open_pull_request(exec, cwd), branch_changes(exec, cwd));
    GitSummary {
        pull_request,
        changes,
    }
}

async fn branch_changes(exec: &dyn WorkspaceExec, cwd: &Path) -> Option<BranchChanges> {
    git(exec, cwd, &["rev-parse", "--git-dir"]).await?;
    let reference = default_branch(exec, cwd).await?;
    let base = git(exec, cwd, &["merge-base", "HEAD", &reference]).await?;
    if base.trim().is_empty() {
        return None;
    }
    let range = format!("{}..HEAD", base.trim());
    let diff = git(
        exec,
        cwd,
        &[
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--numstat",
            &range,
        ],
    )
    .await?;
    let mut additions = 0_u64;
    let mut deletions = 0_u64;
    for row in diff.lines() {
        let mut columns = row.split('\t');
        additions = additions.saturating_add(
            columns
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
        );
        deletions = deletions.saturating_add(
            columns
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
        );
    }
    Some(BranchChanges {
        additions,
        deletions,
    })
}

async fn default_branch(exec: &dyn WorkspaceExec, cwd: &Path) -> Option<String> {
    let remotes = git(exec, cwd, &["remote"]).await.unwrap_or_default();
    let origin = remotes.lines().find(|remote| *remote == "origin");
    for remote in origin
        .into_iter()
        .chain(remotes.lines().filter(|remote| *remote != "origin"))
    {
        if let Some(reference) = remote_default(exec, cwd, remote).await {
            return Some(reference);
        }
    }
    for reference in ["refs/heads/main", "refs/heads/master"] {
        if ref_exists(exec, cwd, reference).await {
            return Some(reference.to_owned());
        }
    }
    None
}

async fn remote_default(exec: &dyn WorkspaceExec, cwd: &Path, remote: &str) -> Option<String> {
    let prefix = format!("refs/remotes/{remote}/");
    let head = format!("{prefix}HEAD");
    if let Some(reference) = git(exec, cwd, &["symbolic-ref", "--quiet", &head]).await {
        let reference = reference.trim();
        if reference.starts_with(&prefix) && ref_exists(exec, cwd, reference).await {
            return Some(reference.to_owned());
        }
    }
    let output = git(exec, cwd, &["remote", "show", "--", remote]).await?;
    for name in output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("HEAD branch:"))
        .map(str::trim)
    {
        let reference = format!("{prefix}{name}");
        if !name.is_empty() && ref_exists(exec, cwd, &reference).await {
            return Some(reference);
        }
    }
    None
}

async fn ref_exists(exec: &dyn WorkspaceExec, cwd: &Path, reference: &str) -> bool {
    git(exec, cwd, &["rev-parse", "--verify", "--quiet", reference])
        .await
        .is_some()
}

#[derive(Deserialize)]
struct GhPullRequest {
    number: u64,
    url: String,
    state: String,
}

#[derive(Deserialize)]
struct ApiPullRequest {
    number: u64,
    html_url: String,
    state: String,
}

fn open(number: u64, url: String, state: &str) -> Option<PullRequest> {
    state
        .eq_ignore_ascii_case("open")
        .then_some(PullRequest { number, url })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    name_with_owner: Option<String>,
    parent: Option<ParentRepository>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParentRepository {
    name_with_owner: String,
}

async fn open_pull_request(exec: &dyn WorkspaceExec, cwd: &Path) -> Option<PullRequest> {
    let branch = gh(exec, cwd, &["pr", "view", "--json", "number,url,state"]).await;
    if let Some(pr) = branch
        .as_deref()
        .and_then(|output| serde_json::from_str::<GhPullRequest>(output).ok())
        .and_then(|pr| open(pr.number, pr.url, &pr.state))
    {
        return Some(pr);
    }
    let head = git(exec, cwd, &["rev-parse", "HEAD"]).await?;
    let head = head.trim();
    if head.is_empty() {
        return None;
    }
    let info = gh(
        exec,
        cwd,
        &["repo", "view", "--json", "nameWithOwner,parent"],
    )
    .await?;
    let Repository {
        name_with_owner,
        parent,
    } = serde_json::from_str(&info).ok()?;
    let parent = parent.map(|parent| parent.name_with_owner);
    for repo in parent.iter().chain(
        name_with_owner
            .iter()
            .filter(|name| parent.as_ref() != Some(*name)),
    ) {
        let endpoint = format!("repos/{repo}/commits/{head}/pulls");
        let Some(output) = gh(
            exec,
            cwd,
            &[
                "api",
                "-H",
                "Accept: application/vnd.github+json",
                &endpoint,
            ],
        )
        .await
        else {
            continue;
        };
        if let Some(pr) = serde_json::from_str::<Vec<ApiPullRequest>>(&output)
            .ok()
            .and_then(|items| {
                items
                    .into_iter()
                    .find_map(|pr| open(pr.number, pr.html_url, &pr.state))
            })
        {
            return Some(pr);
        }
    }
    None
}

async fn git(exec: &dyn WorkspaceExec, cwd: &Path, args: &[&str]) -> Option<String> {
    let prefix = ["git", "-c", "safe.bareRepository=explicit"];
    let env = [("GIT_OPTIONAL_LOCKS", "0"), ("GIT_TERMINAL_PROMPT", "0")];
    run(exec, cwd, &prefix, args, &env).await
}

async fn gh(exec: &dyn WorkspaceExec, cwd: &Path, args: &[&str]) -> Option<String> {
    let env = [("GH_PROMPT_DISABLED", "1"), ("GIT_TERMINAL_PROMPT", "0")];
    run(exec, cwd, &["gh"], args, &env).await
}

async fn run(
    exec: &dyn WorkspaceExec,
    cwd: &Path,
    prefix: &[&str],
    args: &[&str],
    env: &[(&str, &str)],
) -> Option<String> {
    let call = Call {
        argv: prefix
            .iter()
            .chain(args)
            .copied()
            .map(str::to_owned)
            .collect(),
        env: env
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
        limits: Limits::Probe,
    };
    let output = exec.run(call, cwd).await.ok()?;
    (output.exit_code == 0).then_some(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tui_core::screens::transcript::diff::Output;
    use std::future::Future;
    use std::pin::Pin;

    struct ForkWorkspace;

    impl WorkspaceExec for ForkWorkspace {
        fn run(
            &self,
            call: Call,
            cwd: &Path,
        ) -> Pin<Box<dyn Future<Output = Result<Output, String>> + Send + '_>> {
            assert_eq!(cwd, Path::new("/work"));
            assert_eq!(call.limits, Limits::Probe);
            let argv: Vec<&str> = call.argv.iter().map(String::as_str).collect();
            let stdout = match argv.as_slice() {
                ["git", "-c", _, "rev-parse", "--git-dir"] => ".git",
                ["git", "-c", _, "remote"] => "upstream\norigin\n",
                [
                    "git",
                    "-c",
                    _,
                    "symbolic-ref",
                    "--quiet",
                    "refs/remotes/origin/HEAD",
                ] => "refs/remotes/origin/trunk\n",
                [
                    "git",
                    "-c",
                    _,
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    "refs/remotes/origin/trunk",
                ] => "target",
                [
                    "git",
                    "-c",
                    _,
                    "merge-base",
                    "HEAD",
                    "refs/remotes/origin/trunk",
                ] => "base",
                [
                    "git",
                    "-c",
                    _,
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--numstat",
                    "base..HEAD",
                ] => "5\t2\tfile\n-\t-\timage\n",
                ["git", "-c", _, "rev-parse", "HEAD"] => "head",
                ["gh", "pr", "view", "--json", "number,url,state"] => {
                    r#"{"number":1,"url":"https://example.com/closed","state":"CLOSED"}"#
                }
                ["gh", "repo", "view", "--json", "nameWithOwner,parent"] => {
                    r#"{"nameWithOwner":"me/fork","parent":{"nameWithOwner":"upstream/repo"}}"#
                }
                [
                    "gh",
                    "api",
                    "-H",
                    "Accept: application/vnd.github+json",
                    "repos/upstream/repo/commits/head/pulls",
                ] => {
                    r#"[{"number":1,"html_url":"https://example.com/closed","state":"closed"},{"number":42,"url":"https://api.example.com/pulls/42","html_url":"https://example.com/pr/42","state":"open"}]"#
                }
                _ => panic!("unexpected command {argv:?}"),
            };
            let stdout = stdout.to_owned();
            Box::pin(async move {
                Ok(Output {
                    exit_code: 0,
                    stdout,
                })
            })
        }
    }

    #[tokio::test]
    async fn a_fork_reads_the_upstream_open_pr_browser_url_and_committed_changes() {
        let summary = read(&ForkWorkspace, Path::new("/work")).await;
        assert_eq!(
            summary.pull_request,
            Some(PullRequest {
                number: 42,
                url: "https://example.com/pr/42".into()
            })
        );
        assert_eq!(
            summary.changes,
            Some(BranchChanges {
                additions: 5,
                deletions: 2
            })
        );
    }
}
