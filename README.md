# sampling-params-matrix

A small Rust library and CLI that answers one question: when a model is in a thinking mode, should
my request send `temperature`?

The answer differs per provider and per model. For some, temperature still works. For some, it is
accepted and does nothing. For some, sending it fails the request with a 400. This crate holds that
as a table, turns it into a send-or-drop decision, and produces a record you can store with the
response so a reader can see when the setting did nothing.

No dependencies. Rust 2021 edition.

## The four classes

| class | meaning | the plan |
|---|---|---|
| `honored` | temperature still changes the answer while thinking | send it |
| `ignored` | accepted, no effect while thinking | leave it out, record `ignored_while_thinking` |
| `rejected` | sending it fails the request (400) | leave it out, record `rejected_while_thinking` |
| `no_thinking` | no thinking mode, so temperature is always live | send it |

`ignored` is the dangerous one. The request succeeds either way, so nothing on the wire tells you the
setting was dead. That is why the plan records `requested` next to `send` and `not_applied`.

## Quick start

```sh
cargo build --release
B=target/release/sampling-params-matrix

# A model with a thinking switch you control (thinking on, then off)
$B deepseek-flash --builtin --thinking on  --temp 0.7
$B deepseek-flash --builtin --thinking off --temp 0.7

# A user-supplied OpenAI-compatible endpoint
$B o3     --endpoint https://api.openai.com/v1 --temp 0.7
$B gpt-4.1 --endpoint https://api.openai.com/v1 --temp 0.7
```

Output is one JSON line, for example:

```json
{"send":false,"not_applied":"rejected_while_thinking","thinking_temperature":"rejected","source":"https://platform.openai.com/docs/guides/reasoning","requested":0.7}
```

As a library:

```rust
use sampling_params_matrix::{temperature_plan, Target};

let plan = temperature_plan("o3", Target::Endpoint { base_url: "https://api.openai.com/v1" });
assert_eq!(plan.apply(0.7), None);          // leave the field out of the request body
let record = plan.to_json(0.7);             // store this with the response
```

## How it works

- `endpoint_capability(base_url, model)` matches the host, then the model name, and returns a class
  plus a documentation URL. A host with no documentation page is `honored` with the source labelled
  `unknown, endpoint's own behaviour`.
- `builtin_capability(model)` covers a model whose thinking switch you control.
- `temperature_plan(model, target)` combines the class with whether thinking is on. For a
  user-supplied endpoint no thinking switch is sent, so a thinking model counts as thinking.
- Routers (OpenRouter) drop parameters an upstream does not support, so a model that would return 400
  on its own API is `ignored` there instead of `rejected`.

## Limits

- The matrix is hand-written from provider documentation dated 2026-11-15 and partly re-checked on
  2026-11-18. Providers change these rules. Treat every row as having an expiry date.
- It does not detect behaviour. `ignored` cannot be told from `honored` with one request, and a probe
  would cost a paid call per check. The table decides.
- Model-name matching is a heuristic (`o1`, `o3`, `o4-mini`, `gpt-5*`; Gemini 3.6 and later). A new
  model name that does not fit falls to the host's default.
- Only temperature is covered. `top_p` and `top_k` are not.
- The built-in table has one model.
- Not every source page was re-checked; see the post for which were.

## Tests

```sh
./scripts/verify.sh
```

It runs build, `cargo test` (11 tests), `cargo clippy -D warnings`, and a CLI smoke check. The last
stdout line is `VERIFY {"verdict":"pass","gates":4}` or a `fail` verdict naming the step.

## License

Apache-2.0. Copyright 2026 Adam Scheurer.
