### Guidelines

- This project is for private use. Do not add backward-compatibility layers or
  fallbacks.
- For Python scripts, do not add a shebang or `from __future__` imports.
- Write Python tests with `pytest`, not `unittest`.
- Implement work in small, incremental changes.
- When a task changes multiple files, fan out sub-agents to parallelize the
  work.
- If a required investigation or build tool is missing, do not look for or use
  a workaround. Tell the user which tool is missing and ask them to install it.

### Repository layout

- `crates/` contains the Rust source code.
- `packages/` contains supplementary packages.
- `docs/` contains project documentation; put specifications in `docs/specs/`.
- `Cargo.toml` and `pyproject.toml` contain project configuration.
- Treat remaining Java files as reference or legacy material unless the task
  explicitly asks you to change them.

### Code discovery

Use the codebase-memory MCP graph tools before grep, glob, or file search when
discovering code. Call `index_repository` first if the repository is not
indexed. Prefer `search_graph`, `trace_path`, `get_code_snippet`, `query_graph`,
and `get_architecture`; use `search_code` when those tools do not provide enough
detail. Use grep or file search for string literals and non-code files.

### Documentation

Use the Google developer documentation style skill when writing or editing
project documentation.

### Terminal tools

- Use Zsh for shell scripts and interactive shell commands.
- Use `rg` and `rg --files` for text and file searches.
- Use `z` from zoxide for directory navigation.
- Prefer Zsh built-ins over Bash compatibility constructs; use `zargs` instead
  of `xargs`.

### CFR decompiler

- Run `just download-cfr` to download the pinned CFR release into
  `target/tools/`. The recipe verifies its SHA-256 before installing the JAR.
- Run `just decompile-cfr <input.jar> <output-dir>` to decompile a JAR. For
  example, use `just decompile-cfr path/to/dependency.jar target/decompiled`.
- Update the pinned release version and checksum together when upgrading CFR.
- Decompile the exact dependency JAR when its source JAR is unavailable; do not
  substitute a nearby release's sources.
