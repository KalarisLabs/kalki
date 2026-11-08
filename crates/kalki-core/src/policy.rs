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
    /// Command argv prefixes this rule matches (bash tool only).
    #[serde(default)]
    pub commands: Vec<String>,
    /// Path globs (relative to the workspace root) this rule matches.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Whether this rule concerns network access.
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
    /// Command argv when the tool is `bash` (argv[0] is the program).
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
}

/// The policy engine. Construct once per session; cheap to clone.
pub struct PolicyEngine {
    policy: Policy,
}

impl PolicyEngine {
    /// Parse from a TOML string.
    ///
    /// # Errors
    /// Invalid TOML is returned as a [`PolicyError`].
    pub fn from_toml_str(s: &str) -> std::result::Result<Self, PolicyError> {
        Ok(Self {
            policy: toml::from_str(s)?,
        })
    }

    /// Decide for a tool request. Default is **deny**.
    #[must_use]
    pub fn decide(&self, req: &ToolRequest<'_>) -> Decision {
        for rule in &self.policy.rules {
            if !rule.tools.is_empty() && !rule.tools.iter().any(|t| t == req.tool) {
                continue;
            }
            if let Some(cmd) = req.command {
                let argv0 = cmd.first().map_or("", String::as_str);
                if !rule.commands.is_empty() && !rule.commands.iter().any(|c| argv0 == c) {
                    continue;
                }
            }
            if let Some(path) = req.path
                && !rule.paths.is_empty()
                && !rule.paths.iter().any(|p| path_starts_with(path, p))
            {
                continue;
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

fn path_starts_with(path: &str, prefix: &str) -> bool {
    path == prefix
        || path.starts_with(&format!("{prefix}/"))
        || path.starts_with(&format!("{prefix}\\"))
}
