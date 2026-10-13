Feature: Check before overwriting a copy
  Scriptorium remembers the last copy that was known to match, in a sidecar
  ledger next to the manuscript. `check` compares the manuscript, the remote
  revision, and that ledger.

  The command is `check`, not `status`. Status would describe a working tree.
  Check is the comparison a copyist makes before the pen touches either page.
  `push` and `pull` run the same comparison and refuse to overwrite a diverged
  side unless the scribe passes `--force`. A diverged `check` exits 3.

  These scenarios use the file-backed fake remote. The work item is
  https://acme.atlassian.net/browse/BOOK-12.

  Scenario: Nothing has changed since the last copy
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
    When I run "scriptorium check colophon.md"
    Then the exit code is 0
    And stdout contains "in-sync https://acme.atlassian.net/browse/BOOK-12"

  Scenario: Only the manuscript changed
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
    When I run "scriptorium check colophon.md"
    Then the exit code is 0
    And stdout contains "local-ahead https://acme.atlassian.net/browse/BOOK-12"

  Scenario: Only the remote changed
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      The exemplar.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium check colophon.md"
    Then the exit code is 0
    And stdout contains "remote-ahead https://acme.atlassian.net/browse/BOOK-12"

  Scenario: Both sides changed
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      A local edit.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium check colophon.md"
    Then the exit code is 3
    And stdout contains "diverged https://acme.atlassian.net/browse/BOOK-12"

  Scenario: The manuscript has never been copied
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
    When I run "scriptorium check colophon.md"
    Then the exit code is 0
    And stdout contains "unsynced https://acme.atlassian.net/browse/BOOK-12"

  Scenario: Pull refuses a diverged manuscript
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      A local edit.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 3
    And stderr contains "refusing to overwrite https://acme.atlassian.net/browse/BOOK-12"
    And the file "colophon.md" is:
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      A local edit.
      """

  Scenario: Pull with force takes the remote text
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      A local edit.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium pull colophon.md --force"
    Then the exit code is 0
    And stdout contains "pulled https://acme.atlassian.net/browse/BOOK-12"
    And the file "colophon.md" is:
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "4" for the manuscript body

  Scenario: Pull copies a remote that moved while the manuscript did not
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      The exemplar.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 0
    And the file "colophon.md" is:
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      A remote edit.
      """

  Scenario: Push refuses when the remote moved
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      The exemplar.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 3
    And stderr contains "refusing to overwrite https://acme.atlassian.net/browse/BOOK-12"
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" is at revision "4":
      """
      A remote edit.
      """

  Scenario: Push with force overwrites the remote
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      A local edit.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "4":
      """
      A remote edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "3" for body:
      """
      The exemplar.
      """
    When I run "scriptorium push colophon.md --force"
    Then the exit code is 0
    And stdout contains "pushed https://acme.atlassian.net/browse/BOOK-12"
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" is at revision "5":
      """
      A local edit.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "5" for the manuscript body
