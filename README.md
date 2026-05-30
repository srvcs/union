# srvcs-union

The set-union service of the srvcs.cloud distributed standard library.

Its single concern: **the union of two sets.** It reads two lists of integers
and returns the sorted list of distinct values appearing in either list.

`srvcs-union` is a **leaf**: it depends on no other service and makes no network
calls. All work is local.

```text
result = sorted distinct values appearing in a or b
union([1, 2], [2, 3]) == [1, 2, 3]
```

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Compute the union of sets `a` and `b` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"a": [1, 2], "b": [2, 3]}'
# {"a":[1,2],"b":[2,3],"result":[1,2,3]}

curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"a": [3, 3, 1], "b": [1, 2, 2]}'
# {"a":[3,3,1],"b":[1,2,2],"result":[1,2,3]}
```

Responses:

- `200 {"a": [...], "b": [...], "result": [...]}` — evaluated. `result` is the
  sorted list of distinct values appearing in `a` or `b`.
- `422 {"error": "a and b must be integers"}` — some element of `a` or `b` is
  not a JSON integer.

Two empty lists yield the empty list. Duplicates collapse, the output is sorted
ascending, and negatives are ordered correctly.

## Dependencies

None. `srvcs-union` is a leaf set service. Because it owns its own validation, it
rejects any non-integer element directly with `422` rather than forwarding to a
dependency.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See [`srvcs/platform`](https://github.com/srvcs/platform) for the shared
standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
