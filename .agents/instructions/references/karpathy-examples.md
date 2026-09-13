<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/engineering-baseline/references/karpathy-examples.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Applying the four foundations: examples

These original examples show how the Core foundations can shape a small,
reviewable change. They are teaching material, not replacement authority: the
[engineering baseline](../engineering-baseline.md) remains canonical. The
examples are inspired by the teaching structure of the upstream
[README](https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/README.md)
and [EXAMPLES.md](https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/EXAMPLES.md)
from multica-ai. Credit: **Karpathy-inspired guidance/examples by
multica-ai**. No upstream code is reproduced here, and the upstream material
is not a license for this repository.

## Applying FND-001 — Think before coding

See the canonical [FND-001 definition](../engineering-baseline.md#fnd-001--think-before-coding).
State assumptions before implementing; surface competing interpretations and
stop on a genuine blocker. A one-line, unambiguous change still needs only
proportional judgment, not a lengthy interview.

### Example 1: Scope and sensitivity are part of the request

**Request:** “Add an export button for account activity.”

**Tempting failure:** Add a button that downloads every account's raw event
rows, including IP address, internal actor ID, and retention-expired events.

**Why it fails:** “Account,” “activity,” and “export” hide scope, audience,
fields, and delivery assumptions. The choice can disclose sensitive data and
may violate retention rules.

**Better action:** Present a proposed interpretation, not an assumed
authorization: “I interpret this as the signed-in account only, with
timestamp, action, and target label; IP and internal IDs require explicit
approval; the date range and maximum rows are bounded; and the result is an
in-browser download rather than a job sent by email.” Confirm the requester
has authority under the real project's policy to approve those scope and
sensitivity choices. Require confirmation of consequential or ambiguous
points before implementing; if ownership or permitted fields remain unclear,
stop and name that blocker.

**Acceptance/evidence:** A review record names the scope, fields, retention
boundary, and delivery mode; a check confirms an account cannot request
another account's rows and rejected requests do not produce a file.

### Example 2: “Faster” has more than one meaning

**Request:** “Make the report faster.”

**Tempting failure:** Add a cache and background worker based on an invented
“current latency of 800 ms,” without measuring or deciding whether the need
is response time, throughput, or earlier visible progress.

**Why it fails:** The change may optimize the wrong outcome, serve stale
results, or add operational cost while the real bottleneck is report
rendering. A guessed number is not evidence.

**Better action:** Ask which outcome matters and measure a representative
baseline with the existing check or profiler. For example, compare report
query time and render time for a named fixture, then change only the observed
bottleneck. If the goal is perceived speed, consider showing the report
shell while the existing request completes; do not call it a latency win.

**Acceptance/evidence:** The record states the selected meaning of “faster,”
the measurement method and fixture, and the before/after result. Any sample
transcript is labeled hypothetical; no latency is presented as observed
unless the check actually ran.

## Applying FND-002 — Simplicity first

See the canonical [FND-002 definition](../engineering-baseline.md#fnd-002--simplicity-first).
Reuse existing standard behavior and add complexity only for a current
requirement. WET can be useful while a second occurrence proves whether two
things are genuinely the same; the rule of three is a judgment aid, not a
license for duplication without review.

### Example 1: Reuse the standard library

**Request:** “Parse a small configuration file with key/value lines.”

**Tempting failure:** Introduce a `ConfigReader` interface, provider factory,
plugin registry, and custom escaping grammar for one file format.

**Why it fails:** The abstraction has one implementation and creates behavior
the request did not specify. It also risks disagreeing with established
parsing rules.

**Better action:** Use the repository's existing parser if one exists; if
not, use a short, local implementation built from standard-library string
handling, with explicit rejection of malformed lines. Keep the excerpt
illustrative rather than pretending it is a complete parser:

```python
# Illustrative excerpt, not a complete parser.
key, separator, value = line.partition("=")
if not separator or not key.strip():
    raise ValueError("expected KEY=VALUE")
```

**Acceptance/evidence:** The review points to the reused parser or the small
new path, and checks cover one valid line plus malformed input. No plugin or
configuration surface exists without a stated requirement.

### Example 2: Do not build tomorrow's storage system

**Request:** “Remember the user's selected dashboard tab.”

**Tempting failure:** Add a preferences service, cache adapter, migration
framework, notification hook, and per-user strategy interface for one value.

**Why it fails:** It expands a single persistence requirement into several
untested policies and makes the eventual behavior harder to change.

**Better action:** Use the existing user-preference persistence path, storing
one named tab value with its existing validation and authorization. If no
such path exists, add the smallest field or record needed by this screen;
do not add caching or notifications until a requirement and evidence justify
them.

**Acceptance/evidence:** A check saves and reloads the selected tab for the
same authorized user, including the default when absent. The diff contains no
new abstraction whose only caller is this feature.

## Applying FND-003 — Surgical changes

See the canonical [FND-003 definition](../engineering-baseline.md#fnd-003--surgical-changes).
Trace changed lines to the stated goal. A scoped Boy Scout improvement is
welcome only inside the diff already required; unrelated cleanup belongs in a
separate change or an explicit report.

### Example 1: Fix the reported defect, not nearby style

**Request:** “Empty project names crash the label validator.”

**Tempting failure:** Reformat the validator, rename unrelated fields, tighten
URL validation, and remove an old helper while changing the empty-name path.

**Why it fails:** The extra edits obscure the defect, increase regression
surface, and make review unable to distinguish required behavior from taste.

**Better action:** Add the smallest guard at the existing validation boundary:

```diff
- name = payload["name"]
+ name = payload.get("name")
+ if not isinstance(name, str) or not name.strip():
+     raise ValueError("project name is required")
```

This is an illustrative excerpt, not a complete implementation. Leave URL
rules and unrelated helpers unchanged. Removing an import made unused by this
specific guard would be an acceptable scoped Boy Scout cleanup.

**Acceptance/evidence:** A reproduction for missing and whitespace-only names
fails before the guard and passes after it; the diff has no unrelated
formatting or validation policy changes.

### Example 2: Keep documentation routing bounded

**Request:** “Add examples for the four foundations and link them from the
baseline.”

**Tempting failure:** Rewrite the normative definitions, renumber matrix rows,
update unrelated catalog prose, and reorganize all reference files.

**Why it fails:** The request is documentation routing plus new examples, not a
policy revision. Broad edits can silently change the contract.

**Better action:** Add one examples reference with two examples per foundation
and change only the baseline's reading paragraph to link it conditionally.
Keep existing FND headings, IDs, definitions, and matrix rows unchanged.

**Acceptance/evidence:** A diff review shows the new examples reference and
only the necessary routing or index links, eight examples, four canonical
anchor links, and unchanged normative-definition and matrix-row counts.

## Applying FND-004 — Goal-driven execution

See the canonical [FND-004 definition](../engineering-baseline.md#fnd-004--goal-driven-execution).
Define an observable completion result and run the check chosen in advance.
Report failures and unavailable controls honestly. Documentation-only work
needs relevant structural/link checks, not a contrived behavior test.

### Example 1: Make a bug fix checkable

**Request:** “Stop duplicate webhook deliveries from creating two invoices.”

**Tempting failure:** Change the handler and report “done” because the code
looks idempotent, without first reproducing the duplicate delivery or checking
that the invoice key is constrained.

**Why it fails:** The claimed goal is observable—one invoice for the same
provider event—but an unverified implementation can still race or accept a
missing event ID.

**Better action:** Define the check first: submit the same event twice and
expect one invoice plus an idempotent second response; submit an event without
an ID and expect boundary rejection. Observe the regression failure before
the behavior fix when this is a behavior change, then run the focused check
and relevant existing invoice checks.

**Acceptance/evidence:** The report includes the actual red/green output (or
says which check was unavailable), the event identity used, and the existing
checks' results. It does not claim a concurrency guarantee that was not tested.

### Example 2: A documentation check can be honest and proportional

**Request:** “Add a reference page and link it from the baseline.”

**Tempting failure:** Add a permanent test fixture or claim the new guidance
is schema-validated even though this category has no schema support.

**Why it fails:** It tests behavior that does not exist and misstates the
repository's controls. A broken relative link would still be a real defect.

**Better action:** Define completion as: the new file exists, each section
links to its matching FND anchor, local links resolve (ignoring illustrative
fences), the package's expected counts remain stable, and the relevant offline
validator/checks complete. If a check cannot run, report it as unavailable;
do not convert that into “passed.” Runnable snippets, if any, get a safe
minimal check; illustrative excerpts are labeled and not executed.

**Acceptance/evidence:** The change record lists the commands actually run and
outputs, including validator status and any unavailable control. The report
separates structural evidence from behavioral certification.

## Source note

These examples are original Maestro-specific teaching content. They were
shaped by the four-foundation teaching approach in [multica-ai's README](https://github.com/multica-ai/andrej-karpathy-skills/blob/main/README.md)
and [EXAMPLES.md](https://github.com/multica-ai/andrej-karpathy-skills/blob/main/EXAMPLES.md),
read as source data for this addition. They do not reproduce that project's
installation, promotional, or configuration instructions.
