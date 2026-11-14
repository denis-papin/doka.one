---
name: guide-integration-test-rust-md
description: Write or fix the Markdown specification files for doka.one Rust integration tests. Use when the user wants to author, review, or correct an `integration_test_*.md` (or `unit_test_*.md`) spec under `_ai/<FEATURE>/`, or when they want the MD spec to match the conventions of an existing doka integration-test `.rs` file. Triggers on phrases like "write the integration test MD", "fix the IT spec", "document the integration tests for Fxxxx".
---

# doka.one — Integration-test Markdown specs

This skill encodes every in-house convention for the Markdown files that
**specify**  Rust integration tests. These MD files live next to the
feature design under `_ai/<FEATURE-ID>/` and act as the human-readable,
traceable contract that the dedicated `doka-api-tests/tests/<FEATURE>.rs` file
implements.

The MD is a **specification**, not a transcript. It is written *before or
alongside* the `.rs` file, in black-box language, and every assertion it lists
maps to a concrete behavioural outcome and the business rule that outcome
proves.

Reference exemplars (read them before writing a new one):
- `_ai/F0003-global-sql-search/integration_test_global_sql_search.md`
- `_ai/F0004-no-filter-in-search/integration_test_no_filter_in_search.md`
- Their `.rs` counterparts: `doka-api-tests/tests/F0003-global-sql-search.rs`,
  `doka-api-tests/tests/F0004-no-filter-in-search.rs`
- The shared harness: `doka-api-tests/tests/test_lib.rs`

---

## When to use

- Authoring a brand-new `integration_test_<slug>.md` for a feature/fix.
- Correcting an existing spec so it matches the conventions below.
- Re-syncing an MD spec with its `.rs` file after the test code changed
  (test names, TC numbering, seeded items, assertions).

If the user only wants the `.rs` test code, this skill still applies for the
spec; flag that the two must stay in lock-step.

---

## File location & naming

- Path: `_ai/<FEATURE-ID>/integration_test_<slug>.md`
  (e.g. `_ai/F0003-global-sql-search/integration_test_global_sql_search.md`).
- Unit-test specs use `unit_test_<slug>.md` / `unit_tests_<slug>.md` in the
  same folder — same structure, but the "System under test" is the internal
  function, not the HTTP API.
- The `<FEATURE-ID>` folder also holds the feature design (`F000X.md`). Keep
  the naming prefix consistent: feature `F0003` → spec id `IT-F0003`.

---

## Required structure

Produce the sections **in this order**:

### 1. YAML front-matter (fenced ```yaml block, not `---`)

The existing files use a fenced ```yaml block at the very top (with a leading
blank line), **not** Jekyll-style `---` fences. Match that.

```yaml
id: IT-F0004
title: Integration tests — No-filter in item search (empty DFS → match all)
type: integration-test
status: draft
target_stream: ITEM-SEARCH
related_feature: F0004      # or related_fix: F0003 for a bug fix
naming_prefix: IT-F0004
language: rust
```

Rules:
- `id` / `naming_prefix` = `IT-` + feature id.
- `type`: `integration-test` (or `unit-test`).
- `status`: starts at `draft`.
- `target_stream: the business flow under test (e.g. `ITEM-SEARCH`).
- Use `related_fix:` for bug branches (`Bxxxx`/`Fxxxx` fixes), `related_feature:`
  for features.

### 2. `# Coverage goal`

A short prose paragraph stating the **behavioural, end-to-end** intent, with the
key phrase that this is validated **through the public HTTP API** (black-box).
Follow it with a bullet list: each bullet is one observable outcome and, in a
dash clause, the rule it proves. Example shape:

> Behavioural end-to-end validation, **through the public document-server HTTP
> API**, that … . What is asserted (black-box, no SQL inspection):
> - a `'` inside a value does not break the search — proves `'` is doubled …;
> - …

### 3. `# System under test`

Name the **real** moving parts explicitly:
- The server(s) and ports actually used by the harness:
  - `document-server` on `localhost:30070`
  - `admin-server` on `localhost:30060`
- The client methods exercised, fully qualified:
  `doka_cli::request_client::DocumentServerClient::{create_item, search_item}`,
  `AdminServerClient::login`.
- State the boundary plainly: **"No mock, no internal SQL inspection — the test
  only observes what the API returns."**
- When a signature makes the cases reachable, quote it in a ```rust block and
  add a wire-form table (see F0004's `filter` argument table).

### 4. `# Test cases`

One subsection per case: `## TC-<PREFIX>-NNN — <one-line title>`.
- Number cases `001`, `002`, … The numbering must align with the `.rs` test
  function order (see "TC ↔ test-fn mapping" below).
- Each case uses the **Given / (Filter / DFS filter) / When / Then** skeleton:

  - **Given**: the random tags (always note they come from
    `generate_random_tag()` "to avoid cross-run collisions on the shared
    customer schema"), and a **markdown table** of seeded items:

    | Item | `<tag_text>` value        | Should match |
    |------|---------------------------|--------------|
    | A    | `50 % de l'élite ranking` | yes          |
    | B    | `50X de l'élite ranking`  | no — reason  |

    Always give the *reason* in the "Should match" cell for the `no` rows.

  - **DFS filter** (or **Filter**): the exact filter string, in backticks.
    For escaping features, add the parser-level facts (canonical AST form,
    unescaped value) under a note, marked "already verified by IT-F000x, kept
    here for traceability".

  - **When**: the exact client call, e.g.
    `document_server.search_item(Some(&filter), &session_id)`.

  - **Then**: a **numbered** list. Each item states the observable outcome
    (which item is returned/excluded, `Ok(_)` / `Err(_)`) **and** the rule it
    proves, written as "proves …". Tie every behaviour back to a business rule
    (F0001/F0002/F0003 …) for traceability. Include the implicit
    `search_item must return Ok(_)` assertion where relevant (a 5xx means
    malformed SQL).

### 5. `# Regression coverage` (optional)

Call out any previously `#[ignore]`d or now-redundant test that this IT
supersedes, with a link, and state it should be removed.

### 6. `# Test file`

- Link to the dedicated `.rs` file with a **relative** link from the MD's
  location: `[`doka-api-tests/tests/F0004-no-filter-in-search.rs`](../../doka-api-tests/tests/F0004-no-filter-in-search.rs)`.
- State the module name (`f0004_no_filter_in_search_tests`) and that the
  `TEST_TO_RUN` list follows the F0003 convention.

---

## TC ↔ test-fn mapping (keep MD and .rs in lock-step)

The `.rs` files follow these conventions — the MD must mirror them:

- One **dedicated file per feature**: `doka-api-tests/tests/<FEATURE>.rs`,
  starting with `mod test_lib;`.
- A top-level `const TEST_TO_RUN: &[&str] = &[ … ];` listing every test
  function name, **in run order**. The harness in `test_lib.rs` uses this list
  to know when all tests finished and the shared customer schema can be dropped.
- Tests live in `#[cfg(test)] mod fXXXX_<slug>_tests { … }`.
- Test fn naming: `t<NN>_fXXXX_<short_desc>`, numbered by tens
  (`t10_…`, `t20_…`, `t30_…`). **TC-Fxxxx-001 ↔ t10_…, -002 ↔ t20_…**, etc.
  (A gap like `t110_…` for `TC-…-011` is fine — preserve it in both.)
- Every test returns `Result<(), ApiError<'static>>` and ends with
  `lookup.close(); Ok(())`.
- Each test opens with a `Lookup::new("<exact fn name>", TEST_TO_RUN)` (or a
  shared `login(test_name)` helper that does so) — the string **must** equal
  the function name and appear in `TEST_TO_RUN`.
- Random tags via a local `generate_random_tag()` helper.
- Servers: `AdminServerClient::new("localhost", 30060)`,
  `DocumentServerClient::new("localhost", 30070)`.
- Each test fn carries a header comment block restating the TC's
  Given/When/Then, and every `assert!` has a message that ties the outcome to
  the rule it proves (mirror the MD's "proves …" wording).

When **correcting** an MD, cross-check against the `.rs`:
- Does every `TEST_TO_RUN` entry have a matching `## TC-…` section (and vice
  versa)?
- Do the seeded items / filter strings / assertions in the MD match the code?
- Do the TC numbers line up with the `t<NN>` numbering?
Report any drift explicitly to the user.

---

## Style rules (project-wide)

- Write in **English**, prose wrapped to a readable width, matching the existing
  files' tone (precise, black-box, rule-traceable).
- Square brackets for runtime values in narration mirrors the code's
  `[{}]` log convention — but in the MD, use backticks for code, filters, AST
  forms, and identifiers.
- Use relative markdown links (`../../…`) to source files so they resolve from
  the `_ai/<FEATURE>/` folder. Always link the `.rs` file, the delegate /
  generator source when a rule references it, and sibling IT specs for
  traceability.
- Never claim SQL-string inspection or mocks — these tests are strictly
  black-box over the HTTP API.
- Keep a single source of truth per fact: parser/AST facts proven by an earlier
  IT are *referenced*, not re-proven ("kept here for traceability").

---

## Workflow

1. Read the feature/fix design (`_ai/<FEATURE>/F000X.md`) to learn the business
   rules and the flow id.
2. Read the sibling exemplar specs (F0003, F0004) for structure.
3. If the `.rs` file already exists, read it and derive TC sections from the
   actual tests; otherwise design the test cases from the rules.
4. Produce the MD in the section order above.
5. Cross-check MD ↔ `.rs` (TC numbering, seeded items, filters, assertions) and
   report any mismatch.
