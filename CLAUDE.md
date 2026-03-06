# jcodemunch MCP — Codebase Search

Use `mcp__jcodemunch` tools instead of Read/Grep for codebase exploration. The repo identifier is `local/inputforge`.

## When to use

- **Understanding a function/type**: `search_symbols` → `get_symbol` (returns only the target, not the whole file)
- **Finding callers/references**: `search_text` (compact line-level matches, no need to read full files)
- **File overview**: `get_file_outline` (all symbols with signatures, no source bodies)
- **Browsing structure**: `get_file_tree` or `get_repo_outline`

## When NOT to use

- **Editing files**: still use Read → Edit (you need full file context for edits)
- **Reading a known file in full**: use Read directly
- **Simple single-file grep**: Grep is fine for 1-2 files

## Re-indexing

Run `index_folder` with `incremental: true` after significant code changes to keep the index current. Use `use_ai_summaries: false` to avoid API costs.
