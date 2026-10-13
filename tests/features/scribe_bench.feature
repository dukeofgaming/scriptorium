Feature: The scribe's bench
  Scriptorium is the bench where a manuscript is copied. The tool answers
  with its name and offers the three copying commands: pull, push, and check.

  Scenario: The tool introduces itself
    When I run "scriptorium --version"
    Then the exit code is 0
    And stdout contains "scriptorium"

  Scenario: The tool lists the copying commands
    When I run "scriptorium --help"
    Then the exit code is 0
    And stdout contains "pull"
    And stdout contains "push"
    And stdout contains "check"
