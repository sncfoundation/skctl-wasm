<p align="center">
  <img src="https://sncfoundation.github.io/logos/skctl-wasm.svg" width="96" alt="skctl-wasm logo">
</p>

<h1 align="center">skctl-wasm</h1>

<p align="center"><b>The CLI compiled to WebAssembly</b><br>
A <a href="https://sncfoundation.github.io">Sheet-Native Computing Foundation</a> project &#183; analog of <b>kubectl in the browser</b></p>

---

**Status:** 📝 Unsaved Draft &#183; this project is a proposal. Design notes and contributions are welcome.

## About

The CLI compiled to WebAssembly Part of the spreadsheet-native stack — the Sheet stays the source of truth, and it reconciles.

`skctl-wasm` is [`skctl`](https://github.com/sncfoundation/sheeternetes/blob/main/skctl) compiled to WebAssembly so it runs **client-side in the browser**, talking straight to the Apps Script apiserver — no shell, no `curl`, no `jq`.

The wasm module is deliberately pure: it only *builds* request URLs / bodies and *renders* responses. The browser does the actual `fetch`. Exposed functions:

- `get_url(base, token, kind)` → the apiserver GET URL (`?token=…&kind=pods|nodes|deployments|events`)
- `apply_body(token, deployments_json)` → the `apply` POST body
- `scale_body(token, name, replicas)` / `delete_body(token, name)` → the `scale` / `delete` POST bodies
- `render_table(items_json, kind)` → a fixed-width text table from an `{items:[…]}` response

## Build & run

Prerequisites: a Rust toolchain and [`wasm-pack`](https://rustwasm.github.io/wasm-pack/).

```bash
# 1. Build the wasm + JS bindings into ./pkg
wasm-pack build --target web

# 2. Serve the folder (any static server works; fetch/ESM need http://, not file://)
python3 -m http.server 8080
# then open http://localhost:8080/index.html
```

Enter your `WEBAPP_URL` (the Apps Script `/exec` URL) and `TOKEN`, pick a kind, and hit **skctl get**.

Run the unit tests on the host (no wasm needed):

```bash
cargo test
```

## Get involved

- 📋 Tracking issue &amp; design: [sncfoundation/sheeternetes#25](https://github.com/sncfoundation/sheeternetes/issues/25)
- 🗺️ [SNCF Landscape](https://sncfoundation.github.io/landscape.html)
- 🧩 [All projects](https://sncfoundation.github.io/projects.html)
- ⚖️ [Governance &amp; how to contribute](https://github.com/sncfoundation/governance)
- 🎓 [Get certified (CSFE)](https://sncfoundation.github.io/certification.html)

## Status legend

Everything starts as an **Unsaved Draft**. It reconciles up the tiers from there — see the
[maturity model](https://sncfoundation.github.io/foundation.html#maturity).

---

<sub>Licensed under Apache-2.0. The SNCF does not recommend running production on a spreadsheet. If you do, please film it.</sub>
