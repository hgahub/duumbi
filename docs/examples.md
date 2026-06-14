# Examples

DUUMBI's flagship reference example is
[`examples/flagship-http-sqlite-json/`](../examples/flagship-http-sqlite-json/).

It demonstrates a small local JSON API backed by SQLite:

- a DUUMBI graph prepares and queries an in-memory SQLite data set;
- the response body includes values read back from SQLite;
- the server binds to loopback, serves one request, and exits;
- the README shows how to materialize the tracked source files into a normal
  `.duumbi/` workspace and build it offline from vendored stdlib modules.

Use it as the first "see DUUMBI work" path before inspecting lower-level
compiler, stdlib, or registry tests.
