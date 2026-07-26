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
