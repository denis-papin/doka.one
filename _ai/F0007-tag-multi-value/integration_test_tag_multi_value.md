
```yaml
id: IT-F0007
title: Integration tests — Multi-value tags (set tags + ANY/ALL/NONE/IS filters)
type: integration-test
status: draft
target_flow: DOKA-ITEM-TAGS
related_feature: F0007
naming_prefix: IT-F0007
language: rust
```

# Coverage goal
Behavioural end-to-end validation, **through the public document-server and
admin-server HTTP APIs**, that the new `Multi` tag type — a set of values drawn
from a predefined vocabulary — round-trips correctly and that the four
set-algebra filter operators (`ANY`, `ALL`, `NONE`, `IS`) select the right
items, including when list elements carry a `%` wildcard, and when two `Multi`
tags (`color` and `module`) are combined in one filter.

What is asserted (black-box, no SQL inspection, no mocks):

- a `Multi` tag can be defined with a bounded `predef_values` vocabulary, and an
  item can carry an empty / single / multi-value set — proves the write path
  persists `value_multi` (F0007 rules 1–3);
- a written set is read back **sorted + deduped** — proves write-time
  canonicalization (F0007 rule 3);
- `ANY` returns items whose set overlaps the requested set — proves the
  `value_multi && ARRAY[…]` predicate (F0007 rule 6);
- `ALL` returns items that hold every requested value — proves the chained
  `&&` predicate (F0007 rule 6);
- `NONE` returns tagged items disjoint from the requested set — proves the
  `NOT (value_multi && …)` predicate (F0007 rule 6);
- `IS` returns the item whose set exactly equals the requested set, and
  `IS []` selects items whose stored set is empty — proves the
  `value_multi = ARRAY[…]` predicate (F0007 rule 6);
- a `%` element is expanded against `predef_values` before matching — proves
  compile-time wildcard expansion (F0007 rule 5);
- two `Multi` tags combine under `AND`, and a `Multi` tag combines with a scalar
  (`Int`) tag — proves the per-condition JOIN shape and that `Multi` does not
  disturb the scalar paths (F0007 rules 6, 8);
- every clause emits a concrete predicate — no clause dropping (F0007 rule 7):
  `ANY [Z%]` (wildcard → `∅`) overlaps an empty array and matches nothing;
  `ANY [%]` (full vocabulary) matches every non-empty set and **excludes** the
  empty-set item; the literal `[]` is the empty-set selector — `ANY []` and
  `IS []` match `{}`, `NONE []` matches every non-empty set, `ALL []` matches
  every item carrying the tag (vacuous);
- an item that **never carries** a `Multi` tag is returned by **no** single
  clause — `ANY`, `ALL`, `NONE`, or `IS` alike — because the join-per-tag shape
  yields no row for it (inherited F0003 behaviour). "Empty set" (a tagged item
  whose set is `{}`) and "untagged" (no tag row) are therefore **distinct** and
  never conflated;
- off-vocabulary writes, wildcards in `IS`, unknown literals, and cross-type
  operator misuse are all rejected — proves the F0007 error contract;
- every accepted search returns `Ok(_)` — a `5xx` would mean the emitted
  set-predicate SQL is malformed.

# System under test
The **public document-server HTTP API** (and admin-server for login), not the
internal generator. Specifically:

- `document-server` on `localhost:30070`, exercised through
  `doka_cli::request_client::DocumentServerClient::{create_tag, create_item,
  get_item, search_item}`;
- `admin-server` on `localhost:30060` for the login round-trip via
  `AdminServerClient::login`.

No mock, no internal SQL string inspection — every assertion is on what
`search_item` / `get_item` return, or on the `Ok`/`Err` of the call itself.

F0007 extends the wire types these methods carry; the `.rs` file must be
authored in lock-step once the API lands:

```rust
// dkdto/src/web_types.rs
pub enum EnumTagValue { /* … */ Multi(Option<Vec<String>>) }   // NEW variant
pub struct AddTagRequest {                                     // gains predef_values
    pub name: String,
    pub tag_type: String,          // "multi"
    pub default_value: Option<String>,
    pub predef_values: Option<Vec<String>>,   // NEW — the bounded vocabulary
}
pub struct TagElement { /* … */ pub predef_values: Option<Vec<String>> } // NEW
```

| Surface exercised            | Wire effect                                                        |
|------------------------------|-------------------------------------------------------------------|
| `create_tag` (`tag_type="multi"`, `predef_values=Some(V)`) | defines a `Multi` vocabulary  |
| `create_item` with `EnumTagValue::Multi(Some(set))`        | writes a set into `value_multi`    |
| `get_item(item_id)`          | reads the stored set back for canonicalization assertions         |
| `search_item(Some(filter))`  | runs one or more `ANY`/`ALL`/`NONE`/`IS` clauses end-to-end        |

# Data sets (pre-initialisation)

To keep the future `.rs` compact and fast, the seeded fixtures are defined
**once** here as numbered data sets and referenced by number from each test's
*Given*. Each data set is seeded **once per run** by a dedicated helper
(`seed_dataset_1`, …) that logs in, creates its tags, creates its items, and
returns their handles (tag names + a letter→`item_id` map); every test that
references the data set reuses that single seeding.

Isolation rule: **each data set draws its own random tag names**
(`generate_random_tag()`, "to avoid cross-run collisions on the shared customer
schema"). So data set 1's `color` tag is a different tag from data set 2's
`color` tag; a filter written for a data set uses that data set's tag names and
therefore only ever matches that data set's items. Item letters are unique
across the whole spec (`A`–`I`, `Z` in DS1; `J`–`N` in DS2; `P`–`S` in DS3), so
a letter names exactly one item everywhere below.

Vocabularies:
- `V_color  = ["BLUE", "GOLD", "GREEN", "RED", "YELLOW"]`
- `V_module = ["BACKEND", "FRONTEND"]`

## Data set 1 — Colours (one `Multi` tag `<color>` over `V_color`)

Covers the colour literal, wildcard, and boundary cases. Item `Z` is created
**without** a `<color>` tag row (the "untagged" control).

| Item | `<color>` set              |
|------|----------------------------|
| A    | `{BLUE, RED}`              |
| B    | `{BLUE, YELLOW}`           |
| C    | `{}` (tagged, empty set)   |
| D    | `{GREEN}`                  |
| E    | `{BLUE, GREEN, RED}`       |
| F    | `{RED}`                    |
| G    | `{GOLD}`                   |
| H    | `{GREEN, RED}`             |
| I    | `{GOLD, RED}`              |
| Z    | *no `<color>` tag row*     |

## Data set 2 — Colours + Modules (two `Multi` tags + one scalar)

Tags: `<color>` (`Multi`, `V_color`), `<module>` (`Multi`, `V_module`), and a
scalar `<prio>` (`Int`). Covers the module nominal cases and the composition
cases.

| Item | `<color>` set   | `<module>` set          | `<prio>` |
|------|-----------------|-------------------------|----------|
| J    | `{RED}`         | `{BACKEND}`             | `25`     |
| K    | `{RED}`         | `{FRONTEND}`            | `25`     |
| L    | `{BLUE}`        | `{BACKEND}`             | `10`     |
| M    | `{GREEN, RED}`  | `{BACKEND, FRONTEND}`   | `30`     |
| N    | `{}`            | `{FRONTEND}`            | `15`     |

## Data set 3 — Write / canonicalization (one `Multi` tag `<color>`)

Items written with deliberately unsorted / duplicated inputs, to assert the
stored (read-back) canonical form.

| Item | Input set written        | Expected stored set |
|------|--------------------------|---------------------|
| P    | `[]`                     | `{}`                |
| Q    | `["GREEN"]`              | `{GREEN}`           |
| R    | `["RED", "BLUE", "RED"]` | `{BLUE, RED}`       |
| S    | `["RED", "GREEN"]`       | `{GREEN, RED}`      |

Error-path tests that must **create** a failing tag/item (TC-F0007-020, -021,
-025) are self-contained: they build a fresh random tag inline rather than reuse
a shared data set, because the create/write itself is the assertion. Filter-
rejection tests (TC-F0007-022 … -024) reference data set 1 read-only.

# Test cases

Groups: **A** colour literals (001–005), **B** colour wildcards (006–009),
**C** module tag (010–011), **D** composition (012–013), **E** write /
canonicalization (014–015), **F** boundary inputs (016–019), **G** errors and
misuse (020–025). A *Then* lists the expected partition as `returns {…}` /
`excludes {…}` by item letter. Every accepted-search case also carries the
implicit assertion that `search_item` returns `Ok(_)` (a `5xx` means malformed
SQL).

---

## TC-F0007-001 — `ANY` with literals: overlap

**Given** data set 1.
**DFS filter**: `(<color> ANY [GREEN, RED])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{A, D, E, F, H, I}` — every item overlapping `{GREEN, RED}` — proves
   `value_multi && ARRAY['GREEN','RED']` selects any overlap (F0007 rule 6, `ANY`).
2. excludes `{B, C, G, Z}` — `B`/`G` disjoint, `C` empty overlaps nothing, `Z`
   untagged has no row.

## TC-F0007-002 — `ALL` with literals: superset

**Given** data set 1.
**DFS filter**: `(<color> ALL [GREEN, RED])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{E, H}` — the only items holding **both** `GREEN` and `RED` — proves
   the chained `value_multi && ARRAY['GREEN'] AND value_multi && ARRAY['RED']`
   (F0007 rule 6, `ALL`).
2. excludes `{A, B, C, D, F, G, I, Z}` — each missing `GREEN` or `RED` (or
   untagged).

## TC-F0007-003 — `NONE` with literals: disjoint (empty set in, untagged out)

**Given** data set 1.
**DFS filter**: `(<color> NONE [GREEN, RED])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{B, C, G}` — disjoint from `{GREEN, RED}`, the tagged empty set `C`
   included — proves `NOT (value_multi && ARRAY['GREEN','RED'])` (F0007 rule 6,
   `NONE`).
2. excludes `{A, D, E, F, H, I}` — each overlaps the requested set.
3. excludes `Z` — an item that never carries `<color>` is returned by no `NONE`
   clause (join-per-tag → no row → `value IS NOT NULL` false, inherited F0003).
   This pins "empty set vs untagged": `C` (`{}`, a row) matches, `Z` (no row)
   does not.

## TC-F0007-004 — `IS` exact equality

**Given** data set 1.
**DFS filter**: `(<color> IS [GREEN, RED])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{H}` only — the sole item whose set equals `{GREEN, RED}` — proves
   `value_multi = ARRAY['GREEN','RED']` (both sides canonical) is exact-set
   equality (F0007 rule 6, `IS`).
2. excludes `{E}` (superset) and `{D}` (subset) among others — neither matches.

## TC-F0007-005 — `IS []` selects items whose stored set is empty

**Given** data set 1.
**DFS filter**: `(<color> IS [])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{C}` — the tagged item whose stored set is `{}` — proves
   `value_multi = ARRAY[]::text[]` (F0007 rule 6, `IS`; example row `(color IS [])`).
2. excludes `Z` — proves `IS []` matches the tagged **empty set**, not an
   **untagged** item; there is no `Multi` filter that selects never-tagged items
   (see the Coverage-goal note).
3. excludes every non-empty item.

---

## TC-F0007-006 — `ANY` with a `%` wildcard element

**Given** data set 1. `G%` expands against `V_color` to `{GOLD, GREEN}`, so the
requested set is `{GOLD, GREEN, RED}`.
**DFS filter**: `(<color> ANY [G%, RED])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{A, D, E, F, G, H, I}` — everything overlapping `{GOLD, GREEN, RED}`
   — proves `G%` is expanded and unioned with `RED` into one overlap set (F0007
   rule 5 + rule 6, `ANY`).
2. excludes `{B, C, Z}` — disjoint / empty / untagged.

## TC-F0007-007 — `ALL` with a `%` wildcard element

**Given** data set 1. The clause is
`value_multi && ARRAY['GOLD','GREEN'] AND value_multi && ARRAY['RED']` — hold at
least one G-colour **and** hold `RED`.
**DFS filter**: `(<color> ALL [G%, RED])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{E, H, I}` — `E`/`H` via `GREEN`+`RED`, `I` via `GOLD`+`RED` — proves
   each element keeps its own `&&` term and a wildcard element is satisfied by
   any one of its expansions (`I` proves `GOLD` satisfies `G%`) (F0007 rule 6,
   `ALL`).
2. excludes `{A, D, F, G, …}` — each missing a G-colour or `RED`.

## TC-F0007-008 — `NONE` with a `%` wildcard element

**Given** data set 1. `G%` → `{GOLD, GREEN}`, clause
`NOT (value_multi && ARRAY['GOLD','GREEN'])`.
**DFS filter**: `(<color> NONE [G%])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{A, B, C, F}` — no G-colour, the empty set `C` included — proves the
   negated overlap over the expanded set (F0007 rule 5 + rule 6, `NONE`).
2. excludes `{D, E, G, H, I}` (each holds `GOLD` or `GREEN`) and `Z` (untagged).

## TC-F0007-009 — `%` in different positions expands correctly

**Given** data set 1. Two filters over the same items prove `%` as prefix and
suffix:
- `G%` (prefix) → `{GOLD, GREEN}`;
- `%N` (suffix) → `{GREEN}` (the only vocabulary value ending in `N`).

**When**: two calls,
`search_item(Some("(<color> ANY [G%])"), &session_id)` and
`search_item(Some("(<color> ANY [%N])"), &session_id)`.

**Then**:
1. `ANY [G%]` returns `{D, E, G, H, I}` — proves prefix `%` expansion.
2. `ANY [%N]` returns `{D, E, H}` — proves suffix `%` expansion, anchored to the
   whole vocabulary value, not a substring of the item set (F0007 rule 5).

---

## TC-F0007-010 — `ANY` on the `module` tag

**Given** data set 2.
**DFS filter**: `(<module> ANY [BACKEND])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{J, L, M}` — items whose `<module>` set contains `BACKEND` — proves
   the `Multi` predicate works on a **second, independently-defined** vocabulary
   (F0007 rule 6, `ANY`).
2. excludes `{K, N}` — `FRONTEND`-only sets do not overlap `{BACKEND}`.

## TC-F0007-011 — `IS` on the `module` tag (two-value exact set)

**Given** data set 2.
**DFS filter**: `(<module> IS [BACKEND, FRONTEND])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{M}` only — the sole item whose `<module>` set is exactly
   `{BACKEND, FRONTEND}` — proves canonical exact-set equality on the module
   vocabulary (F0007 rule 6, `IS`).
2. excludes `{J, K, L, N}` — single-value or `FRONTEND`-only sets.

---

## TC-F0007-012 — Two `Multi` tags combined under `AND`

**Given** data set 2.
**DFS filter**: `(<color> ANY [RED]) AND (<module> ANY [BACKEND])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{J, M}` — `RED` in `<color>` **and** `BACKEND` in `<module>` —
   proves two `Multi` clauses coexist, each on its own JOIN alias (F0007 rule 6).
2. excludes `K` (module `FRONTEND`), `L` (colour `BLUE`), `N` (empty colour) —
   proves neither clause leaks into the other; a `5xx` would mean the composed
   SQL is malformed.

## TC-F0007-013 — A `Multi` tag combined with a scalar (`Int`) tag

**Given** data set 2.
**DFS filter**: `(<color> ANY [RED]) AND (<prio> >= 18)`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{J, K, M}` — `RED` in `<color>` and `<prio> ≥ 18` — proves a `Multi`
   overlap clause and a scalar `Int` clause coexist (F0007 rules 6, 8); the
   `Multi` addition does not disturb the existing scalar path.
2. excludes `L` (colour `BLUE`) and `N` (empty colour); `<prio>` alone would
   have kept `L` (`10 < 18` excludes it anyway) — the point is both branches are
   honoured independently.

---

## TC-F0007-014 — Write / read round-trip: empty, single, sorted + deduped

**Given** data set 3 items `P`, `Q`, `R`.
**When**: for each, `get_item(item_id, &session_id)` and read
`properties[…].value` as `EnumTagValue::Multi(Some(set))`.

**Then**:
1. `P` reads back `{}`, `Q` reads back `{GREEN}` — proves empty and single-value
   sets persist (F0007 rules 2–3).
2. `R` (input `["RED","BLUE","RED"]`) reads back `{BLUE, RED}` — proves the write
   path sorts ascending and dedups before storing `value_multi` (F0007 rule 3).
   This canonical form is what makes `IS` a plain `=`.

## TC-F0007-015 — `IS` is order-independent (canonical on both sides)

**Given** data set 3 item `S` (stored `{GREEN, RED}`).
**When**: two calls,
`search_item(Some("(<color> IS [GREEN, RED])"), &session_id)` and
`search_item(Some("(<color> IS [RED, GREEN])"), &session_id)`.

**Then**:
1. both calls return `{S}` — proves the filter list is canonicalized before
   emission, so `IS` is insensitive to element order on both stored value and
   filter (F0007 rules 3 and 6).

---

## TC-F0007-016 — Wildcard expands to `∅` (`ANY [Z%]`): empty overlap → no rows

**Given** data set 1. `Z%` matches nothing in `V_color`.
**DFS filter**: `(<color> ANY [Z%])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{}` (none of `A`–`I`) with `Ok(_)` — proves the empty expansion emits
   `value_multi && ARRAY[]::text[]`, which PostgreSQL evaluates to `false`
   (F0007 rule 7, "Pattern(s) expand to `∅`"). No clause is dropped; a concrete
   (unsatisfiable) predicate is emitted and `Ok(_)` proves it is valid SQL.

## TC-F0007-017 — Wildcard expands to `∅` (`NONE [Z%]`): negated empty overlap → all tagged

**Given** data set 1.
**DFS filter**: `(<color> NONE [Z%])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{A, B, C, D, E, F, G, H, I}` — every item carrying `<color>`, the
   empty set `C` included — proves `NOT (value_multi && ARRAY[]::text[])` is
   `NOT false` = true for every tag row (F0007 rule 7).
2. excludes `Z` — match-all covers **tagged** items only, never a never-tagged
   item (cross-check TC-F0007-003).

## TC-F0007-018 — `ANY [%]`: full vocabulary matches non-empty, excludes `{}`

**Given** data set 1. `%` expands to the full vocabulary `V_color`.
**DFS filter**: `(<color> ANY [%])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `{A, B, D, E, F, G, H, I}` — every non-empty set — proves `ANY [%]`
   emits the ordinary overlap `value_multi && ARRAY[<all V_color>]` (not a
   dropped clause) (F0007 rule 7, "`[%]` → full vocabulary").
2. excludes `C` (empty set overlaps nothing) and `Z` (untagged).

## TC-F0007-019 — Literal empty list `[]` — behaviour per operator

**Given** data set 1 (using `C` as the empty-set item and, e.g., `F` `{RED}` as
a non-empty representative).
**When**: four calls,
`search_item(Some("(<color> ANY [])"), &session_id)`,
`search_item(Some("(<color> NONE [])"), &session_id)`,
`search_item(Some("(<color> ALL [])"), &session_id)`, and
`search_item(Some("(<color> IS [])"), &session_id)`.

**Then**:
1. `ANY []` returns `{C}` (the empty set), excludes every non-empty item —
   proves the literal empty list emits `value_multi = ARRAY[]::text[]` (F0007
   rule 7, "Literal list `[]`", `ANY`). Deliberate special case: `[]` names the
   empty set, unlike `[Z%]` in TC-F0007-016.
2. `NONE []` returns `{A, B, D, E, F, G, H, I}` (non-empty), excludes `C` —
   proves `value_multi <> ARRAY[]::text[]`, the complement of `ANY []`.
3. `ALL []` returns `{A, B, C, D, E, F, G, H, I}` (every tagged item, `C`
   included) — proves `ALL []` emits `TRUE`, vacuously satisfied by every tag
   row.
4. `IS []` returns `{C}` — identical to `ANY []`, proving the two spellings are
   equivalent for the empty set (cross-check TC-F0007-005).
5. all four exclude `Z` (untagged).

---

## TC-F0007-020 — Define a `Multi` tag with empty/missing vocabulary → rejected

**Given** no data set (self-contained).
**When**: `create_tag` with `tag_type = "multi"` and `predef_values = Some(vec![])`,
then again with `predef_values = None`.

**Then**:
1. both calls return `Err(_)` carrying `BAD_TAG_DEFINITION` — proves a `Multi`
   tag must declare a non-empty vocabulary (F0007 rule 1; errors table, `add_tag`).

## TC-F0007-021 — Write a value outside the vocabulary → rejected

**Given** a fresh `<color>` `Multi` tag over `V_color` (self-contained).
**When**: `create_item` with `EnumTagValue::Multi(Some(vec!["RED", "PURPLE"]))`
(`PURPLE ∉ V_color`).

**Then**:
1. returns `Err(_)` carrying `BAD_TAG_FOR_ITEM` — proves the write path validates
   every element against `predef_values` before storing (F0007 rule 3; errors
   table, `create_item_property`).

## TC-F0007-022 — `IS` with a wildcard element → rejected

**Given** data set 1 (read-only).
**DFS filter**: `(<color> IS [G%])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `Err(_)` carrying `INVALID_FILTER_FOR_TAG_TYPE` — proves `IS` rejects
   wildcards; it is exact-set equality only (F0007 rule 5; errors table).

## TC-F0007-023 — Unknown literal (no wildcard) in a list → rejected

**Given** data set 1 (read-only).
**DFS filter**: `(<color> ANY [PURPLE])` (`PURPLE` is a plain literal absent from
`V_color`).
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `Err(_)` carrying `UNKNOWN_PREDEF_VALUE` — proves a non-wildcard
   literal must exist in the vocabulary (F0007 rule 5; errors table). Contrast
   with TC-F0007-016, where a *wildcard* matching nothing is accepted and simply
   emits an unsatisfiable overlap (0 rows), not an error.

## TC-F0007-024 — Scalar operator on a `Multi` tag → rejected

**Given** data set 1 (read-only).
**DFS filter**: `(<color> == "RED")`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `Err(_)` — operator not allowed for `Multi` — proves `Multi` rejects
   every non-set operator (`LIKE` likewise) (F0007 rule 4; errors table,
   `verify_filter_conditions`).

## TC-F0007-025 — Set operator on a non-`Multi` tag → rejected

**Given** a fresh text tag `<text>` with one item valued `"RED"` (self-contained).
**DFS filter**: `(<text> ANY [RED])`.
**When**: `search_item(Some(&filter), &session_id)`.

**Then**:
1. returns `Err(_)` — `ANY`/`ALL`/`NONE`/`IS` are not allowed for a scalar type —
   proves the operator whitelist is symmetric: set operators are `Multi`-only
   (F0007 rule 4; errors table, `verify_filter_conditions`).

# Test file
All cases live in their own dedicated file
[`doka-api-tests/tests/F0007-tag-multi-value.rs`](../../doka-api-tests/tests/F0007-tag-multi-value.rs),
under the module `f0007_tag_multi_value_tests`, with the `TEST_TO_RUN` list
following the F0003 convention. TC numbering maps to the test functions by tens:
`TC-F0007-001 ↔ t10_…`, `-002 ↔ t20_…`, …, `-025 ↔ t250_…`. Each test opens with
`Lookup::new("<exact fn name>", TEST_TO_RUN)` and ends with
`lookup.close(); Ok(())`.

The data sets are seeded once per run by dedicated helpers, each returning the
handles its referencing tests need:

```rust
// One seeding per data set, shared by all tests that reference it.
// Random tag names per data set keep the fixtures isolated on the shared schema.
fn seed_dataset_1(ds: &DocumentServerClient, sid: &str) -> Dataset1;  // <color> + items A..I, Z
fn seed_dataset_2(ds: &DocumentServerClient, sid: &str) -> Dataset2;  // <color>,<module>,<prio> + J..N
fn seed_dataset_3(ds: &DocumentServerClient, sid: &str) -> Dataset3;  // <color> + P..S

struct Dataset1 { color_tag: String, items: BTreeMap<char, i64> }     // 'A'..='I' (+ 'Z' untagged id)
// Dataset2 { color_tag, module_tag, prio_tag, items }, Dataset3 { color_tag, items }

// Supporting helpers, mirroring the F0004 pattern:
fn create_multi_tag(ds: &DocumentServerClient, sid: &str, name: &str, predef: &[&str])
    -> Result<AddTagReply, ApiError<'static>>;                 // tag_type = "multi"
fn multi_prop(tag_name: &str, values: &[&str]) -> AddTagValue; // EnumTagValue::Multi(Some(..))
```

Because the customer schema is shared and searches return every matching item in
it, a data set's own random tag names guarantee a filter only ever matches that
data set's items — so each *Then* can assert an exact partition by item letter.
The `.rs` must be written in lock-step with the F0007 wire-type additions
(`EnumTagValue::Multi`, `AddTagRequest.predef_values`, `TagElement.predef_values`);
until those land, this spec is the authoritative contract.
