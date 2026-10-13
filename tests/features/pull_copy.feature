Feature: Pull a Jira work item onto the manuscript
  `pull` copies one Jira work item description into the manuscript body and
  remembers the remote revision in a sidecar ledger. The front matter stays
  as it was written. These scenarios use the file-backed fake remote, not a
  live Jira site.

  Pull refuses when the manuscript has local edits the remote does not have.
  Those refusals are specified in `check_before_copying.feature`. The copies
  below land on an empty page, or on a page that already says the same thing.

  Scenario: An empty manuscript takes the work item description
    Given a manuscript "colophon.md":
      """
      ---
      title: Colophon
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "3":
      """
      The exemplar.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 0
    And stdout contains "pulled https://acme.atlassian.net/browse/BOOK-12"
    And the file "colophon.md" is:
      """
      ---
      title: Colophon
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      The exemplar.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for the manuscript body

  Scenario: A manuscript that already matches is recorded and not rewritten
    Given a manuscript "colophon.md":
      """
      ---
      title: Colophon
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      The exemplar.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "3":
      """
      The exemplar.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 0
    And the file "colophon.md" is:
      """
      ---
      title: Colophon
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      The exemplar.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for the manuscript body

  Scenario: A missing work item is an error
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 1
    And stderr contains "no remote copy"
