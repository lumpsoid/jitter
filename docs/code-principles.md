# Code principles

Portable rules for writing simple, readable code. Language- and
framework-agnostic. Drop this next to a project's own instructions and let the
project file carry only what is actually project-specific: stack, layout,
architectural commitments, tooling gates.

---

## What we optimize for

Four properties. **Every rule below is derived from them. If a rule ever
conflicts with a property, the property wins and the rule is wrong** — say so
rather than following the rule off a cliff.

- **Testable** — logic can be exercised with plain values. If testing a rule
  needs a mock, an `await`, or a framework, the rule is in the wrong place.
- **Readable** — you can follow a call path by reading, without a graph tool.
- **Simple** — small modules that stay simple, composed into behavior that
  isn't.
- **Stable** — a one-line business change touches one file.

These four are in tension with each other, and that is the job. Most bad code
comes from maximizing one of them alone.

---

## Abstraction lives at the border

An interface earns its place when **either**:

- the real implementation cannot run in a test — network, disk, clock,
  platform channel, third-party SDK; **or**
- two or more real implementations ship.

**If a dependency is already abstract, fake that.** Wrapping an already-abstract
dependency in a second interface is a rename, not a boundary: it costs a hop on
every call path and buys nothing.

Good — one interface over a disk-backed store that no test can run.

Bad — three byte-identical one-method interfaces, one per consumer, each with
its own implementation, all wrapping that same already-abstract store.

Prefer a concrete class. Add the interface when a test forces you to, not in
anticipation.

---

## One owner for cross-module ordering

When several subsystems must run in a set order, **one named class owns that
order** and exposes intent-named flows. Call sites state what they want, not
how it sequences.

Such an owner is concrete, has no interface, and has a single implementation.
It earns its place by concentrating knowledge that would otherwise be
re-derived at every call site and drift — `configureReturningUser()` reads by
intent, and the "A must happen before B" constraint lives in exactly one place.

This is tell-don't-ask applied to sequencing. It is not an excuse for a god
object: an ordering owner holds *order*, not business rules.

---

## Rules live with their data

Put a rule at the smallest level that has enough data to decide:

1. **A rule about one value** → value object, parsed at construction. Invalid
   values cannot exist downstream, so nothing re-validates.
2. **A rule about one entity's own fields** → method on that entity,
   operation-named (`order.submit()`, `account.withdraw(amount)`). No public
   setters.
3. **A rule spanning entities, still pure** → named policy object. Thresholds
   are named config, never inline literals.
4. **A rule needing fetched data** → the caller loads the facts, then calls
   down for the verdict. **Fetching never decides; deciding never fetches.**

**Extract a rule out of its caller when it is reused, or when the caller cannot
be tested with plain values.** A rule used once, in one place, testable where it
sits, stays there — splitting it costs a hop and buys nothing. "Push everything
down" is how you get an unreadable call graph.

Never put a business `if` in an I/O method, a repository, or a job runner.

---

## Tell, don't ask

```
if (state.sparks - cost < 0) { … }                    // wrong
if (!state.balance.canAfford(SparkCost.match)) { … }  // right
```

Ask an object for a decision, not for its fields so you can decide for it.

---

## Make invalid states unrepresentable

A bool + nullable cluster that must stay in sync (`hasSelection` + `activeId?` +
`replyId?`) is a sum type waiting to happen.

A `?? ''` or a `!`/`as` written to satisfy the compiler at a boundary is a
design error — fix the type, don't paper over it.

---

## Structure

- **Models live in their own directory, one file per model.** Never inlined
  into a consumer.
- **A shared/`core` directory is infrastructure and primitives.** If something
  under it has its own screen, controller, or feature state, it is a feature in
  the wrong place.
- **Cross-feature handoff carries an id, not data.** The destination loads what
  it needs; passing loaded data across a boundary couples the two features'
  shapes together.
- **Consistency beats local cleverness** for whatever architectural loop the
  project has committed to. Deviate only with a stated reason.

---

## Known debt

**Don't propagate it, don't clean it up as a side effect.** Cleanup belongs in
a dedicated change, not in the margins of unrelated work. A project that
catalogues its debt should say where; the rule is the same either way — don't
add more instances, don't remove existing ones ad hoc.

---

## When the rules don't answer it

1. Find the closest existing example in the codebase and read it end to end.
   **The code is the specification** — a prose copy of a file layout goes stale
   the week after it is written, and you would go check the source anyway.
2. Prefer the smallest thing that satisfies the four properties. When in doubt,
   fewer layers.
3. If the existing pattern conflicts with what you need, **say so and propose a
   resolution** — don't invent a new pattern quietly, and don't restructure an
   existing module without being asked.

---

## Working style

- Minimal, targeted changes over rewrites. Touch only what the task needs.
- Direct responses, no padding.
- Verify before reporting done: re-read what you changed, run the project's
  analyze/lint/test gate.
- Reaching for a suppression comment (`// ignore:`, `# noqa`, `@ts-ignore`) to
  silence a *style* lint usually means you misremembered a language feature.
  Check how the surrounding code already solves it before suppressing anything.
- **Facts about the code** — a fix's rationale, a design decision, a gotcha, a
  ticket link — go in the commit body, the PR description, or a doc, where they
  travel with the code. Cross-session memory is for working style and process,
  not code facts.
