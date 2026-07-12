## 概述

**`dashu-python`** Rust crate（位于 [dashu](https://github.com/cmpute/dashu) 工作区的 `python/` 目录中）是 Python 绑定的源码——即暴露 dashu 数值类型为原生扩展的 [PyO3](https://pyo3.rs) 胶水代码。[PyPI](https://pypi.org/project/dashu-rs/) 上的 **`dashu-rs`** 包是该 crate 的预构建、可通过 pip 安装的发行版：`pip install dashu-rs` 会获取编译好的 wheel，因此终端用户无需 Rust 或 dashu 源码树。导入的 Python 模块名为 `dashu`（例如 `from dashu import UBig`）。

{{#include ../../python/USAGE-zh.md}}
