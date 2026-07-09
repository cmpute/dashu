## Overview

The **`dashu-python`** Rust crate (in the `python/` directory of the
[dashu](https://github.com/cmpute/dashu) workspace) is the source of the Python
bindings — [PyO3](https://pyo3.rs) glue that exposes dashu's number types as a
native extension. The **`dashu-rs`** package on
[PyPI](https://pypi.org/project/dashu-rs/) is the prebuilt, pip-installable
distribution of that crate: `pip install dashu-rs` fetches a compiled wheel, so
end users never need Rust or the dashu source tree. The imported Python module is
named `dashu` (e.g. `from dashu import UBig`).

{{#include ../../python/USAGE.md}}
