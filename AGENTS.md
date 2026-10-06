### Guidelines

This project is meant for private use only, never consider backward compatabilty, fallbacks, etc

For Python scripts, never add a shebang or `from __future__` imports.
Python tests must use pytest, never unittest.

Never try to one-shot whole implementation, apply changes in small incremental chunks.

When working on multiple files simultaneosuly, always fan-out sub-agents to parallelize work.


#### Repositopry structure 

- crates/ contains rust sources (need to be implemented)
- packages contains supplementary packages
- docs for documentation 

pyproject.toml and Cargo.toml target project config. 

All other files are leftovers from original java implementation.