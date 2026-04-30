# InputForge: agent notes

## Debugging the Dioxus GUI via Chrome DevTools Protocol

Launch the Dioxus GUI with:

```
dx run -p inputforge-app --no-default-features --features gui-dioxus
```

(Use `dx run`, not `cargo run`; the dioxus CLI handles asset bundling and
hot-reload that plain cargo skips. `--no-default-features` disables the egui
backend so the dioxus backend is the sole GUI target.)

While that command is running in a debug build on Windows, the embedded
WebView2 exposes the Chrome DevTools Protocol on `http://127.0.0.1:9222`. The
`chrome-devtools` MCP server (registered in `.mcp.json`) attaches there and
gives the agent: screenshots, DOM snapshots, console reads, click/type, JS
eval, and network/performance traces against the live `inputforge-gui-dx`
window.

The CDP flag is gated `#[cfg(all(debug_assertions, target_os = "windows"))]` in
`crates/inputforge-gui-dx/src/lib.rs`; release builds do NOT listen on 9222.

If `/json` returns empty against `127.0.0.1`, try `localhost` (and vice versa);
WebView2Feedback#4709 documents an IPv4/IPv6 binding quirk on this exact path.
