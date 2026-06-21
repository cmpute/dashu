# Performance

`dashu` aims to be efficient while staying portable. By default it compiles for
the generic baseline of each target architecture, so a binary that depends on
`dashu` runs on any CPU that the target baseline supports.

When big-number arithmetic is on the hot path, you can get a meaningful speedup
by telling the compiler which CPU you are actually running on.

## Build with `target-cpu=native`

The single most impactful setting is to compile with the host CPU's feature set:

```sh
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

This lets LLVM use the full instruction set of your CPU across the **entire**
crate. On a modern x86-64 processor, for example, it enables `bmi1`/`bmi2`
(the flag-free `mulx` widening multiply), `adx` (`adcx`/`adox` carry chains),
`avx2`, and `fma`, which accelerate the inner multiplication, squaring, and
addition kernels throughout the library. This is strictly broader and faster
than the runtime feature detection `dashu` applies on its own (see below).

To make the setting permanent for a project, add it to `.cargo/config.toml`:

```toml
[build]
rustflags = ["-C", "target-cpu=native"]
```

> **Portability caveat:** a binary built with `target-cpu=native` (or a fixed
> `x86-64-v3`) will crash with an illegal instruction on older CPUs that lack
> those features. Use this for binaries deployed on known, modern hardware.
> `dashu` itself, as a published library, always keeps the portable default —
> the choice of `target-cpu` is yours, the downstream user.
>
> If you want most of the speedup while remaining compatible with reasonably
> recent hardware, `-C target-cpu=x86-64-v3` is a good fixed alternative
> (it assumes a Haswell-era / Excavator-era CPU or newer).

Note that `target-cpu=native` targets the CPU of the **machine doing the
build**. If you build on one host and deploy to another, prefer an explicit
`target-cpu`/`target-feature` that matches the deployment hardware instead.

## Runtime feature detection (default builds)

Even in a default baseline build, `dashu-int`'s hottest basecase multiplication
kernels dispatch at runtime to a BMI2 (`mulx`) implementation on x86-64 when
the host CPU supports it (this requires the `std` feature). So a portable
baseline binary already picks up `mulx` on modern CPUs for those kernels,
without any extra configuration.

This only covers the specific kernels that opt into runtime detection, though.
Building with `target-cpu=native` as described above applies the optimization
**everywhere**, so it is always at least as fast and is the recommended choice
when you control the build.
