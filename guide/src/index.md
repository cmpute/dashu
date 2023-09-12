# The user guide for `dashu`

Welcome to the `dashu` user guide! `dashu` is a library set of arbitrary precision numbers (aka. big numbers) implemented in Rust.

The book is a companion to [`dashu`'s API docs](https://docs.rs/dashu/latest/dashu/). It contains a more concise overview of all the functionalities equipped with `dashu` and some examples.

Please choose from the chapters on the left to jump to individual topics.

## Features

- Pure rust, full `no_std` support.
- Focus on ergonomics & readability, and then efficiency.
- Current MSRV is 1.61

## Cargo Features

Dashu has several optional features defined for cargo that supports various third-party crates. Most of them are not enabled by default. Specially, we use a special naming rule for the features.
- For feature dependencies with stable versions (reached v1.0), we will use `xxx_vyy` to represent its major versions, and `xxx` pointing to one of the major versions. Changing the version which `xxx` is pointing to is regarded as a break change in `dashu` (requiring major version bump). Therefore, when you dependends on `dashu` with these stable features, additional implementations for newer versions in `dashu` will not cause any issues in your code.
- For feature dependencies with only unstable versions (pre v1.0), we will always use `xxx_vyy` to represent each major versions. We will also provide the feature `xxx`, which is an alias of the **newest** version of the crate. Therefore, when you dependends on `dashu` with these unstable features, the upgrade of these dependencies might cause your code to fail. However, we still do not consider this as break changes, because the unstable dependencies will never be enabled by default. If you want to prevent break changes cause by this, please specify which version to use.

**Example**: In `dashu-float`, the support for the diesel library v1 is under the feature name `diesel`, and the support for v2 is under the feature name `diesel2`. On the other hand, the `rand` crate is not stable yet, even if it's already widely used. Therefore, the support for `rand` v0.7 and v0.8 is under the feature name `rand_v07` and `rand_v08` respectively. The feature name `rand` is currently pointing to `rand_v08`.

In your Cargo.toml, if you enable `dashu/diesel`, `dashu/diesel2` or `dashu/rand_v07`, there won't be any risk of break changes when `dashu` updates the support for `diesel` v3 or `rand` v0.9 in future. However the risk exists if you enable `rand` instead of `rand_v08`, because `rand` might point to `rand_v09` in future.

## License

Licensed under either [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT license](https://opensource.org/licenses/MIT) at your option.
