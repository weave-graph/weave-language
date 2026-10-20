# Readers, agent-safe compilation and permission-aware diagnostics

Introduced by LB1 ([#5](https://github.com/weave-graph/weave-language/issues/5))
on contract **1.1.0** (engine B1,
[REGISTRY.md](https://github.com/weave-graph/weave-engine/blob/main/docs/REGISTRY.md));
`widen_readers` since contract **1.2.0** (engine B2 write-time propagation).
Source can now say who may read each fact it writes. The compiler lowers that
intent to the contract's `readers` and `reader_groups` fields; the engine
resolves groups and enforces every read and every write. Nothing here grants
authority: a declaration the writer may not make is rejected by the engine.

```weave
write Shared graph "fleet" {
  assert node "unit-1" entity "UNIT-1" space "fleet" property "status" "active" public;
  assert node "note-1" entity "NOTE-1" space "notes" property "text" "bearing noise" reader_groups "maintenance";
  assert edge "about:1" from "note-1" to "unit-1" relation "about" valid 0 until infinity reader_groups "maintenance";
  assert node "memo-1" entity "MEMO-1" space "notes" property "text" "for bob only" readers "bob";
}
write Revise graph "fleet" {
  correct node "note-1" entity "NOTE-1" space "notes" property "text" "bearing replaced" reader_groups "maintenance";
}
```

The complete example is [examples/permissions/restricted_writes.weave](../examples/permissions/restricted_writes.weave).

## Reader clauses

```ebnf
visibility = "public"
           | "readers", string, { ",", string }
           | "reader_groups", string, { ",", string }
           | "widen_readers" ;          (* correct only, contract 1.2.0 *)
```

A visibility clause may follow any record that has readers in the contract:

- in a `write`: `assert`/`correct` of a `node`, `edge` or `embedding`;
- in a `graph` declaration (snapshot): `node`, `edge`, `relation`, `claim` and
  `attachment`.

Clauses mix freely with `property` and `metadata` clauses, in any order.
`readers` and `reader_groups` may both appear; `public` excludes both.

| Clause | Lowers to | Meaning (engine B1) |
|---|---|---|
| `readers "a", "b"` | `readers: ["a", "b"]` | Principals `a` or `b` (for a user, also its active agents) |
| `reader_groups "g"` | `reader_groups: ["g"]` | Active members of group `g`, directly or through nested groups, resolved on each request |
| both | both fields | One disjunction: any listed principal or any member of a listed group |
| `public` | `readers: []`, no `reader_groups` | Everyone who can read the graph |
| none | the same as `public` | Allowed only where no rule below requires a clause |

Entries are lowered in source order. Each is 1 to 512 bytes without control
characters (`E_ID`), a clause lists at most 256 (`E_BUDGET`), and an entry or
clause may not repeat (`E_DUPLICATE`). A group ID written in `readers` is a
literal principal ID and matches no one, exactly as in the engine. The
compiler cannot see the registry, so it does not check that a principal or
group exists; the engine does.

Records with groups need protocol 1.1.0, which every compiled plan declares.
Public records keep the bytes they had before LB1 (`reader_groups` is omitted
when empty), so existing plans, revision IDs and fingerprints do not change.

Embedding spaces are immutable public descriptors and take no clause.
Snapshot `context_value` graphs have no clause syntax.

## `correct` must restate readers

`correct` replaces the live version of a fact, **including its readers**. A
correction that omitted readers used to publish the new version of a
restricted fact (the risk logged by LA1 part 2). It is now a compile-time
error:

```weave
write W graph "fleet" {
  correct node "note-1" entity "NOTE-1" space "notes" property "text" "fixed";
  // E_VISIBILITY_REQUIRED: restate 'readers'/'reader_groups', or write 'public'
}
```

Every `correct` of a node, edge or embedding carries a visibility clause:
the readers it keeps, or `public` to publish the new version on purpose. The
compiler cannot read the stored version, so it cannot copy its readers; the
clause states the intent explicitly. `assert` (which never replaces a live
fact) and `retract` (which writes no new version) are unchanged.

## Widening a correction: `widen_readers`

Since contract 1.2.0 the engine also refuses a correction that would make a
fact readable by anyone its live version does not admit: the new clause is
public while the old one is not, or it lists a principal or group the old one
does not. The comparison is of the lists, never of group membership, so
adding a member of a listed group is still a widening. Such a correction
fails with `E_READERS_WIDENED`, unless it says so:

```weave
write Publish graph "fleet" {
  correct node "note-1" entity "NOTE-1" space "notes" property "text" "released" public widen_readers;
}
```

`widen_readers` lowers to `"widen_readers": true` on the `correct` change
(omitted otherwise). It is intent, not a reader statement: the correction
still states its readers or `public` (`E_VISIBILITY_REQUIRED`). It applies
only to `correct`; on an `assert` or a snapshot record, which have no
earlier version to widen, it is `E_VISIBILITY`. Restating or narrowing
readers needs no flag. The compiler cannot see the stored readers, so it
does not decide whether a correction widens; the engine does, and the flag
only permits it. Under the untrusted policy the flag is rejected at compile
time (`E_READERS_WIDENED`, at the clause).

Snapshot replacement (`graph G replace revision "r" { ... }`) restates whole
snapshots and keeps its earlier syntax; under the trusted default a record
without a clause is public. Use the untrusted policy below to require a clause
on every record.

## Compile policies for untrusted source

Source supplied by an agent or a user is data until the host decides to run
its plan. Compilation is already pure (no I/O, no store reads, no recorded
time, no authority), and three opt-in policies add compile-time rules. They
only ever reject more; the plan of a source that passes is byte-for-byte the
plan without the policy.

| Policy | CLI | SDK `policy` member | Rule |
|---|---|---|---|
| untrusted | `--untrusted` | `"untrusted": true` | Every written record states its readers (`E_VISIBILITY_REQUIRED` at the record), so nothing becomes public by omission. `widen_readers` is rejected (`E_READERS_WIDENED`), so untrusted source can restate or narrow readers but never publish wider. `context_value` (an always-public snapshot) and host view and handler templates (`E_UNTRUSTED_ARTIFACT`) are rejected |
| read-only | `--read-only` | `"read_only": true` | No `write`, `graph`, `transaction` or `context_value` declarations and no host templates (`E_EFFECT_DENIED` at the declaration); the emitted plan is also checked to contain no `commit`, `commit_batch` or `write` command |
| visible | `--visible MANIFEST.json` | `"visible": {...}` | Graph, group and space references are checked against what one principal can see (next section) |

```sh
weave plan query.weave --untrusted --read-only --visible manifest.json
```

The SDK request (`weave-compiler-request/1`) takes an optional `policy`
object with exactly those members; see [COMPILER_SDK.md](COMPILER_SDK.md).
Native and WASM responses are byte-identical.

### Plan effects

`weave effects FILE` (and `weave_language::policy::plan_effects`) lists every
graph each command of the compiled plan reads, references or writes:

```json
{"version": "1.1.0", "commands": 3, "writes": 3, "effects": [
  {"command": 0, "op": "write", "access": "write", "graph_id": "fleet", "branch_id": "main"}, ...]}
```

`access` is `write` (a commit or fact write target), `read` (any graph a
query, traversal or evaluation names) or `reference` (a pinned graph named
inside written data, such as a metadata attachment; the engine treats an
unreadable one as an unavailable revision). A host can show or compare this
list before running a plan.

### Data and code

Strings in source are data. They lower to JSON string values only; they never
become plan structure, commands or object keys. `tests/hostile_source.rs`
checks this by fuzzing: it mutates every example and hostile templates
(write declarations, JSON plan fragments, comment and string breakouts,
escapes, Unicode, reserved names) and, for every mutant that compiles,
requires a strict contract round trip, exactly one write command per write
declaration of the source with the same graph and branch, only graphs named
in the source, no hostile text in object keys or `op` values, and that the
read-only policy admits the mutant only if it writes nothing. CI runs 20,000
mutants (`WEAVE_FUZZ_ITERATIONS` raises it; 200,000 pass locally).

## Visible manifests and permission-aware diagnostics

A **visible manifest** (`weave-visible/1`) states what one principal can see
and name. The host supplies it; the compiler never reads a store.

```json
{"format": "weave-visible/1", "principal": "alice-bot",
 "graphs": [{"graph_id": "fleet", "permission": "write", "branches": ["main"],
             "embedding_spaces": [{"id": "notes", "revision": "1", "encoder": "enc", "encoder_version": "1",
                                   "preprocessing": "words/v1", "dimensions": 3, "metric": "cosine_distance"}]}],
 "reader_groups": ["maintenance"]}
```

| Member | Meaning |
|---|---|
| `principal` | The calling principal (for messages only) |
| `graphs` | Every graph it can discover, with its permission (`discover`, `read`, `write`, `administer`), branches, and optionally its `schema` and the `embedding_spaces` it can see (absent means unknown, `[]` means none) |
| `reader_groups` | The groups it may name in `reader_groups` |

Decoding is strict (`E_MANIFEST` for an unknown member, format, permission or
a duplicate; `E_BUDGET` above 10,000 graphs, 1,024 groups, 1,024 spaces per
graph or 8 MiB). With a manifest, the compiler reports the engine's own codes
at compile time, at the exact source span:

| Code | Cause | Span |
|---|---|---|
| `E_UNAVAILABLE` | A graph not in the manifest (missing and hidden graphs give the same message) | The `use`, `write` or `graph` name, or the graph ID literal |
| `E_FORBIDDEN` | A graph the principal can discover but not read, or read but not write | As above |
| `E_UNAVAILABLE` | A `reader_groups` entry the principal may not name (unknown, hidden or retired groups give the same message) | The group string |
| `E_SPACE_MISMATCH` | A declared space whose `id@revision` is in the graph with a different descriptor; the message states the stored one | The space name |
| `E_UNKNOWN_SPACE` | A declared space the principal cannot see in that graph | The space name |

Every graph the emitted plan reads or writes is checked too, so operators
without a dedicated rule (for example `recorded_handle`) are covered. A
manifest is advisory: the engine authorizes the plan again when it runs, and
membership can change between compilation and execution.

**Where manifests come from.** Engine B4 serves the manifest of the
authenticated caller: `GET /v1/manifest` on `weave-service` returns
`weave-visible/1` with its discoverable graphs, permissions and branches and
exactly the `reader_groups` a write by it may name. It omits embedding spaces
and schemas; the MCP tool `describe_schema` with a `graph_id` returns each
visible space descriptor under `embedding_spaces[].space`, which a host copies
into that graph's `embedding_spaces`. `scripts/check_agent_service.py` builds
the manifest this way.

Without the service, a host assembles it: `weave-engine graphs` lists the
graphs and permissions, a compiled `changes C from G { since as_of 0; }` run
as the principal returns the facts and embedding spaces it can see, and a
user's groups are its `weave:member` edges in `weave:registry`. An agent
cannot read `weave:registry` unless delegated, so the host that registered it
supplies the groups it delegated. `weave-engine graphs` omits `weave:`
graphs; a manifest lists `weave:registry` (with `read`) only if the source
should be allowed to query it.

**Compiling in a sandboxed host.** A host that cannot link the compiler runs
`weave sdk` ([COMPILER_SDK.md](COMPILER_SDK.md#subprocess-mode-weave-sdk)):
one request with its `policy` (including the `visible` manifest) on stdin,
one response on stdout, stable exit codes, no other I/O. `--untrusted` and
`--read-only` on its command line set a floor the request cannot lower.

## End-to-end acceptance

The scripts need an engine built from engine `main` (B1 for the first two,
B4's `weave-service` for the third) and enable its registry:

```sh
python3 scripts/check_permissions.py --compiler target/debug/weave --engine ../weave-engine/target/debug/weave-engine
python3 scripts/check_agent_repair.py --compiler target/debug/weave --engine ../weave-engine/target/debug/weave-engine
python3 scripts/check_agent_service.py --compiler target/debug/weave --service ../weave-engine/target/debug/weave-service
```

- `check_permissions.py` (contract 1.2.0): a source correction that widens
  readers fails with `E_READERS_WIDENED` (public, or an added principal, even
  a group member), passes with `widen_readers`, which `--untrusted` rejects,
  and a narrowing one needs no flag; a public fact citing restricted evidence
  fails with `E_DERIVED_READERS` and passes when restricted like it. Also:
  group-restricted source writes, corrections and
  snapshot records are visible to members and hidden from a non-member who
  can read the graph (lens, `expand`, `changes`, `nearest`); literal readers
  and a group ID in `readers` behave as in the engine; a correction without
  readers does not compile; the engine rejects a hidden or unknown group with
  one identical error, rejects an agent naming a group it does not exercise,
  and stamps an allowed agent write `on_behalf_of` its owner; the manifest
  rejects the same over-reach at compile time; revoking membership hides
  group facts on the next request.
- `check_agent_repair.py`: a registered agent with random graph, space,
  dimension and property names discovers its graphs and their embedding
  space, writes a `nearest` query with the wrong vector size, repairs it from
  the `E_SPACE_MISMATCH` diagnostic alone and runs it (hits equal an
  independent ranking of the embeddings it may see), then records a finding:
  rejected as implicitly public under `--untrusted`, repaired with a group
  from its manifest, hidden from a non-member. Guessed graphs and groups fail
  at compile time with `E_UNAVAILABLE`.
- `check_agent_service.py`: the same agent scenario on the authenticated
  service, with nothing assembled by the host. The agent authenticates with
  its own agent token, reads its manifest from `GET /v1/manifest` and its
  spaces and property keys from `describe_schema`, compiles every source with
  `weave sdk` (request `policy` `{untrusted, read_only, visible}`, run with an
  empty environment in an empty directory), runs every plan with
  `POST /v1/execute`, and records its finding under a record ID minted by
  `POST /v1/ids`. The host loads its fixtures under IDs it mints the same way,
  because the service requires minted IDs from every principal, tenant
  administrators included (engine #76).

## Diagnostics

| Code | Cause | Span |
|---|---|---|
| `E_VISIBILITY_REQUIRED` | `correct` without a clause (`widen_readers` alone is not one); under `--untrusted`, any written record without one, or `context_value` | The change, the record ID or the declaration name |
| `E_READERS_WIDENED` | `widen_readers` under `--untrusted` (the engine reports the same code for a widening correction without the flag) | The clause |
| `E_VISIBILITY` | `public` together with `readers` or `reader_groups`; `widen_readers` outside a `correct` | The later clause keyword, or `widen_readers` |
| `E_DUPLICATE` | A repeated clause, or a repeated entry in one clause | The keyword or the entry |
| `E_ID` | An empty entry, one above 512 bytes, or one with a control character | The entry |
| `E_BUDGET` | More than 256 entries in one clause | The 257th entry |
| `E_EFFECT_DENIED` | A write or host template under `--read-only` | The declaration name |
| `E_UNTRUSTED_ARTIFACT` | A host view or handler template under `--untrusted` | The template name |
| `E_UNAVAILABLE`, `E_FORBIDDEN`, `E_SPACE_MISMATCH`, `E_UNKNOWN_SPACE`, `E_MANIFEST` | Visible-manifest checks (above) | As above |

## Limits

- The compiler is not a security boundary. It checks intent and gives early
  diagnostics; the engine decides every read and write.
- A manifest is a snapshot: the engine serves it (`GET /v1/manifest`, with
  spaces from `describe_schema`), but membership can change before the plan
  runs, and the engine checks again. The engine's MCP server has no compile
  tool yet; it will run `weave sdk`.
- Readers are literals, not parameters. A `correct` cannot say "keep the
  stored readers": the contract has no such field, so the source restates
  them.
- Snapshot `replace` records may omit a clause except under `--untrusted`.
- Derived results follow the engine's propagation (B2). Its declared
  derivation rule (`E_DERIVED_READERS`: a fact citing restricted provenance
  evidence must be no broader) applies only to claimed `evidence`, which
  source cannot express yet, so compiled plans never trigger it;
  `scripts/check_permissions.py` exercises it with one JSON write.
- Hidden live versions: since 1.2.0 the engine treats a live version the
  writer cannot read as absent, so a source `correct` of a
  fact the writer cannot see fails as for a missing fact, and an `assert` of
  its ID fails with `E_CONFLICT` ("fact ID is not available").
- The manifest's `schema` member is decoded but not yet used to type `use`
  reads.
