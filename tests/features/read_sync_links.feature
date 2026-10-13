Feature: Read which copies a manuscript names
  The YAML front matter of a Markdown file holds a `sync` property. It may
  name one link, or a list of links. Each link is a Jira work item, a
  Confluence page, or a Google Doc. `scriptorium links` prints that
  classification and does not copy anything.

  Scenario: One Jira work item
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/browse/BOOK-12
      ---
      The colophon.
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 0
    And stdout contains "jira BOOK-12 https://acme.atlassian.net/browse/BOOK-12"

  Scenario: A list of three kinds of copy
    Given a manuscript "colophon.md":
      """
      ---
      title: Colophon
      sync:
        - https://acme.atlassian.net/browse/BOOK-12
        - https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon
        - https://docs.google.com/document/d/abc_XYZ-123/edit
      ---
      The colophon.
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 0
    And stdout contains "jira BOOK-12 https://acme.atlassian.net/browse/BOOK-12"
    And stdout contains "confluence 98765 https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon"
    And stdout contains "google-doc abc_XYZ-123 https://docs.google.com/document/d/abc_XYZ-123/edit"

  Scenario: A page id in the query string is still a Confluence page
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/wiki/pages/viewpage.action?pageId=42
      ---
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 0
    And stdout contains "confluence 42 https://acme.atlassian.net/wiki/pages/viewpage.action?pageId=42"

  Scenario: A board link that names the work item
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://acme.atlassian.net/jira/software/projects/BOOK/boards/1?selectedIssue=BOOK-12
      ---
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 0
    And stdout contains "jira BOOK-12 https://acme.atlassian.net/jira/software/projects/BOOK/boards/1?selectedIssue=BOOK-12"

  Scenario: Front matter without sync is an error
    Given a manuscript "colophon.md":
      """
      ---
      title: Colophon
      ---
      The colophon.
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 1
    And stderr contains "sync"

  Scenario: An empty sync list names no copies
    Given a manuscript "colophon.md":
      """
      ---
      sync: []
      ---
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 1
    And stderr contains "no links"

  Scenario: An unknown link is an error
    Given a manuscript "colophon.md":
      """
      ---
      sync: https://example.com/not-a-copy
      ---
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 1
    And stderr contains "unrecognized"

  Scenario: A file with no front matter is an error
    Given a manuscript "colophon.md":
      """
      The colophon, with no header.
      """
    When I run "scriptorium links colophon.md"
    Then the exit code is 1
    And stderr contains "front matter"

  Scenario: A missing manuscript is an error
    When I run "scriptorium links missing.md"
    Then the exit code is 1
    And stderr contains "missing.md"
