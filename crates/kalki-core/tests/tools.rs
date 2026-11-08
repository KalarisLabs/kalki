//! Integration tests for the built-in tools: policy gate, confinement, blob overflow, `apply_patch`.

use std::path::{Path, PathBuf};

use kalki_core::policy::PolicyEngine;
use kalki_core::tools::{ApplyPatchArgs, BashArgs, ToolContext, ToolError, apply_patch, run_bash};

const ALLOW_ALL: &str = r#"rules = [{ id = "allow-all", action = "allow" }]"#;

fn test_ctx(root: &Path) -> ToolContext {
    let workspace = root.join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace must exist for tests");
    ToolContext {
        workspace,
        blob_dir: root.join("blobs"),
        inline_cap: 4 * 1024,
    }
}

fn engine(toml: &str) -> PolicyEngine {
    PolicyEngine::from_toml_str(toml).expect("test policy must parse")
}

const MODIFY_PATCH: &str =
    "--- a/hello.txt\n+++ b/hello.txt\n@@ -1,3 +1,3 @@\n foo\n-bar\n+qux\n baz\n";

// --- bash: happy path -------------------------------------------------------

#[cfg(windows)]
#[tokio::test]
async fn bash_echo_prints_hello_on_windows() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let args = BashArgs {
        argv: vec![
            "cmd".to_string(),
            "/C".to_string(),
            "echo".to_string(),
            "hello".to_string(),
        ],
        timeout_secs: 15,
    };
    let execution = run_bash(&ctx, &engine(ALLOW_ALL), args)
        .await
        .expect("echo must run");
    assert_eq!(execution.output.exit_code, Some(0));
    assert!(
        execution.output.inline.contains("hello"),
        "inline: {:?}",
        execution.output.inline
    );
    assert!(!execution.truncated_to_blob);
}

#[cfg(unix)]
#[tokio::test]
async fn bash_echo_prints_hello_on_unix() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let args = BashArgs {
        argv: vec!["sh".to_string(), "-c".to_string(), "echo hello".to_string()],
        timeout_secs: 15,
    };
    let execution = run_bash(&ctx, &engine(ALLOW_ALL), args)
        .await
        .expect("echo must run");
    assert_eq!(execution.output.exit_code, Some(0));
    assert!(
        execution.output.inline.contains("hello"),
        "inline: {:?}",
        execution.output.inline
    );
    assert!(!execution.truncated_to_blob);
}

// --- bash: policy gate ------------------------------------------------------

#[tokio::test]
async fn bash_is_denied_when_policy_denies_publish() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let policy = engine(
        r#"rules = [{ id = "deny-cargo-publish", action = "deny", tools = ["bash"], commands = ["cargo publish"] }]"#,
    );
    let args = BashArgs {
        argv: vec![
            "cargo".to_string(),
            "publish".to_string(),
            "--allow-dirty".to_string(),
        ],
        timeout_secs: 15,
    };
    let err = run_bash(&ctx, &policy, args)
        .await
        .expect_err("must be denied");
    match err {
        ToolError::Policy(message) => assert!(
            message.contains("deny-cargo-publish"),
            "error must name the rule id, got: {message}"
        ),
        other => panic!("expected policy error, got: {other}"),
    }
}

#[tokio::test]
async fn bash_is_denied_by_default_when_no_rule_matches() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let args = BashArgs {
        argv: vec!["anything".to_string()],
        timeout_secs: 15,
    };
    let err = run_bash(&ctx, &engine("rules = []"), args)
        .await
        .expect_err("default deny");
    assert!(matches!(err, ToolError::Policy(_)));
}

#[tokio::test]
async fn bash_is_denied_when_ask_rule_matches_headless_v02() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let policy = engine(
        r#"rules = [{ id = "ask-cargo-publish", action = "ask", tools = ["bash"], commands = ["cargo publish"] }]"#,
    );
    let args = BashArgs {
        argv: vec!["cargo".to_string(), "publish".to_string()],
        timeout_secs: 15,
    };
    let err = run_bash(&ctx, &policy, args)
        .await
        .expect_err("ask is headless-deny in v0.2");
    match err {
        ToolError::Policy(message) => assert!(message.contains("ask-cargo-publish")),
        other => panic!("expected policy error, got: {other}"),
    }
}

// --- bash: confinement ------------------------------------------------------

#[tokio::test]
async fn bash_rejects_dotdot_segment_in_argv() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let args = BashArgs {
        argv: vec!["..".to_string(), "x".to_string()],
        timeout_secs: 15,
    };
    let err = run_bash(&ctx, &engine(ALLOW_ALL), args)
        .await
        .expect_err("must be confined");
    assert!(matches!(err, ToolError::Confinement(_)));
}

#[tokio::test]
async fn bash_rejects_absolute_path_outside_workspace() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let outside: PathBuf = std::env::temp_dir().join("kalki-outside-workspace-probe");
    let args = BashArgs {
        argv: vec![outside.to_string_lossy().into_owned()],
        timeout_secs: 15,
    };
    let err = run_bash(&ctx, &engine(ALLOW_ALL), args)
        .await
        .expect_err("outside path must be confined");
    assert!(matches!(err, ToolError::Confinement(_)));
}

// --- apply_patch ------------------------------------------------------------

#[tokio::test]
async fn apply_patch_dry_run_reports_hunks_without_writing() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let target = ctx.workspace.join("hello.txt");
    std::fs::write(&target, "foo\nbar\nbaz\n").expect("seed file");
    let patch = MODIFY_PATCH.to_string();
    let args = ApplyPatchArgs {
        patch,
        dry_run: true,
    };
    let execution = apply_patch(&ctx, args).await.expect("dry run must succeed");
    assert_eq!(execution.output.exit_code, None);
    assert!(
        execution.output.inline.contains("dry run"),
        "inline: {}",
        execution.output.inline
    );
    assert!(
        execution.output.inline.contains("1 hunk(s)"),
        "inline: {}",
        execution.output.inline
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("read back"),
        "foo\nbar\nbaz\n"
    );
}

#[tokio::test]
async fn apply_patch_applies_the_hunk() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let target = ctx.workspace.join("hello.txt");
    std::fs::write(&target, "foo\nbar\nbaz\n").expect("seed file");
    let patch = MODIFY_PATCH.to_string();
    let args = ApplyPatchArgs {
        patch,
        dry_run: false,
    };
    let execution = apply_patch(&ctx, args).await.expect("patch must apply");
    assert!(
        execution.output.inline.contains("changed"),
        "inline: {}",
        execution.output.inline
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("read back"),
        "foo\nqux\nbaz\n"
    );
}

#[tokio::test]
async fn apply_patch_fails_on_context_mismatch() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let target = ctx.workspace.join("hello.txt");
    std::fs::write(&target, "foo\nbar\nbaz\n").expect("seed file");
    let patch = "--- a/hello.txt\n+++ b/hello.txt\n@@ -1,3 +1,3 @@\n WRONG\n-bar\n+qux\n baz\n";
    let args = ApplyPatchArgs {
        patch: patch.to_string(),
        dry_run: false,
    };
    let err = apply_patch(&ctx, args)
        .await
        .expect_err("mismatch must fail");
    assert!(matches!(err, ToolError::Patch(_)));
    // The file must be untouched after a failed apply.
    assert_eq!(
        std::fs::read_to_string(&target).expect("read back"),
        "foo\nbar\nbaz\n"
    );
}

#[tokio::test]
async fn apply_patch_creates_new_file() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let patch = "--- /dev/null\n+++ b/created.txt\n@@ -0,0 +1,2 @@\n+alpha\n+beta\n";
    let args = ApplyPatchArgs {
        patch: patch.to_string(),
        dry_run: false,
    };
    apply_patch(&ctx, args).await.expect("create must succeed");
    assert_eq!(
        std::fs::read_to_string(ctx.workspace.join("created.txt")).expect("read back"),
        "alpha\nbeta\n"
    );
}

#[tokio::test]
async fn apply_patch_deletes_file() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let target = ctx.workspace.join("doomed.txt");
    std::fs::write(&target, "gone\n").expect("seed file");
    let patch = "--- a/doomed.txt\n+++ /dev/null\n@@ -1,1 +0,0 @@\n-gone\n";
    let args = ApplyPatchArgs {
        patch: patch.to_string(),
        dry_run: false,
    };
    apply_patch(&ctx, args).await.expect("delete must succeed");
    assert!(!target.exists());
}

#[tokio::test]
async fn apply_patch_rejects_path_escaping_workspace() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let patch = "--- a/../escape.txt\n+++ b/../escape.txt\n@@ -0,0 +1,1 @@\n+owned\n";
    let args = ApplyPatchArgs {
        patch: patch.to_string(),
        dry_run: false,
    };
    let err = apply_patch(&ctx, args)
        .await
        .expect_err("escape must be rejected");
    assert!(matches!(err, ToolError::Confinement(_)));
}

// --- apply_patch dry-run: multi-hunk summary shape --------------------------

#[tokio::test]
async fn apply_patch_dry_run_for_missing_target_reports_patch_error() {
    let root = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(root.path());
    let patch = "--- a/nope.txt\n+++ b/nope.txt\n@@ -1,1 +1,1 @@\n-x\n+y\n";
    let args = ApplyPatchArgs {
        patch: patch.to_string(),
        dry_run: true,
    };
    let err = apply_patch(&ctx, args)
        .await
        .expect_err("missing file must fail");
    assert!(matches!(err, ToolError::Patch(_)));
}
