# Flagship HTTP + SQLite + JSON Example

This example is DUUMBI's first runnable reference program. It builds a bounded
loopback JSON API from a semantic graph, prepares a SQLite data set, reads a row
back from SQLite, constructs a JSON response, and serves it through the local
static-route HTTP server.

The checked-in files are intentionally outside `.duumbi/` so they are tracked by
Git. The setup step below materializes a normal DUUMBI workspace from those
sources.

## Files

- `config.toml` declares the stdlib modules used by the graph.
- `graph/main.jsonld` is the reader-facing graph source.

The graph imports:

- `@duumbi/stdlib-db` for SQLite open/execute/query/cleanup.
- `@duumbi/stdlib-json` for response header JSON parsing.
- `@duumbi/stdlib-server` for a bounded loopback static route.

It also uses core string operations to build the response body from values read
back from SQLite.

## Run

From the repository root:

```sh
cargo build
cd examples/flagship-http-sqlite-json

rm -rf workspace
mkdir workspace
../../target/debug/duumbi init workspace
cp config.toml workspace/.duumbi/config.toml
cp graph/main.jsonld workspace/.duumbi/graph/main.jsonld
cd workspace

../../../target/debug/duumbi deps vendor --all
../../../target/debug/duumbi build --offline
../../../target/debug/duumbi describe

../../../target/debug/duumbi run > /tmp/duumbi-flagship-example.log 2>&1 &
server_pid=$!

for attempt in $(seq 1 50); do
  if curl --fail --silent --show-error --max-time 2 http://127.0.0.1:39388/facts; then
    break
  fi
  if [ "$attempt" -eq 50 ]; then
    echo "server did not become ready" >&2
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" || true
    exit 1
  fi
  sleep 0.1
done

wait "$server_pid"
cat /tmp/duumbi-flagship-example.log
```

Expected response:

```json
{"service":"flagship-http-sqlite-json","route":"/facts","count":1,"first_fact":"Ada Lovelace","storage":"sqlite-memory"}
```

The default server binds only to `127.0.0.1:39388`, serves one request, then
exits. The SQLite database is in memory, so repeated runs start from the same
clean state and do not leave a database file behind.

## Cleanup

Generated workspace state is confined to `workspace/`:

```sh
cd ..
rm -rf workspace
rm -f /tmp/duumbi-flagship-example.log
```

## Limits

This is a reference example, not a production web service. It does not include
dynamic request handlers, authentication, TLS, concurrent serving, migrations,
or public network access.
