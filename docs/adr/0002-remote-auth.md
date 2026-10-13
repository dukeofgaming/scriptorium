# ADR 0002: Authenticate with acli, a Confluence API token, and gcloud ADC

## Status

Accepted

## Context

The MVP copies one manuscript to Jira work items, Confluence pages, and Google Docs. The owner asked for the simplest authentication that does not leave us maintaining an OAuth app for Atlassian. Remote services sit behind a port so end-to-end tests use a file-backed fake, and live tests run only when environment variables are set.

What the official tools can do, as of October 2026:

- Atlassian's `acli` authenticates with `acli jira auth login` (browser OAuth or an API token). `acli jira workitem view` and `acli jira workitem edit` read and write a work item description. The published command set is Jira, Rovo Dev, and admin. Confluence page create and update are not in `acli`. Community requests for Confluence support have no ship date.
- Confluence Cloud's REST API v2 reads and updates a page with the same API token a scribe already creates for `acli` (`email` + token + site). Basic auth over HTTPS is Atlassian's documented token flow.
- Google Docs has no comparable first-party CLI for document bodies. The Docs REST API (`documents.get` and `documents.batchUpdate`) is the stable surface. Application Default Credentials via `gcloud auth application-default print-access-token` avoid a custom token cache. The Docs scope is not included in the default cloud-platform ADC login, so the scribe must pass a client id file and the documents scope once. A `GOOGLE_ACCESS_TOKEN` variable overrides gcloud for short-lived or CI use.

Embedding `yup_oauth2` installed-app flow, or the generated `google-docs1` client, would mean storing a client secret and keeping a large generated client in tree. Shelling out to `gcloud` for the token, then calling the two REST methods, is less code and uses the credential store the scribe already has.

## Decision

- Jira adapter: shell out to `acli`.
- Confluence adapter: Confluence REST API v2 with `ATLASSIAN_SITE`, `ATLASSIAN_EMAIL`, and `ATLASSIAN_API_TOKEN`.
- Google Docs adapter: Docs REST API. Token from `GOOGLE_ACCESS_TOKEN`, otherwise `gcloud auth application-default print-access-token`.
- One `RemotePort` trait. `SCRIPTORIUM_FAKE_REMOTE` points the binary at a JSON file of copies. Live tests are gated by `SCRIPTORIUM_LIVE=1`.

## Consequences

- Jira setup is `acli` login. Confluence setup is three environment variables, because `acli` cannot write pages.
- Google setup is a one-time ADC login that includes the Docs scope. See the README.
- End-to-end features never call the network. Adapter unit tests feed recorded JSON and a stub process runner.
