# dashu

[English](README.md) | [简体中文](README_zh.md)

<img src="guide/src/assets/dashu-banner.png" alt="dashu">

[![Crate](https://img.shields.io/crates/v/dashu.svg)](https://crates.io/crates/dashu)
[![Docs](https://docs.rs/dashu/badge.svg)](https://docs.rs/dashu)
[![Tests](https://github.com/cmpute/dashu/actions/workflows/tests.yml/badge.svg)](https://github.com/cmpute/dashu/actions)
[![MSRV 1.68](https://img.shields.io/badge/rustc-1.68%2B-informational.svg)](#dashu)
[![License](https://img.shields.io/crates/l/dashu)](#license)
[![Book](https://img.shields.io/badge/book-user_guide-yellow.svg)](https://zyxin.xyz/dashu-zh/)

一套用 Rust 实现的任意精度数值（即大数）库。它是 GNU GMP + MPFR + MPC 的 Rust 原生替代方案。其主要特性包括：
- 纯 Rust 实现，完整支持 `no_std`。
- 优先关注易用性与可读性，其次才是运行效率。
- 经过优化的运行速度与内存占用。
- 当前 MSRV 为 1.68。

## 套件内的crate

- [`dashu-base`](./base)：通用 trait 定义
- [`dashu-int`](./integer)：任意精度整数
- [`dashu-float`](./float)：任意精度浮点数
- [`dashu-ratio`](./rational)：任意精度有理数
- [`dashu-cmplx`](./complex)：任意精度复数
- [`dashu-macros`](./macros)：用于创建大数的宏

`dashu` 是一个元 crate（meta crate），重新导出上述所有子 crate 中的类型。各子目录下的 README.md 中有针对单个 crate 的专门介绍。

## Python 包

[`dashu-python`](./python) 是 dashu 核心功能的友好试验田：通过 PyPI 上的
[`dashu-rs`](https://pypi.org/project/dashu-rs/) 包，用户无需 Rust 工具链，即可在
Python 中体验 dashu 的任意精度整数、有理数、浮点数与复数，直观了解 dashu 的能力。
除了用于探索 dashu 之外，它本身也是一个独立的、面向 Python 生态的任意精度数值包。

## 许可证

根据以下任一协议授权：

 * Apache License, Version 2.0
   ([LICENSE-APACHE](../LICENSE-APACHE) 或 https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](../LICENSE-MIT) 或 https://opensource.org/licenses/MIT)

由你自行选择。

## 贡献

除非你明确声明，否则依据 Apache-2.0 协议的定义，任何由你主动提交并包含在本作品中的贡献，
都将按上述方式双重授权，不附加任何额外的条款或条件。
