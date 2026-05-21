
```yaml
id: IT-F0003
title: Integration tests — Global SQL Search (DFS → item search end-to-end)
type: integration-test
status: draft
target_flow: DOKA-ITEM-SEARCH
related_fix: F0003
naming_prefix: IT-F0003
language: rust
```

# Coverage goal
Behavioural end-to-end validation, **through the public document-server
HTTP API**, that an item search driven by a DFS expression containing
F0001 / F0002 escapes returns the expected items once F0003 routes every
value literal through `to_sql_literal` inside the `unaccent_lower(…)`
wrapper.

What is asserted (black-box, no SQL inspection):

- a `'` inside a value (e.g. `l'élite`) does not break the search —
  proves `'` is doubled inside `unaccent_lower('…')` (F0002 rule 1);
- a `#%` inside a `LIKE` pattern matches **only** the literal `%`
  character, not any sequence — proves `\%` + `ESCAPE '\'` is emitted
  (F0002 rule 3, `Literal` branch);
- a bare `%` in a `LIKE` pattern still matches any sequence — proves
  `AnySequence` is emitted bare (F0002 rule 3);
- a numeric branch (`age >= 18`) co-exists with the text `LIKE` branch
  and is itself unaffected — proves F0003 business rules 2 / 3 do not
  leak across conditions;
- overall query validity — the HTTP call returns `Ok(_)` rather than
  500, indirectly proving the generated SQL parses under PostgreSQL.

# System under test
The **public document-server HTTP API**, not the internal
`generate_search_sql` function. Specifically:

- `document-server` on `localhost:30070`, exercised through
  `doka_cli::request_client::DocumentServerClient::{create_item,
  search_item}`;
- `admin-server` on `localhost:30060` for the login round-trip via
  `AdminServerClient::login`.

No mock, no internal SQL string inspection — the test only observes
what the API returns. The escaping rules of F0001 / F0002 / F0003 are
validated **indirectly**: each behavioural outcome (item matched /
excluded / no HTTP error) maps to a specific escape rule, listed
under *Then* below.

# Test cases

## TC-F0003-001 — Composite expression: `LIKE` with escape rules AND a numeric condition

Given:
- DFS input (built with the randomised tag names below):
  `(<tag_text> LIKE "50 #% de l'élite%") AND (<tag_int> >= 18)`
- Two random tag names for this run (see `generate_random_tag()` in
  `ut60`), to avoid cross-run collisions on the shared customer:
  - `<tag_text>` declared as a text tag;
  - `<tag_int>` declared as an int tag.
- Three items seeded via `document_server.create_item(...)`:

  | Item | `<tag_text>` value          | `<tag_int>` value | Should match |
  |------|-----------------------------|-------------------|--------------|
  | A    | `50 % de l'élite ranking`   | `25`              | yes          |
  | B    | `50X de l'élite ranking`    | `25`              | no — no literal `%` after `50 ` |
  | C    | `50 % de l'élite ranking`   | `15`              | no — `age < 18` |

  Item A's text value packs every escape axis into one string: a
  literal `%` (matched by `#%`), a `'` inside `l'élite` (must be
  doubled in SQL), and a trailing free sequence (matched by the
  trailing wildcard).

Parser-level facts (already verified by IT-F0001, kept here for
traceability):
- canonical form of the AST:
  `([<tag_text><LIKE>50 %\ de l'élite\%\]AND[<tag_int><GTE>18])`
- text condition AST value:
  `ValuePattern([Literal("50 % de l'élite"), AnySequence])`
- int condition AST value: `ValueInt(18)`

When:
- the test logs in via `AdminServerClient::login`, then calls
  `document_server.search_item(Some(&filter), &session_id)?` with the
  DFS filter above.

Then (assertions on the HTTP reply, **not** on the internal SQL):

1. **Item A is returned.**
   `reply.items.iter().any(|it| it.item_id == item_a.item_id)` is
   `true`. This single positive outcome exercises four rules at once:
   - the `'` in `l'élite` was doubled in the generated SQL (F0002
     rule 1) — otherwise the SQL would be malformed and the call
     would have returned an error, not item A;
   - the `#%` was rendered as `\%` with an `ESCAPE '\'` clause (F0002
     rule 3, `Literal` branch) — otherwise A's literal `%` would not
     have matched the pattern;
   - the trailing `%` was rendered as bare `%` (F0002 rule 3,
     `AnySequence` branch) — otherwise the ` ranking` suffix would
     not have matched;
   - the `ESCAPE '\'` clause sits outside the `unaccent_lower(…)`
     call (F0003 business rule 2) — otherwise the SQL would be
     malformed and the call would have errored out.

2. **Item B is excluded.**
   `reply.items.iter().all(|it| it.item_id != item_b.item_id)` is
   `true`. If F0003 had emitted `#%` as a bare wildcard, B's
   `50X de l'élite…` would have matched. The exclusion proves that
   literal-`%` semantics are enforced.

3. **Item C is excluded.**
   `reply.items.iter().all(|it| it.item_id != item_c.item_id)` is
   `true`. Proves the `<tag_int> >= 18` branch is honoured (F0003
   business rule 3) and that the `ESCAPE '\'` clause did not leak
   into the int sub-query (which would have produced a parse error
   surfaced as a 500, not a silent mis-match).

4. **The search did not error out.**
   `search_item` returned `Ok(_)`. This proves the assembled SQL is
   syntactically valid PostgreSQL — every `'` was doubled, every
   literal `%` / `_` was prefixed, and the `ESCAPE '\'` clause was
   correctly attached to the `LIKE`, not nested inside
   `unaccent_lower(…)`.

The test lives in its own dedicated file [`doka-api-tests/tests/F0003-global-sql-search.rs`](../../doka-api-tests/tests/F0003-global-sql-search.rs), and follows the practice 
of doka integration tests.
