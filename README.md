# Scriptorium

Monks in a scriptorium copied a book by hand and kept the exemplar and the copy in agreement. Scriptorium does that for a Markdown manuscript and the Jira work items, Confluence pages, Google Docs, and GitHub issues and pull requests named in its front matter.

## Commands

| Command | What the scribe is doing |
| --- | --- |
| `pull <file>` | Copy the remote exemplar onto the manuscript. |
| `push <file>` | Copy the manuscript onto each remote. |
| `check <file>` | Compare the manuscript, the remote, and the last copy that matched. |
| `links <file>` | Show how each `sync` link was classified. Inspection only. |

`pull` and `push` take `--force` when the scribe means to overwrite a diverged copy.

`check` prints one line per link:

| Line | Meaning |
| --- | --- |
| `in-sync` | The body hash and the remote revision both match the ledger. |
| `local-ahead` | Only the manuscript changed. Safe to `push`. |
| `remote-ahead` | Only the remote revision changed. Safe to `pull`. |
| `diverged` | Both changed. Exit code 3. `push` and `pull` refuse without `--force`. |
| `unsynced` | No ledger entry yet. A blank side, or two sides that already say the same thing, may be copied. |

The remote revision is the provider's version token: Jira's `updated` field, the Confluence page version, a Google Docs `revisionId`, or the `updatedAt` of a GitHub issue or pull request. The fake store used in tests uses a decimal counter.

### Why `check`

The third command is `check`, not `status`. Status sounds like a tour of a working tree. Check is the comparison a copyist makes before the pen touches either page: does the manuscript still match the last agreed copy, and does the remote? `push` and `pull` run that same comparison and refuse to overwrite a diverged side unless `--force` is passed.

## Manuscript

```yaml
---
sync: https://acme.atlassian.net/browse/BOOK-12
---
```

`sync` may also be a list. Every link is another copy of the **same** body.

```yaml
---
title: Colophon
sync:
  - https://acme.atlassian.net/browse/BOOK-12
  - https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon
  - https://docs.google.com/document/d/DOCUMENT_ID/edit
  - https://github.com/acme/library/issues/12
  - https://github.com/acme/library/pull/4
---

The text that is copied.
```

Recognised links:

- Jira: `https://<site>.atlassian.net/browse/BOOK-12`, or a board URL whose query contains `selectedIssue=BOOK-12`
- Confluence: a `/wiki/.../pages/<id>` URL, or `pageId=<id>`
- Google Docs: `https://docs.google.com/document/d/<id>/...`
- GitHub issue: `https://github.com/<owner>/<repo>/issues/<number>`
- GitHub pull request: `https://github.com/<owner>/<repo>/pull/<number>`

A `/files` suffix or a comment fragment on a GitHub URL still names that issue or pull request. The copied text is the body. Discussion URLs and a repository home page are not copies.

The last agreed copy is stored beside the manuscript, not inside the front matter, so the header stays the scribe's. For `colophon.md` the ledger is `colophon.scriptorium.json`. It records each link's remote revision and a SHA-256 of the body.

## Auth

Jira uses the official Atlassian CLI. Confluence uses the Cloud REST API, because `acli` still has no page write command. Google Docs uses the Docs REST API with Application Default Credentials. GitHub issues and pull requests use the official GitHub CLI. Details and the reasoning are in [docs/adr/0002-remote-auth.md](docs/adr/0002-remote-auth.md) and [docs/adr/0003-github-auth.md](docs/adr/0003-github-auth.md).

1. Install [`acli`](https://developer.atlassian.com/cloud/acli/guides/how-to-get-started/) and log in:

   ```bash
   acli jira auth login --web
   ```

   Or pipe an API token: `echo "$ATLASSIAN_API_TOKEN" | acli jira auth login --site "acme.atlassian.net" --email "you@acme.com" --token`.

2. For Confluence, export the same kind of API token. The site host is taken from the page URL.

   ```bash
   export ATLASSIAN_EMAIL=you@acme.com
   export ATLASSIAN_API_TOKEN=...
   ```

3. For Google Docs, install the [Google Cloud CLI](https://cloud.google.com/sdk) and log in once with the Documents scope. The default `gcloud auth application-default login` token does **not** include that scope.

   ```bash
   gcloud auth application-default login \
     --client-id-file=client.json \
     --scopes=https://www.googleapis.com/auth/documents,openid,https://www.googleapis.com/auth/userinfo.email
   ```

   `client.json` is an OAuth **Desktop** client from a Google Cloud project that has the Docs API enabled. A short-lived token also works: `export GOOGLE_ACCESS_TOKEN=...`. If a quota project is required, set `GOOGLE_CLOUD_QUOTA_PROJECT`.

4. For GitHub, install the [GitHub CLI](https://cli.github.com/) and log in:

   ```bash
   gh auth login
   ```

   `gh` also reads `GH_TOKEN` or `GITHUB_TOKEN`. The copied text is the issue body or the pull request body.

Tests do not need any of this. Set `SCRIPTORIUM_FAKE_REMOTE` to a JSON file and the binary talks to that file. Live checks are skipped unless `SCRIPTORIUM_LIVE=1`.

## Parser

YAML front matter is read with [`gray_matter`](https://crates.io/crates/gray_matter). The body is replaced without re-serializing the header. See [docs/adr/0001-frontmatter-parser.md](docs/adr/0001-frontmatter-parser.md).

## Layout

Use cases are top-level modules (`pull`, `push`, `check`), independent of `acli`, `gh`, and HTTP. Adapters sit behind a remote port. Each vertical slice has one Gherkin file in `tests/features/` and runs the `scriptorium` binary.

## Development

Open the repository in the [dev container](.devcontainer/devcontainer.json) (Rust, rustfmt, clippy). Then:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

`cargo test` runs unit tests and the cucumber features. A fake remote (`SCRIPTORIUM_FAKE_REMOTE`) keeps those features off the network. To call a real service as well:

```bash
export SCRIPTORIUM_LIVE=1
export SCRIPTORIUM_LIVE_JIRA_URL=https://acme.atlassian.net/browse/BOOK-12
export SCRIPTORIUM_LIVE_CONFLUENCE_URL=https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon
export SCRIPTORIUM_LIVE_GOOGLE_DOC_URL=https://docs.google.com/document/d/DOCUMENT_ID/edit
export SCRIPTORIUM_LIVE_GITHUB_ISSUE_URL=https://github.com/acme/library/issues/12
export SCRIPTORIUM_LIVE_GITHUB_PR_URL=https://github.com/acme/library/pull/4
cargo test --test live
```

Set only the URLs you want to call. `SCRIPTORIUM_LIVE_PUSH=1` writes the fetched body back to that remote.
