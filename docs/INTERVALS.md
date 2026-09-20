# Exact source intervals

`Interval` is a pure source value representing a nonempty half-open range
`[start, end)`. Its start is an existing signed 64-bit Time value; its end is a
finite Time or unbounded. Finite bounds require `start < end`. No calendar
conversion, host clock read or integer arithmetic is implicit. This profile uses
unchanged protocol 0.18 graph plans and does not add a runtime interval schema.

```weave
function Clip revision "1" (interval input, interval window) returns interval {
  return interval_intersection(param input, param window);
}
apply FromTen from Clip { interval window interval_open(time 10); }
apply Selected from FromTen { interval input interval(time 0, time 20); }
value Start interval_start(value Selected);
value End interval_end(value Selected);
value Touching interval_meets(value Selected, interval(time 20, time 30));
```

Intervals support immutable partial application, explicit unary callback signatures,
pure imported functions and scalar literal substitution. Integer and Time remain
distinct: `interval(0, 10)` fails; use `interval(time 0, time 10)`. A computed
endpoint can supply an existing `time` function argument and hence a `valid_at`
selector. See [the executable example](../examples/intervals.weave).

| Operator | Result and boundary |
| --- | --- |
| `time_equal(Time, Time)`, `time_lt(Time, Time)` | Exact time comparison |
| `interval(Time, Time)` | Checked finite interval |
| `interval_open(Time)` | Checked interval with unbounded end |
| `interval_start(Interval)` | Finite start Time |
| `interval_end(Interval)` | Finite end Time; unbounded end is an error |
| `interval_equal(Interval, Interval)` | Equal bounds, including open ends |
| `interval_contains(Interval, Time)` | Start inclusive, finite end exclusive |
| `interval_before(Interval, Interval)` | Strict precedence: first finite end **less than** second start |
| `interval_meets(Interval, Interval)` | First finite end **equals** second start |
| `interval_overlaps(Interval, Interval)` | Symmetric nonempty intersection, including containment and equality |
| `interval_within(Interval, Interval)` | Set inclusion, including equality |
| `interval_intersection(Interval, Interval)` | Nonempty intersection; disjoint or touching inputs are an error |

`before` distinguishes Allen-style precedence from meeting. `overlaps` here names
the broader symmetric intersection predicate, not Allen's directional proper-overlap
relation. An open first interval never precedes or meets another interval. Finite
intervals may be within an open interval; an open interval cannot be within a finite
one. All operations compare bounds directly without subtraction or overflow, including
`i64::MIN`, `i64::MAX`, and an interval open from `i64::MAX`.

Invalid finite bounds produce `E_INTERVAL_BOUNDS`. A disjoint or touching
intersection produces `E_INTERVAL_EMPTY`; there is no empty Interval value, optional
result or empty graph substitution. Extracting an open end produces
`E_INTERVAL_UNBOUNDED`. Wrong operand types use `E_SCALAR_TYPE`. Unused definitions
are checked for closed-expression failures while unknown parameters remain symbolic.
Errors retain original module spans and application/import traces.

The public Rust `interval::Interval` keeps fields private. `new` and strict
`Deserialize` enforce the same bounds; JSON requires exactly one `start` and one
`end`, rejecting unknown, duplicate or missing fields. Typed compiler output is
`{"type":"interval","value":{"start":10,"end":20}}`. An open end is `null`.
Untyped graph properties and literal attachments receive only the inner canonical
JSON object. Such metadata describes a value; it does not itself impose a graph
validity window or claim runtime interval typing. Existing scalar schemas reject
an Interval leaf rather than erasing its type.

Evaluation uses the existing precharged scalar work/byte/depth limits, with bounded
fixed-size interval values. It emits no runtime command for a scalar-only program.
The formatter preserves the syntax; module content pins still bind exact bytes.

This closes a bounded part of the original L01/L03/L05 scalar and temporal gates.
General graph time-window/sequence operators, calendar literals, runtime interval
schemas, recorded-time corrections/history and full bitemporal semantics remain open.

```sh
weave values examples/intervals.weave
python3 scripts/check_intervals.py --compiler target/debug/weave --engine /path/to/weave-engine
```

Verification at source implementation `09778d0`:157 tests, strict lint/format,11,876-byte executed native/WASM parity and23 prior standalone artifacts unchanged. An exact-commit archive built locked/offline. The orchestrator independently passed368 finite-set/boundary oracle cases and4 expected failures, reviewed strict decoding, and ran actual persistence/restart against the preserved native protocol0.18/store16 binary. `scripts/check_interval_oracle.py` runs against the installed compiler in Linux/macOS/Windows CI. Hosted results are checked separately; full temporal gates remain open.
