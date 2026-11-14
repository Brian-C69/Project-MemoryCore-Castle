# Contributing to MemoryCore Castle

Keep each pull request focused on one feature, fix, or documentation change. Start
from the current `main` branch and leave unrelated changes for another PR. If your
work depends on an open PR, link it and explain the dependency.

## Pull request titles

Use `type: short description` with an imperative verb:

- `feat: add an optional top-bar clock`
- `fix: verify companion presence before showing online`
- `docs: add contribution guidelines`
- `refactor: simplify provider selection`
- `test: cover companion admission failures`
- `chore: update build dependencies`

Describe the final change, not the history of the conversation or work session.

## Pull request descriptions

Use the [PR template](.github/pull_request_template.md) for every PR, including
drafts. Keep its four headings in this order; write `Not applicable` where a
section does not apply. A small change needs only a sentence or two per section.

1. **Problem**: explain the user-visible problem or missing capability. For bugs,
   include a concrete trigger and the observed behaviour. Link relevant issues.
2. **Changes**: describe the resulting behaviour and any important implementation
   choice. Keep this about the final diff.
3. **Validation**: record exact commands and completed results, including failures,
   skips, and relevant environment assumptions. Include screenshots or a short
   before/after description for visible UI changes.
4. **Risks and limitations**: list compatibility concerns, required configuration,
   dependencies, and checks that were not run. Write `None identified` when
   appropriate; do not imply that an unrun check passed.

Check the description against the final diff before requesting review. If the scope
changes, update both the title and description. Avoid raw logs and unsupported
claims that a failure is pre-existing; compare the same command and environment
on the base commit before making that claim.

## Validation

The Windows CI workflow runs:

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Run checks appropriate to the affected behaviour. For documentation-only changes,
review links and formatting and run `git diff --check`; application tests may be
marked not run with that reason. For code changes, include relevant regression
coverage and report which CI checks you ran locally and which remain unrun.

Separate clean-checkout tests from runs using a personal `.env`, saved credentials,
live services, or private helper scripts. Label local-only tooling explicitly so
reviewers know what they can reproduce from the repository. Only quote test totals
from a completed run.

For UI work, check keyboard access, focus, disabled and error states, and narrow
windows. Prefer native buttons, dropdowns, and scrollbars styled with Castle's
theme; keep controls sized to their content when a wider control is unnecessary.

## Privacy and scope

Never include credentials, personal memory, chat logs, private configuration, or
unredacted personal data in commits, screenshots, or PR descriptions. Use fixtures
for tests and redact screenshots. Do not bump the application version or include
generated installers unless the change explicitly concerns a release.

Keep replies to review findings concrete: verify the issue, fix it when needed,
and describe the validation performed. Merge and release remain separate from
submitting a PR.
