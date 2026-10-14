Feature: Copy GitHub issues and pull requests
  A manuscript may name a GitHub issue or a GitHub pull request, alone or
  in a list beside the other copies. Every link is a copy of the same body.
  The copied text is the issue body or the pull request body. Pull writes
  that body only when every remote says the same text. Push writes the
  manuscript to each link that is safe to update, and writes nothing if any
  link must be refused. A diverged copy is refused unless `--force` is
  passed.

  These scenarios use the file-backed fake remote, not the GitHub CLI.

  Scenario: One GitHub issue
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/issues/12
      ---
      The colophon.
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 0
    And stdout contains "github-issue acme/library#12 https://github.com/acme/library/issues/12"

  Scenario: A files link is still the pull request
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/pull/4/files
      ---
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 0
    And stdout contains "github-pr acme/library#4 https://github.com/acme/library/pull/4/files"

  Scenario: A list names an issue, a pull request, and a Jira work item
    Given a manuscript "colophon.md":
      """
      ---
      title: Colophon
      sync:
        - https://github.com/acme/library/issues/12
        - https://github.com/acme/library/pull/4
        - https://acme.atlassian.net/browse/BOOK-12
      ---
      The colophon.
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 0
    And stdout contains "github-issue acme/library#12 https://github.com/acme/library/issues/12"
    And stdout contains "github-pr acme/library#4 https://github.com/acme/library/pull/4"
    And stdout contains "jira BOOK-12 https://acme.atlassian.net/browse/BOOK-12"

  Scenario: A repository home page is not a copy
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library
      ---
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 1
    And stderr contains "unrecognized"

  Scenario: An empty manuscript takes the issue body
    Given a manuscript "colophon.md":
      """
      ---
      title: Colophon
      sync: https://github.com/acme/library/issues/12
      ---
      """
    And the remote copy "https://github.com/acme/library/issues/12" at revision "3":
      """
      The exemplar.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 0
    And stdout contains "pulled https://github.com/acme/library/issues/12"
    And the file "colophon.md" is:
      """
      ---
      title: Colophon
      sync: https://github.com/acme/library/issues/12
      ---
      The exemplar.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/issues/12" at revision "3" for the manuscript body

  Scenario: A missing issue is an error
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/issues/12
      ---
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 1
    And stderr contains "no remote copy"

  Scenario: The first push creates a missing issue
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/issues/12
      ---
      A first copy.
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 0
    And stdout contains "pushed https://github.com/acme/library/issues/12"
    And the remote copy "https://github.com/acme/library/issues/12" is at revision "1":
      """
      A first copy.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/issues/12" at revision "1" for the manuscript body

  Scenario: A local edit is copied onto the pull request
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/pull/4
      ---
      A local edit.
      """
    And the remote copy "https://github.com/acme/library/pull/4" at revision "3":
      """
      The exemplar.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/pull/4" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 0
    And stdout contains "pushed https://github.com/acme/library/pull/4"
    And the remote copy "https://github.com/acme/library/pull/4" is at revision "4":
      """
      A local edit.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/pull/4" at revision "4" for the manuscript body

  Scenario: A copy that already matches is not rewritten
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/issues/12
      ---
      The exemplar.
      """
    And the remote copy "https://github.com/acme/library/issues/12" at revision "3":
      """
      The exemplar.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/issues/12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 0
    And stdout contains "already in sync https://github.com/acme/library/issues/12"
    And the remote copy "https://github.com/acme/library/issues/12" is at revision "3":
      """
      The exemplar.
      """

  Scenario: Push copies one body to the issue and the pull request
    Given a manuscript "colophon.md":
      """
      ---
      sync:
        - https://github.com/acme/library/issues/12
        - https://github.com/acme/library/pull/4
      ---
      One text.
      """
    And the remote copy "https://github.com/acme/library/issues/12" at revision "1":
      """
      """
    And the remote copy "https://github.com/acme/library/pull/4" at revision "1":
      """
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 0
    And stdout contains "pushed https://github.com/acme/library/issues/12"
    And stdout contains "pushed https://github.com/acme/library/pull/4"
    And the remote copy "https://github.com/acme/library/issues/12" is at revision "2":
      """
      One text.
      """
    And the remote copy "https://github.com/acme/library/pull/4" is at revision "2":
      """
      One text.
      """

  Scenario: Pull refuses when the issue and the pull request disagree
    Given a manuscript "colophon.md":
      """
      ---
      sync:
        - https://github.com/acme/library/issues/12
        - https://github.com/acme/library/pull/4
      ---
      """
    And the remote copy "https://github.com/acme/library/issues/12" at revision "2":
      """
      From the issue.
      """
    And the remote copy "https://github.com/acme/library/pull/4" at revision "5":
      """
      From the pull request.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 1
    And stderr contains "disagree"
    And the file "colophon.md" is:
      """
      ---
      sync:
        - https://github.com/acme/library/issues/12
        - https://github.com/acme/library/pull/4
      ---
      """

  Scenario: Pull keeps one text when the issue and the pull request agree
    Given a manuscript "colophon.md":
      """
      ---
      sync:
        - https://github.com/acme/library/issues/12
        - https://github.com/acme/library/pull/4
      ---
      """
    And the remote copy "https://github.com/acme/library/issues/12" at revision "2":
      """
      The same text.
      """
    And the remote copy "https://github.com/acme/library/pull/4" at revision "5":
      """
      The same text.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 0
    And the file "colophon.md" is:
      """
      ---
      sync:
        - https://github.com/acme/library/issues/12
        - https://github.com/acme/library/pull/4
      ---
      The same text.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/issues/12" at revision "2" for the manuscript body
    And the ledger for "colophon.md" records "https://github.com/acme/library/pull/4" at revision "5" for the manuscript body

  Scenario: Both sides of an issue changed
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/issues/12
      ---
      A local edit.
      """
    And the remote copy "https://github.com/acme/library/issues/12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/issues/12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium check colophon.md"
    Then the exit code is 3
    And stdout contains "diverged https://github.com/acme/library/issues/12"

  Scenario: Pull refuses a diverged issue
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/issues/12
      ---
      A local edit.
      """
    And the remote copy "https://github.com/acme/library/issues/12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/issues/12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 3
    And stderr contains "refusing to overwrite https://github.com/acme/library/issues/12"
    And the file "colophon.md" is:
      """
      ---
      sync: https://github.com/acme/library/issues/12
      ---
      A local edit.
      """

  Scenario: Push refuses when the pull request moved
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/pull/4
      ---
      The exemplar.
      """
    And the remote copy "https://github.com/acme/library/pull/4" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/pull/4" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 3
    And stderr contains "refusing to overwrite https://github.com/acme/library/pull/4"
    And the remote copy "https://github.com/acme/library/pull/4" is at revision "4":
      """
      A remote edit.
      """

  Scenario: Push with force overwrites the issue
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://github.com/acme/library/issues/12
      ---
      A local edit.
      """
    And the remote copy "https://github.com/acme/library/issues/12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/issues/12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium push colophon.md --force"
    Then the exit code is 0
    And stdout contains "pushed https://github.com/acme/library/issues/12"
    And the remote copy "https://github.com/acme/library/issues/12" is at revision "5":
      """
      A local edit.
      """
    And the ledger for "colophon.md" records "https://github.com/acme/library/issues/12" at revision "5" for the manuscript body
