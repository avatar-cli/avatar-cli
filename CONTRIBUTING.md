# Contributing Guidelines

## Acceptance Criteria

In order for us to accept contributions, the merge request must fulfill certain
requirements:

### Commit signatures
For security & regulations compliance, commits must be cryptographically signed
by PGP or GPG. You can read more about this topic here:
  - [Git's documentation](https://git-scm.com/book/en/v2/Git-Tools-Signing-Your-Work)
  - [Github's documentation](https://help.github.com/en/github/authenticating-to-github/signing-commits)
  - [Gitlab's documentation](https://docs.gitlab.com/ee/user/project/repository/gpg_signed_commits/).

### Commit messages

Commit messages must be properly formatted (following the
[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) rules).
The reasons behind this decision are many:
  - The project's history has to be "easy" to read.
  - It's easier to extract statistics from the commit logs.
  - It's easier to generate useful changelogs.
  - This practice enforces that committers think twice about the nature of their
    contributions.
  - Above everything else, it allows us to automate version numbering (following
    [Semantic Versioning](https://semver.org/) rules)

Because this requirement could generate too much overhead, we introduced some
tooling to ease our lives.

Running `cd cx && npm install` (which requires NodeJS) configures some git hooks
that will help the committer to generate compliant commit messages.

### Branch history

The merge request's commits have to present a "clean" history, `git rebase` is
your friend. This means:
  - linear history
  - commit messages matching what the commit does
  - no "experimental" commits + their revert commits

### Other considerations

[AIPs (Avatar-CLI Improvement Proposals)](https://gitlab.com/avatar-cli/aips/)
might also be important to decide if some contributions are acceptable or not.

It's a good idea to read these documents before start contributing.
