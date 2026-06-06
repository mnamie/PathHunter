# PathHunter

Cross-platform PATH auditor, implemented in Python with `uv`.

## After editing code

Always run before considering a task done:

```sh
uv run ruff check src/   # linting — catches unused imports, style issues
uv run pyright src/         # type checking — catches exhaustive match gaps, type errors
uv run pytest -q         # tests
```

## Dev commands

```sh
uv sync          # install deps
uv run ph        # run from source
uv run ph --help
```
