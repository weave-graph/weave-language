# Weave protocol v0.3.0 integration notes

Version 0.3 adds immutable, program-local graph values. The engine continues accepting v0.1/v0.2 operations under their original versions; graph-value commands require v0.3. The compiler now emits v0.3.0. These notes supplement [v0.2 joins](../v0.2/README.md) and [v0.1 storage queries](../v0.1/README.md).

`Bind { name, value }` evaluates a pure graph expression once, returns its graph result, and binds that immutable result for subsequent commands in the same program. Duplicate bindings and forward/unknown references are errors. A value name is **not a persistent graph identity**. Binding, filtering and joining do not create revisions or commit events.

`Evaluate { value }` returns an expression's result without creating a name. The source compiler uses `Bind` for concrete named lenses and join outputs. Parameterized templates are compiler objects and emit no incomplete runtime evaluation.

`GraphExpression` supports:

- `query { query }`: execute a stored graph query under the trusted host context.
- `reference { name }`: access a previously bound immutable result.
- `filter { input, predicate, valid_at }`: filter a result while preserving its input snapshot, coverage, metadata and evidence restrictions.
- `join { left, right, output_predicate, match_on }`: compose graph expressions with the existing exact entity-and-space path-join semantics.

`QueryResult.edge_origins` maps result edge identities to their exact leaf assertion references. Composed joins flatten this dependency mapping; a synthetic intermediate edge ID never pretends to be an assertion in a persisted revision. `input_snapshots` preserves the complete revision vector. The engine independently enforces authorization for stored sources and conservatively retains restrictions through pure value operations.

Expression depth is bounded at 32 and total expression nodes at 1,000 per engine evaluation. Compilation uses named references to avoid expanding whole subexpressions repeatedly. Runtime expression limits still apply to manually submitted plans.

Metadata already resolved in a value is retained. Adding new metadata traversal to a materialized value is currently unsupported and rejected by the compiler; perform that traversal in the stored query before deriving values. Graph value persistence, higher-order function arguments and full arbitrary relational joins remain later work.
