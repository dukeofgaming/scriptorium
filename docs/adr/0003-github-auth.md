# ADR 0003: Authenticate GitHub with the official gh CLI

## Status

Accepted

## Context

A manuscript may name a GitHub issue or a GitHub pull request next to the Jira work items, Confluence pages, and Google Docs it already copies. The copied text is the issue body or the pull request body. Comments are a separate thread and are not a second copy of the manuscript.

[ADR 0002](0002-remote-auth.md) chose the official tool when that tool can both read and write the body, and a REST API only when it cannot. `acli` can edit a Jira description. `acli` cannot edit a Confluence page, and Google has no body CLI, so those two use REST.

The GitHub CLI can do both for this slice:

- `gh issue view <url> --json body,updatedAt` and `gh pr view <url> --json body,updatedAt` read the body and the provider's version token.
- `gh issue edit <url> --body-file <file>` and `gh pr edit <url> --body-file <file>` write the body.
- `gh auth login` stores credentials. `gh` also reads `GH_TOKEN` or `GITHUB_TOKEN`, so a scribe or a CI job does not need a second token cache.

Calling the GitHub REST API directly would mean owning another bearer header and another set of error bodies for the same two operations `gh` already wraps. Issue and pull request numbers in a `github.com` URL are enough for `gh` to find the repository.

`updatedAt` changes when the body changes. It also changes when a label, assignee, or comment changes. That is the same tradeoff as Jira's `updated` field: the remote revision is the provider's version token, not a hash we invent. A comment therefore marks the remote ahead, and `push` refuses until the scribe passes `--force` or pulls.

## Decision

- GitHub adapter: shell out to `gh`.
- Issues use `gh issue`. Pull requests use `gh pr`. The noun comes from the classified link.
- The remote revision is `updatedAt`.
- `GH_PROMPT_DISABLED=1` is set on the child process so `gh` cannot stop and ask a question.
- End-to-end features keep using `SCRIPTORIUM_FAKE_REMOTE`. Live tests call `gh` only when `SCRIPTORIUM_LIVE=1` and a GitHub URL is set.

## Consequences

- GitHub setup is `gh auth login`, or `GH_TOKEN` / `GITHUB_TOKEN`. See the README.
- A `github.com` issue URL (`/issues/<number>`) and pull request URL (`/pull/<number>`) are sync links. A repository home page, a discussion, and any other host are not.
- A suffix such as `/files` or a comment fragment still names that pull request or issue. The body copied is the description, not the comment thread.
- The adapter updates an existing issue or pull request. It does not open a new one. The file-backed fake used in features still creates a missing copy at revision `1`, as it does for Jira.
