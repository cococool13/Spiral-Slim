"""Put the repository root on sys.path for pytest.

`python -m unittest discover` runs with the working directory importable, so
`import browser_collection` just works. pytest does not do that when the tests
live in a directory without an `__init__.py`, so CI could not import the
package that half the suite is about.

This is the only thing in the repository that exists for the test runner. The
scripts themselves remain stdlib-only and import nothing from here.
"""

import sys

from pathlib import Path


ROOT = Path(__file__).resolve().parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


# Modules that cannot even be imported on this platform.
#
# The macOS and Linux entrypoints draw a curses TUI, and Windows has no
# curses in the standard library. Without this, collecting the suite on
# Windows raises ModuleNotFoundError before a single test runs — which is
# exactly what happened the first time the Windows job ran, and it hid
# everything that came after it.
#
# All three load an entrypoint that draws that TUI. They test the macOS and
# Linux side; there is nothing in them for Windows to assert.
_NEEDS_CURSES = (
    "tests/test_slimbrave.py",
    "tests/test_plan_interface.py",
    "tests/test_custom_profile.py",
)

collect_ignore = []
try:  # pragma: no cover - the branch taken depends on the host
    import curses  # noqa: F401
except ImportError:
    collect_ignore.extend(_NEEDS_CURSES)
