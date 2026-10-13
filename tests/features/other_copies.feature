Feature: Copy Confluence pages and Google Docs
  A manuscript may name a Confluence page or a Google Doc, alone or in a
  list beside a Jira work item. Every link is a copy of the same body.
  Pull writes that body only when every remote says the same text. Push
  writes the manuscript to each link that is safe to update, and writes
  nothing if any link must be refused. These scenarios use the file-backed
  fake remote, not live Atlassian or Google.

  Scenario: Pull a Confluence page onto an empty manuscript
    Given a manuscript "colophon.md":
      """
      ---
      title: Colophon
      sync: https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon
      ---
      """
    And the remote copy "https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon" at revision "2":
      """
      From the page.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 0
    And stdout contains "pulled https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon"
    And the file "colophon.md" is:
      """
      ---
      title: Colophon
      sync: https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon
      ---
      From the page.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon" at revision "2" for the manuscript body

  Scenario: Push a manuscript onto a Google Doc
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://docs.google.com/document/d/abc_XYZ-123/edit
      ---
      From the manuscript.
      """
    And the remote copy "https://docs.google.com/document/d/abc_XYZ-123/edit" at revision "1":
      """
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 0
    And stdout contains "pushed https://docs.google.com/document/d/abc_XYZ-123/edit"
    And the remote copy "https://docs.google.com/document/d/abc_XYZ-123/edit" is at revision "2":
      """
      From the manuscript.
      """

  Scenario: Push copies one body to every named remote
    Given a manuscript "colophon.md":
      """
      ---
      sync:
        - https://acme.atlassian.net/browse/BOOK-12
        - https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon
        - https://docs.google.com/document/d/abc_XYZ-123/edit
      ---
      One text.
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "1":
      """
      """
    And the remote copy "https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon" at revision "1":
      """
      """
    And the remote copy "https://docs.google.com/document/d/abc_XYZ-123/edit" at revision "1":
      """
      """
    When I run "scriptorium push colophon.md"
    Then the exit code is 0
    And stdout contains "pushed https://acme.atlassian.net/browse/BOOK-12"
    And stdout contains "pushed https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon"
    And stdout contains "pushed https://docs.google.com/document/d/abc_XYZ-123/edit"
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" is at revision "2":
      """
      One text.
      """
    And the remote copy "https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon" is at revision "2":
      """
      One text.
      """
    And the remote copy "https://docs.google.com/document/d/abc_XYZ-123/edit" is at revision "2":
      """
      One text.
      """

  Scenario: Pull refuses when the remote copies disagree
    Given a manuscript "colophon.md":
      """
      ---
      sync:
        - https://acme.atlassian.net/browse/BOOK-12
        - https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon
      ---
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "2":
      """
      From Jira.
      """
    And the remote copy "https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon" at revision "5":
      """
      From Confluence.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 1
    And stderr contains "disagree"
    And the file "colophon.md" is:
      """
      ---
      sync:
        - https://acme.atlassian.net/browse/BOOK-12
        - https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon
      ---
      """

  Scenario: Pull keeps one text when every remote agrees
    Given a manuscript "colophon.md":
      """
      ---
      sync:
        - https://acme.atlassian.net/browse/BOOK-12
        - https://docs.google.com/document/d/abc_XYZ-123/edit
      ---
      """
    And the remote copy "https://acme.atlassian.net/browse/BOOK-12" at revision "2":
      """
      The same text.
      """
    And the remote copy "https://docs.google.com/document/d/abc_XYZ-123/edit" at revision "5":
      """
      The same text.
      """
    When I run "scriptorium pull colophon.md"
    Then the exit code is 0
    And the file "colophon.md" is:
      """
      ---
      sync:
        - https://acme.atlassian.net/browse/BOOK-12
        - https://docs.google.com/document/d/abc_XYZ-123/edit
      ---
      The same text.
      """
    And the ledger for "colophon.md" records "https://acme.atlassian.net/browse/BOOK-12" at revision "2" for the manuscript body
    And the ledger for "colophon.md" records "https://docs.google.com/document/d/abc_XYZ-123/edit" at revision "5" for the manuscript body
