# Jitter

A GUI application for watching Twitch streams.

<!-- agent-kit:rules begin. Written by /repo-rules from agent-kit/conventions/agent-rules.md; edit it there, not here. Put repo-specific exceptions under "## Repo overrides" outside this block. -->
## Agent rules

These rules apply to every agent working in this repo. A rule under
**Repo overrides** elsewhere in this file wins over this block.

### Code

Follow @CODE-PRINCIPLES.md. Project-specific rules (stack, layout, gates)
live in this file, outside this block.

### Commits

[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):
`<type>(<scope>): <subject>`.

- **type**: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `build`, `ci`,
  `chore`, `revert`.
- **scope**: the module or feature touched, lower-case. Leave it out when the
  change has no single home.
- **subject**: imperative, lower-case, no trailing period, 72 characters at
  most.
- **body**: why, not what. Rationale, design decisions and gotchas go here,
  where they travel with the code.
- **breaking**: `!` after the scope and a `BREAKING CHANGE: <what>` footer.
- One logical change per commit, with its gate green.

### Branches

- Never commit to `main` directly. Start each piece of work on a new
  branch cut from an up-to-date `main`, named `<type>/<slug>`
  (`feat/offline-sync`, `fix/empty-queue`).
- To land, rebase first, re-run the full gate on the rebased branch, then
  merge with a merge commit:

  ```sh
  git rebase main
  # full gate, green
  git switch main && git merge --no-ff <branch> && git branch -d <branch>
  ```

- No squash merges, no fast-forward merges. Never push or force-push unless
  asked.

### Tasks

Tasks live in the `agent-plan` store, outside the repo, so they can't be
committed by mistake. Never put a plan, task list or handoff in the repo.

- A track of work is one **meta task** `<track>-meta`, plus one **task** per
  unit of work, `<track>-<NN>-<slug>` (`sync-03-conflict-ui`).
- `agent-plan <name>` prints a plan's path, `agent-plan --list --name-prefix
  <track>-` lists a track, and `agent-plan <name> --append` adds stdin to the
  end of a plan.
- Read the meta task first. It is an index: open a task only when its
  "Open it for:" line says it holds what you need.
- **Starting a task:** set its Status to `in progress` and point the meta's
  **Active task** at it.
- **Finishing a task:** tick its acceptance criteria, add a Log line with
  the commit shas, set Status to `done (<date>)`. In the meta, update its
  Tasks entry with a one-line result and "Open it for:", copy leftovers into
  **Follow-ups** tagged `(<NN>)`, and move **Active task** on.
- A decision that crosses tasks goes in the meta's **Cross-task decisions**.
  A decision local to one task stays in that task.
- Durable facts (rationale, gotchas, API shape) go in commit bodies or this
  file, not only in a plan. When the track is finished, archive every task
  and then the meta: `agent-plan --archive <name>`.

Meta task (`<track>-meta`):

```markdown
# Meta: <track title>

**Context:** <why this track exists, in a paragraph>

Index only. Each task lives in its own plan (`agent-plan <name>`).
Only decisions that cross tasks are kept here.

**Active task:** <NN-slug, or none and why>

## References
- <design plan, fixture, external repo, doc> - <path or plan name>

## Cross-task decisions
- <decision> - <why> (<who decided, date> if the user did)

## Tasks
1. `<track>-01-<slug>` - **todo | in progress | done (<date>) | deferred (<why>)**.
   <one-line result once done>
   Open it for: <what only that plan holds>

## Not planned yet
<work the track will need later; add a task when it starts>

## Follow-ups
Open items left by finished tasks. Each names its source task.
- (<NN>) <item>
```

Task (`<track>-<NN>-<slug>`):

```markdown
# Task <NN>: <title>
Meta: `<track>-meta` · Status: **todo | in progress | done (<date>)** · Commits: <shas>

## Why
<what made this task necessary: the finding, bug or decision behind it>

## Goal
<the outcome, not steps>

## Decided
- <decision local to this task> - <why>

## Open (decide first)
- <question to settle before coding; move it to Decided once answered>

## Acceptance criteria
- [ ] <a concrete check: a test, a command and its output, an observed behaviour>

## Non-goals
<what this task leaves to which other task>

## Log
- <date>: <sha> <subject>; <what was learned>
```
<!-- agent-kit:rules end -->

## Stack

- **Rust** — all business logic, state, and Twitch access.
- **Flutter** — GUI head and platform effects only. No business logic in Dart.
- **[rinf](https://github.com/cunarist/rinf)** — the Rust↔Flutter bridge.
  Layering and signal rules: [rinf bridge](./docs/rinf-bridge.md).

## Layout

```
native/core   (jitter_core)    business logic + state. No rinf, no tokio, no I/O.
native/twitch (jitter_twitch)  Twitch access; returns core types.
native/store  (jitter_store)   SQLite; implements core's storage ports.
native/hub    (hub)            rinf bridge — the Flutter head's Rust side.
native/cli    (jitter_cli)     CLI head.
native/gtk    (jitter_gtk)     Experimental native GTK head.
lib/                           Flutter GUI head.
```

Each interaction head (GUI, TUI, CLI) is its own crate on top of core,
twitch and store. Dependencies point from heads to twitch and store, and
from those to core, never back.

## Architectural commitments

1. Rust owns state, in feature controllers that each own one state object.
   Rust sends view state to Flutter as `RustSignal`s; Dart sends user intents
   as `DartSignal`s.
2. The core is usable without Flutter: exercised by `cargo test` and the CLI.
   If the CLI cannot do something without Flutter, logic leaked out of core.
3. Core does not know Twitch's wire formats; twitch does not decide.
4. No panics in Rust (`unwrap`/`expect`/`panic!`); return `Result`.

## Gates

`make check` (`scripts/check.sh`, which CI runs too) runs all of them in
order, and checks that `Cargo.toml` and `pubspec.yaml` agree on the
version. Individually:

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`
- `rinf gen` after changing any signal struct, and on a fresh checkout
  before `flutter analyze` (the generated bindings are gitignored)
- `flutter analyze` / `flutter test` for the GUI

Releasing and the rest of the maintainer's jobs: [maintaining](./docs/maintaining.md).
