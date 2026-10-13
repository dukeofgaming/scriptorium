# ADR 0001: Parse YAML front matter with gray_matter

## Status

Accepted

## Context

A manuscript is Markdown. The `sync` property lives in YAML front matter, and it is either one link or a list of links. The body below the front matter is the text that is copied. Rewriting a file must keep every other front-matter key the scribe wrote.

Popular Rust options in 2026:

- `gray_matter` 0.3 (maintained port of the widely used gray-matter library). Extracts YAML, TOML, and JSON front matter and deserializes the block with serde. About 840k downloads, 74 dependents.
- `frontmatter-gen`. Newer, multi-format, but its main API returns a `Value` map and pulls in a large optional static-site stack.
- `pulldown-cmark` or `comrak` plus a hand-rolled `---` split. Excellent Markdown parsers, but front matter is not their job. `pulldown-cmark-frontmatter` reads a fenced code block, which is not the YAML `---` form scribes already write.
- `serde_yaml` on a manually sliced block. Works, and we still use serde, but we would own delimiter rules that gray-matter already settled.

The body is copied as text. Scriptorium does not need a full Markdown AST to find `sync` or to replace the body.

## Decision

Use `gray_matter` with its YAML engine to read front matter into a serde type. When a command rewrites the file, keep the original front-matter bytes and replace only the body. That preserves comments, key order, and keys this tool does not understand.

`pulldown-cmark` stays out of the MVP. Add it later if a slice needs structure inside the body.

## Consequences

- `sync` as a string or as a list is one untagged serde enum.
- A manuscript whose first block is not YAML front matter fails with a clear error.
- Body edits do not re-serialize YAML, so we cannot accidentally reformat the scribe's header.
