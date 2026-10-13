Feature: Push the manuscript onto a Jira work item
  `push` copies the manuscript body onto the Jira work item named in `sync`
  and records the new revision in the sidecar ledger. When the ledger, the
  manuscript, and the remote revision already agree, the remote is left
  untouched. These scenarios use the file-backed fake remote.

  A later slice refuses to overwrite a remote that has moved. The copies
  below either match the ledger or have never been stored.

  Scenario: A local edit is copied onto the work item
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      A local edit.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "3":
      """
      The exemplar.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 0
    And stdout contains "pushed https://acme.atlassian.net/browse/BOOK-12"
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" is at revision "4":
      """
      A local edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "4" for the manuscript body

  Scenario: A copy that already matches is not rewritten
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      The exemplar.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "3":
      """
      The exemplar.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 0
    And stdout contains "already in sync https://acme.atlassian.net/browse/BOOK-12"
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" is at revision "3":
      """
      The exemplar.
      """

  Scenario: The first push creates a missing work item
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      A first copy.
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 0
    And stdout contains "pushed https://acme.atlassian.net/browse/BOOK-12"
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" is at revision "1":
      """
      A first copy.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "1" for the manuscript body
