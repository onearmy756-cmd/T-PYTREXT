# 🤝 Contributing to T-PYTREXT

> Karibu! Welcome! We welcome contributions from everyone.

---

## Code of Conduct

- Be respectful and inclusive
- Write in English or Swahili
- Focus on code quality and documentation
- Help others learn

---

## How to Contribute

### 1. Fork & Clone

```bash
git clone https://github.com/YOUR_USERNAME/T-PYTREXT.git
cd T-PYTREXT
pip install -r requirements.txt
pip install -r requirements-dev.txt
```

### 2. Create a Branch

```bash
git checkout -b feature/my-awesome-feature
# or
git checkout -b fix/my-bug-fix
```

### 3. Make Changes

- Follow existing code style
- Add tests for new features
- Update documentation if needed
- Run tests before committing

### 4. Run Tests

```bash
# All tests
pytrex test

# Specific tests
python -m pytest tests/test_extended_features.py -v

# Ensure 100% pass rate
```

### 5. Commit & Push

```bash
git add .
git commit -m "feat: add amazing new feature"
git push origin feature/my-awesome-feature
```

### 6. Create Pull Request

Go to GitHub and create a PR against `master` branch.

---

## Project Structure

```
T-PYTREXT/
├── pytrex/           # Python package (all .py files here)
├── src/              # Rust source (lib.rs)
├── pytrex_engine/    # Elixir OTP application
├── demos/            # Demo applications
├── tests/            # Test suites
├── deploy/           # Production deployment configs
├── frontend/         # UI templates
└── docs/             # Documentation
```

---

## Code Style

### Python
- Follow PEP 8
- Use type hints where possible
- Docstrings in English or Swahili
- Maximum line length: 100 characters

### Rust
- Follow standard Rust conventions
- Use `#[pyfunction]` for Python-exposed functions
- Document with `///` comments

### Elixir
- Follow Elixir conventions
- Use `@moduledoc` and `@doc`
- Pattern matching preferred

---

## Adding New Features

### Adding a Python Module

1. Create `pytrex/new_module.py`
2. Add class with proper docstrings
3. Import in `pytrex/__init__.py`
4. Add to `__all__` list
5. Add tests in `tests/`

### Adding a Rust Function

1. Add `#[pyfunction]` in `src/lib.rs`
2. Register in `#[pymodule] my_framework` block
3. Add Python bridge class in `pytrex/core.py`
4. Add tests

### Adding an Elixir Handler

1. Add event handler in `pytrex_engine/lib/task_dispatcher.ex`
2. Update `websocket_handler.ex` if needed
3. Test with `iex -S mix`

---

## Reporting Bugs

Use GitHub Issues with:
- Title: `[BUG] Brief description`
- Steps to reproduce
- Expected vs actual behavior
- PyTreXT version
- Python version
- OS info

---

## Feature Requests

Use GitHub Issues with:
- Title: `[FEATURE] Brief description`
- Use case description
- Proposed implementation (optional)
- Why it benefits the framework

---

## Recognition

All contributors will be listed in the README and CONTRIBUTORS.md.

---

**Asante! Thank you! 🇹🇿**
