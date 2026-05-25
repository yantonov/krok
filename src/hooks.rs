use anyhow::{Result, bail};

// Hook names are taken from githooks(5). Groups are kept explicit so the
// taxonomy of the documentation stays visible in the code.

const APPLYPATCH_HOOKS: &[&str] = &[
    "applypatch-msg",
    "pre-applypatch",
    "post-applypatch",
];

const COMMIT_WORKFLOW_HOOKS: &[&str] = &[
    "pre-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
];

const EMAIL_WORKFLOW_HOOKS: &[&str] = &[
    "pre-rebase",
    "post-rewrite",
    "post-checkout",
    "post-merge",
    "pre-push",
    "pre-auto-gc",
];

const OTHER_CLIENT_HOOKS: &[&str] = &[
    "sendemail-validate",
    "fsmonitor-watchman",
    "p4-pre-submit",
    "p4-prepare-changelist",
    "p4-changelist",
    "p4-post-changelist",
    "post-index-change",
];

const SERVER_SIDE_HOOKS: &[&str] = &[
    "pre-receive",
    "update",
    "proc-receive",
    "post-receive",
    "post-update",
    "push-to-checkout",
    "reference-transaction",
];

const NEWER_HOOKS: &[&str] = &[
    "pre-merge-commit",
];

const ALL_GROUPS: &[&[&str]] = &[
    APPLYPATCH_HOOKS,
    COMMIT_WORKFLOW_HOOKS,
    EMAIL_WORKFLOW_HOOKS,
    OTHER_CLIENT_HOOKS,
    SERVER_SIDE_HOOKS,
    NEWER_HOOKS,
];

pub fn is_known(name: &str) -> bool {
    ALL_GROUPS.iter().any(|group| group.contains(&name))
}

pub fn ensure_valid(name: &str, force: bool) -> Result<()> {
    if force || is_known(name) {
        Ok(())
    } else {
        bail!(
            "'{}' is not a known git hook name; pass --force (-f) to override",
            name
        )
    }
}
