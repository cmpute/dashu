<p align="center">
  <img src="./assets/dashu-banner.png" alt="dashu">
</p>

# `dashu` 用户指南

[English](https://zyxin.xyz/dashu/) | [简体中文](https://zyxin.xyz/dashu-zh/)

欢迎来到 `dashu` 用户指南！`dashu` 是一套用 Rust 实现的任意精度数值（即大数）库。

本指南是 [`dashu` API 文档](https://docs.rs/dashu/latest/dashu/)的配套读物，以更精炼的方式概述 `dashu` 提供的全部功能，并附有示例。

请从左侧的章节列表中选择，跳转到各个主题。

{{#include version.md}}

## 设计理念与特性

`dashu` 希望成为你在以下场景中的首选库：编写涉及任意精度数值的算法，或者构建依赖任意精度数值的 Rust 工具。它完全用 Rust 从头构建，提供友好且符合 Rust 惯例的 API。对于高精度的计算负载，它也许不是最快的库，但它的设计目标是让你在绝大多数情况下几乎察觉不到它的存在。

为了让每位 Rust 开发者都能用上它，它具有以下特性：
- 纯 Rust 实现，完整支持 `no_std`。
- 优先关注易用性与可读性，其次才是运行效率。
- 当前 MSRV 为 1.68。

## 元 crate

`dashu` 是一个元 crate（meta crate），对外暴露所有子 crate（`dashu-base`、`dashu-int`、`dashu-float`、`dashu-ratio`、`dashu-cmplx` 和 `dashu-macros`）的功能。每个子 crate 在 `dashu` 中对应一个模块：`dashu-base` → `dashu::base`、`dashu-int` → `dashu::integer`、`dashu-float` → `dashu::float`、`dashu-ratio` → `dashu::rational`、`dashu-cmplx` → `dashu::complex`。它还为各类数值类型创建了更具可读性的别名：
- `dashu::Natural` = `dashu::integer::UBig`
- `dashu::Integer` = `dashu::integer::IBig`
- `dashu::Rational` = `dashu::rational::RBig`
- `dashu::Real` = `dashu::float::FBig`
- `dashu::Decimal` = `dashu::float::DBig`
- `dashu::Complex` = `dashu::complex::CBig`

在本指南中，我们使用这些数值类型的原始名称（即 `XBig`），但相关说明同样适用于上述重导出的别名类型。

## Cargo 特性

Dashu 为 cargo 定义了若干可选特性（feature），用于支持各种第三方 crate。其中大多数默认未启用。特别地，我们对这些特性采用了一套特殊的命名规则：
- 对于已发布稳定版本（达到 v1.0）的依赖，我们使用 `xxx_vyy` 表示其各个主版本，并用 `xxx` 指向其中某一个主版本。更改 `xxx` 所指向的版本在 `dashu` 中视为破坏性变更（需要提升主版本号）。因此，当你启用这些稳定特性来依赖 `dashu` 时，`dashu` 后续为更新版本新增的实现不会影响你的代码。
- 对于仅有不稳定版本（v1.0 之前）的依赖，我们始终使用 `xxx_vyy` 表示每个主版本，同时提供 `xxx` 特性作为**最新**版本的别名。因此，当你启用这些不稳定特性来依赖 `dashu` 时，这些依赖的升级可能导致你的代码编译失败。但我们仍不将其视为破坏性变更，因为不稳定的依赖默认不会被启用。如果你想避免由此引发的破坏性变更，请显式指定所使用的版本。

**示例**：在 `dashu-float` 中，对 diesel 库 v1 的支持放在名为 `diesel_v1` 的特性下，对 v2 的支持放在名为 `diesel_v2` 的特性下（未编号的 `diesel` 特性指向 v2）。另一方面，`rand` crate 尚未发布稳定版，即便它已被广泛使用。因此，对 `rand` v0.8、v0.9 和 v0.10 的支持分别放在名为 `rand_v08`、`rand_v09` 和 `rand_v010` 的特性下，而特性名 `rand` 当前指向 `rand_v010`。

在你的 Cargo.toml 中，如果你启用的是 `dashu/diesel_v1`、`dashu/diesel_v2` 或 `dashu/rand_v08`，那么将来 `dashu` 新增对 diesel v3 或 rand v0.11 的支持时，也不会带来任何破坏性变更的风险。但如果你启用的是 `dashu/rand` 而非 `dashu/rand_v010`，则存在风险，因为将来 `rand` 可能会指向更新的版本。

## 许可证

根据以下任一协议授权，由你自行选择：[Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) 或 [MIT license](https://opensource.org/licenses/MIT)。
