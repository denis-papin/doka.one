-- ============================================================================
-- F0007 — Multi tag type: SQL syntax validation harness
--
-- Purpose: prove the FINAL SQL (columns, predicates, GIN index, and the full
-- LEFT-OUTER-JOIN-per-condition query shape) is valid PostgreSQL and returns
-- the right rows — BEFORE writing any of the Rust DSL compilation chain.
--
-- This is a throwaway sandbox: it creates schema `f0007_validate`, seeds a few
-- items, runs one COMPLETE query per operator, and drops everything at the end.
-- Nothing here touches the real `cs_<customer>` schemas.
--
-- Run:  psql -f validate_multi_sql.sql "postgresql://user:pwd@host:5432/db"
--
-- Each query is preceded by the DSL filter it stands for and the item ids it
-- must return. Eyeball the result against the "-- expect:" line.
-- ============================================================================

DROP SCHEMA IF EXISTS f0007_validate CASCADE;
CREATE SCHEMA f0007_validate;
SET search_path TO f0007_validate;

-- --- Minimal shape of the three tables the feature touches ------------------
-- Only the columns relevant to F0007 are modelled.

CREATE TABLE item (
    id bigint PRIMARY KEY
);

CREATE TABLE tag_definition (
    id            bigint PRIMARY KEY,
    "name"        varchar(25) NOT NULL,
    "type"        varchar(25) NOT NULL,
    predef_values text[]                 -- NEW in F0007
);

CREATE TABLE tag_value (
    id          bigint PRIMARY KEY,
    tag_id      bigint NOT NULL,
    item_id     bigint NOT NULL,
    value_multi text[]                   -- NEW in F0007 (NULL for non-Multi tags)
);

-- The GIN index that backs the `&&` / `@>` / `=` array predicates.
CREATE INDEX tag_value_multi_gin_idx ON tag_value USING gin (value_multi);

-- The unique-row invariant that F0007 keeps: one row per (tag, item).
CREATE UNIQUE INDEX tag_value_tag_item_udx ON tag_value USING btree (tag_id, item_id);

-- --- Seed data --------------------------------------------------------------
-- One Multi tag `color` with a 5-value vocabulary.

-- DSL (define the Multi tag + its vocabulary), indicative CLI surface:
--   doka-cli tag create --name color --type multi \
--                       --predef "BLUE,GOLD,GREEN,RED,YELLOW"
INSERT INTO tag_definition (id, "name", "type", predef_values) VALUES
    (1, 'color', 'Multi', ARRAY['BLUE','GOLD','GREEN','RED','YELLOW']);

-- Items and their color set. value_multi is stored ALREADY sorted + deduped,
-- exactly as the write path (Rule 3) will canonicalize it.
-- The DSL column shows the tag as typed on the write side, e.g.
--   doka-cli item tag -id 1 -u "(color:RED,BLUE:multi)"
INSERT INTO item (id) VALUES (1),(2),(3),(4),(5),(6),(7);

INSERT INTO tag_value (id, tag_id, item_id, value_multi) VALUES
    (101, 1, 1, ARRAY['BLUE','RED']),          -- i1  DSL (color:RED,BLUE:multi)   -> {BLUE,RED}
    (102, 1, 2, ARRAY['BLUE','YELLOW']),       -- i2  DSL (color:BLUE,YELLOW:multi)-> {BLUE,YELLOW}
    (103, 1, 3, ARRAY[]::text[]),              -- i3  DSL (color::multi)           -> {}  (empty set)
    (104, 1, 4, ARRAY['BLUE','GREEN','RED']),  -- i4  DSL (color:GREEN,RED,BLUE:multi) -> {BLUE,GREEN,RED}
    (105, 1, 5, ARRAY['GREEN','RED']),         -- i5  DSL (color:GREEN,RED:multi)  -> {GREEN,RED}
    (106, 1, 6, ARRAY['GOLD']);                -- i6  DSL (color:GOLD:multi)       -> {GOLD}
-- i7 has NO color row at all — models an item never given the tag (no DSL write).

-- ============================================================================
-- The generator's per-condition shape (from SEARCH_CONDITION_SQL.md):
--
--   SELECT i.id
--   FROM item i
--   LEFT OUTER JOIN (
--       SELECT tv.item_id, tv.value_multi AS value
--       FROM tag_definition td
--       JOIN tag_value tv ON tv.tag_id = td.id
--                        AND td."name" = 'color'
--                        AND <PREDICATE>
--   ) ot_color_0 ON ot_color_0.item_id = i.id
--   WHERE ot_color_0.value IS NOT NULL
--
-- Only <PREDICATE> changes per operator. Each block below is a COMPLETE query.
-- ============================================================================

\echo '--- ANY [GREEN, RED]  -> expect: 1, 4, 5 ---'
-- DSL filter : (color ANY [GREEN, RED])
-- CLI        : doka-cli item search -f "(color ANY [GREEN, RED])"
-- predicate  : value_multi && ARRAY['GREEN','RED']
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi && ARRAY['GREEN','RED']::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

\echo '--- ALL [GREEN, RED]  -> expect: 4, 5 ---'
-- DSL filter : (color ALL [GREEN, RED])
-- CLI        : doka-cli item search -f "(color ALL [GREEN, RED])"
-- predicate  : value_multi && ARRAY['GREEN'] AND value_multi && ARRAY['RED']
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi && ARRAY['GREEN']::text[]
                     AND tv.value_multi && ARRAY['RED']::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

\echo '--- NONE [GREEN, RED]  -> expect: 2, 3, 6 (tagged-only; i7 absent) ---'
-- DSL filter : (color NONE [GREEN, RED])
-- CLI        : doka-cli item search -f "(color NONE [GREEN, RED])"
-- predicate  : NOT (value_multi && ARRAY['GREEN','RED'])
-- NOTE: i7 has no color row, so the JOIN never sees it and it is NOT returned
-- by NONE (nor by any other Multi clause). This is the defined behaviour: an
-- untagged item (no tag row) is distinct from the tagged empty set i3 (`{}`),
-- which DOES match. See F0007 rule 7 and IT-F0007 TC-003.
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND NOT (tv.value_multi && ARRAY['GREEN','RED']::text[])
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

\echo '--- IS [GREEN, RED]  -> expect: 5 ---'
-- DSL filter : (color IS [GREEN, RED])
-- CLI        : doka-cli item search -f "(color IS [GREEN, RED])"
-- predicate  : value_multi = ARRAY['GREEN','RED']
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi = ARRAY['GREEN','RED']::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

\echo '--- IS []  -> expect: 3 (the tagged empty set {}; NOT the untagged i7) ---'
-- DSL filter : (color IS [])
-- CLI        : doka-cli item search -f "(color IS [])"
-- predicate  : value_multi = ARRAY[]::text[]
-- Careful: `value IS NOT NULL` still holds for an empty array, so i3 is kept.
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi = ARRAY[]::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

\echo '--- ANY [G%, RED]  (G% -> {GOLD,GREEN})  -> expect: 1, 4, 5, 6 ---'
-- DSL filter : (color ANY [G%, RED])           compiled set {GOLD,GREEN,RED}
-- CLI        : doka-cli item search -f "(color ANY [G%, RED])"
-- The wildcard is expanded at COMPILE time; the SQL only ever sees literals.
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi && ARRAY['GOLD','GREEN','RED']::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

\echo '--- ALL [G%, RED]  -> expect: 4, 5 ---'
-- DSL filter : (color ALL [G%, RED])
-- CLI        : doka-cli item search -f "(color ALL [G%, RED])"
-- predicate  : value_multi && ARRAY['GOLD','GREEN'] AND value_multi && ARRAY['RED']
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi && ARRAY['GOLD','GREEN']::text[]
                     AND tv.value_multi && ARRAY['RED']::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

-- ============================================================================
-- Compile-time wildcard expansion, checked in SQL
-- These SELECTs mirror what type_resolver does in Rust: LIKE a predef value.
-- Use them to confirm your Rust expansion agrees with Postgres LIKE.
-- ============================================================================

\echo '--- expand G%  -> expect: {GOLD, GREEN} ---'
-- Fires for any DSL element carrying a wildcard, e.g. the `G%` in
--   (color ANY [G%, RED])   /   (color ALL [G%, RED])   /   (color NONE [G%])
SELECT array_agg(v ORDER BY v) AS expansion
FROM tag_definition td, unnest(td.predef_values) AS v
WHERE td."name" = 'color' AND v LIKE 'G%';

\echo '--- expand Z%  -> expect: NULL / empty (no vocabulary value matches) ---'
-- DSL: (color ANY [Z%]) -> the expansion is empty. No clause is dropped: the
-- resolver still emits a concrete predicate over the empty array (see the
-- boundary-inputs block below), which PostgreSQL evaluates to false.
SELECT array_agg(v ORDER BY v) AS expansion
FROM tag_definition td, unnest(td.predef_values) AS v
WHERE td."name" = 'color' AND v LIKE 'Z%';

-- ============================================================================
-- Boundary inputs — [%] (full vocabulary) and [] (empty-set selector).
-- F0007 rule 7: every clause emits a concrete predicate, nothing is dropped.
-- These prove the two extremes behave differently from a wildcard that merely
-- expands to nothing (Z% above).
-- ============================================================================

\echo '--- ANY [%]  (full vocabulary)  -> expect: 1, 2, 4, 5, 6  (all non-empty; i3 {} excluded) ---'
-- DSL filter : (color ANY [%])   compiled set = full vocabulary V
-- CLI        : doka-cli item search -f "(color ANY [%])"
-- predicate  : value_multi && ARRAY[<all V>]  -> matches every non-empty set
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi && ARRAY['BLUE','GOLD','GREEN','RED','YELLOW']::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

\echo '--- ANY [Z%]  (wildcard matched nothing)  -> expect: (no rows) ---'
-- DSL filter : (color ANY [Z%])   compiled set = {} (from a pattern)
-- predicate  : value_multi && ARRAY[]::text[]  -> always false
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi && ARRAY[]::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

\echo '--- ANY []  (literal empty list = empty-set selector)  -> expect: 3 ---'
-- DSL filter : (color ANY [])   special case: names the empty set (= IS [])
-- CLI        : doka-cli item search -f "(color ANY [])"
-- predicate  : value_multi = ARRAY[]::text[]  -> only the empty-set item
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi = ARRAY[]::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

\echo '--- NONE []  (complement of ANY [])  -> expect: 1, 2, 4, 5, 6  (all non-empty) ---'
-- DSL filter : (color NONE [])   predicate: value_multi <> ARRAY[]::text[]
SELECT i.id
FROM item i
LEFT OUTER JOIN (
    SELECT tv.item_id, tv.value_multi AS value
    FROM tag_definition td
    JOIN tag_value tv ON tv.tag_id = td.id
                     AND td."name" = 'color'
                     AND tv.value_multi <> ARRAY[]::text[]
) ot_color_0 ON ot_color_0.item_id = i.id
WHERE ot_color_0.value IS NOT NULL
ORDER BY i.id;

-- ============================================================================
-- Optional: confirm the GIN index is usable by the overlap predicate.
-- On this tiny table Postgres will still pick a seq scan; use a larger seed
-- or `SET enable_seqscan = off;` to force the index and eyeball the plan.
-- ============================================================================

\echo '--- EXPLAIN of the ANY overlap predicate (index-usability sanity) ---'
SET enable_seqscan = off;
EXPLAIN
SELECT tv.item_id
FROM tag_value tv
WHERE tv.value_multi && ARRAY['GREEN','RED']::text[];
SET enable_seqscan = on;

-- --- Teardown ---------------------------------------------------------------
DROP SCHEMA f0007_validate CASCADE;
