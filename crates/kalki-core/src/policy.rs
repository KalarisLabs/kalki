//! Deny-first TOML policy engine (v1). See `docs/ARCHITECTURE.md` §Tool plane.
//!
//! Rule ids are stable and additive; absence of a matching `allow` rule = denied.
//! Cedar replaces this engine in v1.5 behind the same `Decision` API (ADR-0003).

use serde::Deserialize;

/// One policy rule.
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    /// Stable, kebab-case identifier (e.g. `allow-ls-in-workspace`).
    pub id: String,
    /// `allow` | `deny` | `ask`
    pub action: Action,
    /// Tool names this rule applies to (empty = all tools).
    #[serde(default)]
    pub tools: Vec<String>,
    /// Command argv prefixes this rule matches (bash tool only). Each entry is a
    /// whitespace-separated prefix, so `"cargo publish"` matches
    /// `cargo publish --allow-dirty` without matching `cargo build`.
    #[serde(default)]
    pub commands: Vec<String>,
    /// Path prefixes (relative to the workspace root) this rule matches.
    /// Separators are normalized: forward and back slashes both match.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Whether this rule concerns network access. A `network = true` rule only
    /// matches requests that ask for the network; requests without network
    /// activity are never matched by it.
    #[serde(default)]
    pub network: bool,
}

/// Rule action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Permit without prompting.
    Allow,
    /// Refuse.
    Deny,
    /// Permit after interactive approval.
    Ask,
}

/// Parsed policy file.
#[derive(Debug, Clone, Deserialize)]
pub struct Policy {
    /// Ordered rules; first matching rule wins.
    pub rules: Vec<Rule>,
}

/// Input to a policy decision.
pub struct ToolRequest<'a> {
    /// Tool name (e.g. `bash`).
    pub tool: &'a str,
    /// Command argv when the tool is `bash` (`argv[0]` is the program).
    pub command: Option<&'a [String]>,
    /// Target path when the tool touches the filesystem.
    pub path: Option<&'a str>,
    /// Whether the call requests network access.
    pub network: bool,
}

/// Decision returned by [`PolicyEngine::decide`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Permitted.
    Allowed,
    /// Denied; carries the matching rule (or `None` = default deny).
    Denied {
        /// Rule that denied, if one matched.
        rule: Option<String>,
    },
    /// Requires interactive approval; carries the rule to approve against.
    NeedsApproval {
        /// Rule requiring approval.
        rule: String,
    },
}

/// Errors while loading policy files.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// TOML parse failure.
    #[error("policy toml: {0}")]
    Toml(#[from] toml::de::Error),
    /// A path/URI was not expressible in the request.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// Two rules declared the same id; rule ids are stable and must be unique.
    #[error("duplicate rule id: {rule}")]
    DuplicateRule {
        /// The duplicated rule id.
        rule: String,
    },
}

/// The policy engine. Construct once per session; cheap to clone.
pub struct PolicyEngine {
    policy: Policy,
}

impl PolicyEngine {
    /// Parse from a TOML string.
    ///
    /// # Errors
    /// Invalid TOML is returned as a [`PolicyError`], as is a duplicate rule id.
    pub fn from_toml_str(s: &str) -> std::result::Result<Self, PolicyError> {
        let policy: Policy = toml::from_str(s)?;
        let mut seen = std::collections::HashSet::with_capacity(policy.rules.len());
        for rule in &policy.rules {
            if !seen.insert(rule.id.as_str()) {
                return Err(PolicyError::DuplicateRule {
                    rule: rule.id.clone(),
                });
            }
        }
        Ok(Self { policy })
    }

    /// Decide for a tool request. Default is **deny**.
    ///
    /// First matching rule wins. A rule narrows the request on every axis it
    /// constrains: a `commands` rule requires the request to carry an argv with a
    /// matching prefix, a `paths` rule requires a matching path, and a
    /// `network = true` rule requires `req.network`. Constrained axes that are
    /// absent from the request never match — deny-first means a rule cannot widen
    /// what it matches by accident.
    #[must_use]
    pub fn decide(&self, req: &ToolRequest<'_>) -> Decision {
        for rule in &self.policy.rules {
            if !rule.tools.is_empty() && !rule.tools.iter().any(|t| t == req.tool) {
                continue;
            }
            if !rule.commands.is_empty() {
                let Some(cmd) = req.command else { continue };
                if !rule.commands.iter().any(|c| argv_prefix_matches(cmd, c)) {
                    continue;
                }
            }
            if !rule.paths.is_empty() {
                let Some(path) = req.path else { continue };
                if !rule.paths.iter().any(|p| path_starts_with(path, p)) {
                    continue;
                }
            }
            if rule.network && !req.network {
                continue;
            }
            return match rule.action {
                Action::Allow => Decision::Allowed,
                Action::Deny => Decision::Denied {
                    rule: Some(rule.id.clone()),
                },
                Action::Ask => Decision::NeedsApproval {
                    rule: rule.id.clone(),
                },
            };
        }
        Decision::Denied { rule: None }
    }
}

/// Match a rule's whitespace-separated `prefix` against an argv array.
fn argv_prefix_matches(cmd: &[String], prefix: &str) -> bool {
    let tokens: Vec<&str> = prefix.split_whitespace().collect();
    !tokens.is_empty()
        && tokens.len() <= cmd.len()
        && cmd.iter().zip(tokens).all(|(arg, tok)| arg == tok)
}

/// Prefix match that is safe on Windows: both `\` and `/` count as separators,
/// and comparison is case-insensitive on Windows (case-sensitive elsewhere).
fn path_starts_with(path: &str, prefix: &str) -> bool {
    let norm = |s: &str| s.replace('\\', "/");
    let trimmed = norm(prefix).trim_end_matches('/').to_string();
    let (p, pre) = if cfg!(windows) {
        (norm(path).to_lowercase(), trimmed.to_lowercase())
    } else {
        (norm(path), trimmed)
    };
    if pre.is_empty() {
        return false;
    }
    p == pre || p.starts_with(&format!("{pre}/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(toml: &str) -> PolicyEngine {
        PolicyEngine::from_toml_str(toml).expect("test policy must parse")
    }

    #[test]
    fn denies_everything_when_policy_is_empty() {
        let engine = engine("rules = []");
        // Named binding: the inline temporary only compiles on rustc newer than the MSRV.
        let argv = ["cargo".to_string()];
        let req = ToolRequest {
            tool: "bash",
            command: Some(&argv),
            path: None,
            network: false,
        };
        assert_eq!(engine.decide(&req), Decision::Denied { rule: None });
    }

    #[test]
    fn allows_bash_when_argv_prefix_matches_allow_rule() {
        let engine = engine(
            r#"rules = [{ id = "allow-cargo", action = "allow", tools = ["bash"], commands = ["cargo"] }]"#,
        );
        let argv = vec!["cargo".to_string(), "build".to_string()];
        let req = ToolRequest {
            tool: "bash",
            command: Some(&argv),
            path: None,
            network: false,
        };
        assert_eq!(engine.decide(&req), Decision::Allowed);
    }

    #[test]
    fn matches_multi_token_argv_prefix() {
        let engine = engine(
            r#"rules = [{ id = "deny-publish", action = "deny", tools = ["bash"], commands = ["cargo publish"] }]"#,
        );
        let argv = vec![
            "cargo".to_string(),
            "publish".to_string(),
            "--allow-dirty".to_string(),
        ];
        let req = ToolRequest {
            tool: "bash",
            command: Some(&argv),
            path: None,
            network: false,
        };
        assert_eq!(
            engine.decide(&req),
            Decision::Denied {
                rule: Some("deny-publish".to_string()),
            }
        );
    }

    #[test]
    fn deny_beats_allow_when_deny_rule_comes_first() {
        let engine = engine(
            r#"rules = [
                { id = "deny-cargo", action = "deny", tools = ["bash"], commands = ["cargo"] },
                { id = "allow-cargo", action = "allow", tools = ["bash"], commands = ["cargo"] },
            ]"#,
        );
        let argv = vec!["cargo".to_string(), "build".to_string()];
        let req = ToolRequest {
            tool: "bash",
            command: Some(&argv),
            path: None,
            network: false,
        };
        assert_eq!(
            engine.decide(&req),
            Decision::Denied {
                rule: Some("deny-cargo".to_string()),
            }
        );
    }

    #[test]
    fn ask_rule_yields_needs_approval() {
        let engine = engine(
            r#"rules = [{ id = "ask-deploy", action = "ask", tools = ["bash"], commands = ["deploy"] }]"#,
        );
        let argv = vec!["deploy".to_string(), "production".to_string()];
        let req = ToolRequest {
            tool: "bash",
            command: Some(&argv),
            path: None,
            network: false,
        };
        assert_eq!(
            engine.decide(&req),
            Decision::NeedsApproval {
                rule: "ask-deploy".to_string(),
            }
        );
    }

    #[test]
    fn path_prefix_matches_with_forward_and_backslashes() {
        let engine = engine(
            r#"rules = [{ id = "allow-src", action = "allow", tools = ["read"], paths = ["src/app"] }]"#,
        );
        for path in [
            "src/app",
            "src/app/main.rs",
            "src\\app",
            "src\\app\\main.rs",
            "src/app\\sub\\file.rs",
        ] {
            let req = ToolRequest {
                tool: "read",
                command: None,
                path: Some(path),
                network: false,
            };
            assert_eq!(engine.decide(&req), Decision::Allowed, "path: {path}");
        }
        let req = ToolRequest {
            tool: "read",
            command: None,
            path: Some("src/application/main.rs"),
            network: false,
        };
        assert_eq!(engine.decide(&req), Decision::Denied { rule: None });
    }

    #[test]
    fn network_rule_only_matches_when_request_asks_for_network() {
        let engine = engine(
            r#"rules = [{ id = "allow-fetch", action = "allow", tools = ["bash"], commands = ["fetch"], network = true }]"#,
        );
        let argv = vec!["fetch".to_string(), "example.com".to_string()];
        let off = ToolRequest {
            tool: "bash",
            command: Some(&argv),
            path: None,
            network: false,
        };
        assert_eq!(engine.decide(&off), Decision::Denied { rule: None });
        let on = ToolRequest {
            tool: "bash",
            command: Some(&argv),
            path: None,
            network: true,
        };
        assert_eq!(engine.decide(&on), Decision::Allowed);
    }

    #[test]
    fn rule_with_empty_tools_matches_all_tools() {
        let engine = engine(r#"rules = [{ id = "allow-all", action = "allow" }]"#);
        for tool in ["bash", "read", "write", "anything-else"] {
            let req = ToolRequest {
                tool,
                command: None,
                path: None,
                network: false,
            };
            assert_eq!(engine.decide(&req), Decision::Allowed, "tool: {tool}");
        }
    }

    #[test]
    fn rejects_duplicate_rule_ids() {
        let err = PolicyEngine::from_toml_str(
            r#"rules = [
                { id = "dup", action = "allow" },
                { id = "dup", action = "deny" },
            ]"#,
        )
        .err();
        assert!(matches!(
            err,
            Some(PolicyError::DuplicateRule { rule }) if rule == "dup"
        ));
    }

    #[test]
    fn commands_rule_does_not_match_request_without_command() {
        let engine =
            engine(r#"rules = [{ id = "allow-cargo", action = "allow", commands = ["cargo"] }]"#);
        let req = ToolRequest {
            tool: "read",
            command: None,
            path: None,
            network: false,
        };
        assert_eq!(engine.decide(&req), Decision::Denied { rule: None });
    }

    #[test]
    fn paths_rule_does_not_match_request_without_path() {
        let engine = engine(r#"rules = [{ id = "allow-src", action = "allow", paths = ["src"] }]"#);
        let req = ToolRequest {
            tool: "bash",
            command: None,
            path: None,
            network: false,
        };
        assert_eq!(engine.decide(&req), Decision::Denied { rule: None });
    }
}
