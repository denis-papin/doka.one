---
name: query-specs
description: Answer a question about the project's specifications by reading the `_ai/` folder. Use when the user asks "what does the spec say about X", "which feature covers Y", "is Z in scope", "what's the rule for …", or any question whose answer lives in the feature/spec docs under `_ai/`. Gives a human-readable, business-oriented answer that sums up the relevant business rules — not a technical dump. Read-only — it never modifies the repository. Always cite the implied Fxxxx feature.
---

# Query the `_ai` specifications

This skill answers a question by reading the project's specification corpus under
`_ai/` and reporting the answer **with a reference to the `Fxxxx` feature that
covers it**.

The answer must be **human-readable and business-oriented**: explain what the
product does and why, in plain language a non-developer stakeholder would follow.
**Sum up the underlying business rules** rather than quoting them verbatim or
dumping technical detail. Translate rule ids, routine names, field types, and
status codes into business meaning; mention them only as light supporting
references, never as the substance of the answer.

It is strictly **read-only**: it inspects files, it never creates,
edits, moves, or deletes anything.

The user invokes it as `/query-specs <question>` (the text after the skill name
is the question). If no question is supplied, ask the user what they want to know
before searching.

---

## What lives in `_ai/`

The folder is the single source of truth for *what the project should do*,
written before/alongside the code:

| Path | Contents |
| --- | --- |
| `_ai/features/Fxxxx-<slug>/Fxxxx.md` | The feature design: goal, gap, inputs, output, flow & routines, **rules** (`B-*` business rules), errors, examples, out-of-scope. |
| `_ai/features/Fxxxx-<slug>/IT-Fxxxx.md` | Integration-test spec (black-box, over the public API). |
| `_ai/global/architecture.md` | System layout (the Rust back end `server`), runtime. |
| `_ai/global/coding-rules.md` | Folder conventions (thin handlers delegating to business-named subfolders), logging rules, status-code contracts. |

Each `Fxxxx.md` opens with a fenced `yaml` block carrying its `id`, `stream-id`,
`status`, and `related_tests` — useful for confirming the feature a question maps
to.

---

## Workflow

1. **Read the question.** Pull out the concrete subject (a route, a rule, a
   routine name, a status code, a mode, a field, a folder, etc.).

2. **Locate the relevant docs.** Search `_ai/` for the subject — prefer the
   Grep/Glob/Read tools over shell. Typical moves:
   - `Glob` `_ai/features/*/F*.md` to list the features that exist.
   - `Grep` the subject across `_ai/` (e.g. the route, a rule id like `B-4`, a
     routine like `createResource`, a field like `status`).
   - `Read` the matching `Fxxxx.md` (and its `IT-` sibling, or the global docs)
     to confirm the answer in context.

3. **Identify the implied feature.** Map the answer to its `Fxxxx` id. If the
   subject spans the global docs only (architecture/coding-rules) and no single
   feature, say so and still name any feature whose rules reference it.

4. **Answer in business language.** Lead with the direct answer, phrased as what
   the product does for the user — not how the code does it. **Sum up the
   business rules** behind it in plain prose a non-technical stakeholder would
   understand (e.g. "when a request omits the priority, the system defaults it to
   'normal' and rejects any value outside the allowed set"). Cite rule ids only
   as a light parenthetical for traceability (e.g. "(rule **B-3**)"); the
   explanation, not the rule text, carries the answer.

5. **Cite the feature and source.** End with a clear reference line, e.g.
   `Feature: F0001 — create-resource` and a clickable link to the file/section you
   relied on (`[F0001.md](_ai/features/F0001-create-resource/F0001.md)`).

---

## Output shape

Keep it tight and readable — prose first, jargon last:

- **Answer** — one or two plain-language sentences that directly answer the
  question in business terms.
- **Business rules** — a short summary (a sentence or a few bullets) of the rules
  that govern this behaviour, written for a stakeholder, with rule ids in light
  parentheses for traceability.
- **Feature** — `Fxxxx — <slug>` and a markdown link to the file (and section
  anchor) you used.

Avoid: pasting raw rule text, code signatures, JSON shapes, or routine names as
the body of the answer. Translate them into business meaning. Use them only when
a stakeholder genuinely needs the exact value (e.g. a port or a limit).

If the `_ai/` folder does not cover the question, say so plainly, name the
closest feature (if any), and suggest where the answer might belong — do **not**
invent a rule.

---

## Constraints

- **Read-only.** Use only read/search tools (Read, Grep, Glob). Never call Edit,
  Write, or any command that changes files or git state.
- **Ground every claim in a file.** Base the answer on the spec, not on memory or
  the code. If spec and code disagree, the question is about the spec — report
  what the spec says and flag the drift.
- **Business-readable, not technical.** Answer for a stakeholder: summarise the
  business rules in plain language; keep rule ids, signatures, and field types as
  light references only.
- **Always name the `Fxxxx` feature.** That reference is the point of the skill.
