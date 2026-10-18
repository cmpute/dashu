#!/usr/bin/env python3
"""Run the library comparison sweep and plot the results.

Times one cell per (task, n, library) by timing `benchmark ... print`
processes, then writes `results.json` and `chart.html` next to this script.

    cargo build --release --features gmp   # rug/rust-gmp need system GMP
    python3 run.py                         # full sweep (sequential, ~10 min)
    python3 run.py --tasks pi,factor       # subset of tasks
    python3 run.py                         # re-run: existing cells are skipped
    python3 run.py --results results.json  # re-plot an existing sweep only

Methodology:
- one iteration per process; cells finishing under 1 s take the min of 3 runs
- factor: the library repeats 5x in one process (sharing the cached prime);
  the one-time prime generation, measured with dashu, is subtracted
"""
import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
BIN = SCRIPT_DIR / "target" / "release" / "benchmark"

# The recommended three-size ladder (see README.md); per-library subsets are
# the libraries that implement each task's number type.
LIBS_INT = ["dashu", "ibig", "num", "malachite", "rug", "rust-gmp"]
LIBS_RATIO = ["dashu", "num", "malachite"]
LIBS_PI = ["dashu", "astro_float", "rug"]
LIBS_DEC = ["dashu", "bigdecimal"]

CELLS = {
    "e": [(100000, LIBS_INT), (500000, LIBS_INT), (4000000, LIBS_INT)],
    "fib": [(1000000, LIBS_INT), (4000000, LIBS_INT), (30000000, LIBS_INT)],
    "fib_hex": [(1000000, LIBS_INT), (10000000, LIBS_INT), (100000000, LIBS_INT)],
    "io_int": [(100000, LIBS_INT), (1000000, LIBS_INT), (5000000, LIBS_INT)],
    "factor": [(200, LIBS_INT), (1000, LIBS_INT), (5000, LIBS_INT)],
    "fib_ratio": [(2000, LIBS_RATIO), (6000, LIBS_RATIO), (20000, LIBS_RATIO)],
    "io_ratio": [(1000000, LIBS_RATIO), (4000000, LIBS_RATIO), (20000000, LIBS_RATIO)],
    "io_decimal": [(100000, LIBS_DEC), (1000000, LIBS_DEC), (5000000, LIBS_DEC)],
    "pi": [(10000, LIBS_PI), (20000, LIBS_PI), (50000, LIBS_PI)],
}

TASK_UNITS = {
    "e": "digits of e",
    "fib": "Fibonacci index",
    "fib_hex": "Fibonacci index",
    "factor": "bits per prime factor",
    "io_int": "decimal digits",
    "fib_ratio": "iterations",
    "io_ratio": "decimal digits",
    "io_decimal": "decimal digits",
    "pi": "bits of π",
}

# Stable per-panel series colors; pairs sharing an index never meet in a task.
LIB_SERIES = {
    "dashu": 1, "rug": 2, "rust-gmp": 3, "malachite": 4,
    "ibig": 5, "num": 6, "astro_float": 4, "bigdecimal": 3,
}
LIB_ORDER = ["dashu", "rug", "rust-gmp", "malachite", "ibig", "num",
             "astro_float", "bigdecimal"]


def run_once(libs, task, n):
    args = [str(BIN)]
    for lib in libs:
        args += ["--lib", lib]
    args += ["--task", task, "-n", str(n), "print"]
    t0 = time.perf_counter()
    subprocess.run(args, stdout=subprocess.DEVNULL, check=True)
    return time.perf_counter() - t0


def measure_factor_gen(n):
    """One-time prime-generation cost: gen + iter vs gen + 2*iter."""
    t1 = run_once(["dashu"], "factor", n)
    t2 = run_once(["dashu", "dashu"], "factor", n)
    return max(2 * t1 - t2, 0.0)


def sweep(results, tasks, results_path):
    for task in tasks:
        configs = CELLS[task]
        factor_gen = {}
        if task == "factor":
            for n, _ in configs:
                factor_gen[n] = measure_factor_gen(n)
                print(f"factor n={n}: prime generation {factor_gen[n]:.2f}s", flush=True)
        results.setdefault(task, {})
        for n, libs in configs:
            results[task].setdefault(n, {})
            for lib in libs:
                if lib in results[task][n]:
                    print(f"{task:10} n={n:<8} {lib:10} cached", flush=True)
                    continue
                if task == "factor":
                    t = run_once([lib] * 5, task, n)
                    t = max((t - factor_gen[n]) / 5, 1e-4)
                else:
                    t = run_once([lib], task, n)
                    if t < 1.0:
                        t = min(t, run_once([lib], task, n), run_once([lib], task, n))
                results[task][n][lib] = round(t, 4)
                print(f"{task:10} n={n:<8} {lib:10} {t:8.3f}s", flush=True)
                json.dump(results, open(results_path, "w"), indent=1)
    print("sweep done")


def fmt_n(n):
    if n >= 1_000_000 and n % 1_000_000 == 0:
        return f"{n // 1_000_000}M"
    if n >= 1_000 and n % 1_000 == 0:
        return f"{n // 1_000}k"
    return str(n)


def generate_chart(results, out_path):
    data = {}
    for task in CELLS:
        if task not in results:
            continue
        sizes = sorted(int(n) for n in results[task])
        libs = [l for l in LIB_ORDER
                if all(l in results[task][n] for n in sizes)]
        if not libs:
            continue
        data[task] = {
            "unit": TASK_UNITS[task],
            "sizes": [fmt_n(n) for n in sizes],
            "libs": libs,
            "times": {lib: [round(float(results[task][n][lib]), 4)
                            for n in sizes] for lib in libs},
        }
    payload = json.dumps(data, separators=(",", ":"))
    series = json.dumps(LIB_SERIES)
    html = HTML_TEMPLATE.replace("__PAYLOAD__", payload).replace("__SERIES__", series)
    out_path.write_text(html)
    print(f"chart written to {out_path}")


HTML_TEMPLATE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Library comparison by benchmark task</title>
<style>
:root {
  --fg: #1f2937; --muted: #6b7280; --border: #d1d5db;
  --s1: #2563eb; --s2: #16a34a; --s3: #ea580c; --s4: #9333ea; --s5: #0d9488; --s6: #db2777;
}
@media (prefers-color-scheme: dark) {
  :root {
    --fg: #e5e7eb; --muted: #9ca3af; --border: #4b5563;
    --s1: #60a5fa; --s2: #4ade80; --s3: #fb923c; --s4: #c084fc; --s5: #2dd4bf; --s6: #f472b6;
  }
}
body { margin: 0 auto; padding: 16px; max-width: 960px;
  font: 400 14px/1.5 system-ui, sans-serif; color: var(--fg); }
h3 { margin: 0 0 4px; font-size: 16px; font-weight: 500; }
.sub { color: var(--muted); font-size: 12px; margin-bottom: 12px; }
.row { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin: 8px 0; }
button { font: inherit; color: inherit; }
.btn { background: transparent; border: 1px solid var(--border);
  border-radius: 6px; padding: 3px 10px; cursor: pointer; }
.btn[aria-pressed="true"] { background: var(--s1); border-color: var(--s1); color: #fff; }
.legend-btn { background: transparent; border: none; padding: 2px 6px; cursor: pointer; }
.swatch { display: inline-block; width: 10px; height: 10px; border-radius: 2px;
  margin-right: 6px; vertical-align: baseline; }
#tip { position: fixed; pointer-events: none; background: var(--fg); color: #fff;
  border-radius: 4px; padding: 3px 8px; font-size: 12px; display: none; white-space: nowrap; }
.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden;
  clip: rect(0 0 0 0); white-space: nowrap; }
svg text { fill: var(--fg); font-size: 12px; }
</style>
</head>
<body>
<div id="bench-lib-comparison">
  <h3>Library comparison by benchmark task</h3>
  <div class="sub">Time for one task iteration, log scale — lower is better. Min of 3 process runs for sub-second cells; the factor task excludes its one-time prime generation.</div>
  <div class="row" id="blc-tasks" role="group" aria-label="Benchmark task"></div>
  <div class="row" id="blc-legend" role="group" aria-label="Libraries"></div>
  <div class="sr-only" aria-live="polite" id="blc-status"></div>
  <div id="blc-chart"></div>
  <div id="tip" role="tooltip"></div>
</div>
<noscript>This chart requires JavaScript and loads D3 from a CDN.</noscript>
<script src="https://cdn.jsdelivr.net/npm/d3@7.9.0/dist/d3.min.js"></script>
<script>
(function () {
  const root = document.getElementById("bench-lib-comparison");
  if (!root) return;
  const DATA = __PAYLOAD__;
  const SERIES = __SERIES__;
  const TASKS = Object.keys(DATA);
  const tip = root.querySelector("#blc-tip");
  let state = { task: TASKS[0], hidden: [] };

  function fmt(v) {
    return v >= 100 ? v.toFixed(0) : v >= 10 ? v.toFixed(1)
      : v >= 1 ? v.toFixed(2) : v.toFixed(3);
  }

  const taskBox = root.querySelector("#blc-tasks");
  TASKS.forEach((task) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "btn";
    b.textContent = task;
    b.setAttribute("aria-pressed", String(task === state.task));
    b.addEventListener("click", () => {
      state.task = task;
      taskBox.querySelectorAll("button").forEach((x) =>
        x.setAttribute("aria-pressed", String(x === b)));
      buildLegend();
      draw();
      const st = root.querySelector("#blc-status");
      if (st) st.textContent =
        `showing ${task}: ${DATA[task].libs.length} libraries, sizes ${DATA[task].sizes.join(", ")}`;
    });
    taskBox.appendChild(b);
  });

  function buildLegend() {
    const box = root.querySelector("#blc-legend");
    box.textContent = "";
    DATA[state.task].libs.forEach((lib) => {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "legend-btn";
      const swatch = document.createElement("span");
      swatch.className = "swatch";
      swatch.style.background = `var(--s${SERIES[lib]})`;
      if (state.hidden.includes(lib)) swatch.style.opacity = "0.25";
      b.appendChild(swatch);
      b.appendChild(document.createTextNode(lib));
      b.setAttribute("aria-pressed", String(!state.hidden.includes(lib)));
      b.addEventListener("click", () => {
        state.hidden = state.hidden.includes(lib)
          ? state.hidden.filter((x) => x !== lib)
          : state.hidden.concat([lib]);
        buildLegend();
        draw();
      });
      box.appendChild(b);
    });
  }

  function showTip(ev, text) {
    tip.textContent = text;
    tip.style.display = "block";
    const pad = 12;
    let x = ev.clientX + pad, y = ev.clientY + pad;
    if (x + tip.offsetWidth > innerWidth) x = ev.clientX - tip.offsetWidth - pad;
    if (y + tip.offsetHeight > innerHeight) y = ev.clientY - tip.offsetHeight - pad;
    tip.style.left = x + "px";
    tip.style.top = y + "px";
  }
  function hideTip() { tip.style.display = "none"; }

  function draw() {
    const host = root.querySelector("#blc-chart");
    host.textContent = "";
    hideTip();
    const t = DATA[state.task];
    const libs = t.libs.filter((l) => !state.hidden.includes(l));
    const w = Math.max(host.clientWidth || host.parentElement.clientWidth, 320);
    const h = 380;
    const m = { top: 18, right: 18, bottom: 64, left: 64 };

    const svg = d3.create("svg")
      .attr("viewBox", `0 0 ${w} ${h}`)
      .attr("width", "100%")
      .attr("role", "img")
      .attr("aria-label", `Bar chart of single-iteration time for the ${state.task} task, grouped by input size, one bar per library, logarithmic time axis`);
    const g = svg.append("g");

    const values = libs.flatMap((l) => t.times[l]);
    if (!values.length) { host.appendChild(svg.node()); return; }
    const lo = Math.min(...values), hi = Math.max(...values);
    const y = d3.scaleLog()
      .domain([lo / 1.6, hi * 1.9])
      .range([h - m.bottom, m.top]);
    const x0 = d3.scaleBand()
      .domain(t.sizes)
      .range([m.left, w - m.right])
      .paddingOuter(0.18)
      .paddingInner(0.28);
    const x1 = d3.scaleBand()
      .domain(libs)
      .range([0, x0.bandwidth()])
      .padding(0.08);

    g.selectAll("line.grid").data(y.ticks(5)).join("line")
      .attr("x1", m.left).attr("x2", w - m.right)
      .attr("y1", (d) => y(d)).attr("y2", (d) => y(d))
      .attr("stroke", "var(--border)")
      .attr("stroke-width", 1)
      .attr("opacity", 0.55);

    g.append("g").attr("transform", `translate(${m.left},0)`)
      .call(d3.axisLeft(y).ticks(5, (d) => d3.format("~g")(d)))
      .call((ax) => ax.select(".domain").remove())
      .selectAll("text").classed("tabular", true);
    g.append("text")
      .attr("transform", "rotate(-90)")
      .attr("x", -(m.top + (h - m.bottom - m.top) / 2)).attr("y", 16)
      .attr("text-anchor", "middle").attr("fill", "var(--muted)")
      .text("time per iteration (s)");

    g.append("g").attr("transform", `translate(0,${h - m.bottom})`)
      .call(d3.axisBottom(x0))
      .selectAll("text");
    g.append("text")
      .attr("x", m.left + (w - m.left - m.right) / 2)
      .attr("y", h - 12)
      .attr("text-anchor", "middle").attr("fill", "var(--muted)")
      .text(t.unit);

    t.sizes.forEach((size, i) => {
      libs.forEach((lib) => {
        const v = t.times[lib][i];
        const top = y(Math.max(v, y.domain()[0]));
        g.append("rect")
          .attr("x", x0(size) + x1(lib))
          .attr("width", Math.max(x1.bandwidth(), 3))
          .attr("y", top)
          .attr("height", Math.max(y(y.domain()[0]) - top, 1))
          .attr("fill", `var(--s${SERIES[lib]})`)
          .on("pointerenter pointermove", (ev) =>
            showTip(ev, `${lib} · ${state.task} n=${size}: ${fmt(v)} s`))
          .on("pointerleave", hideTip);
      });
    });

    host.appendChild(svg.node());
  }

  buildLegend();
  draw();
  if (window.ResizeObserver) {
    new ResizeObserver(() => draw()).observe(root.querySelector("#blc-chart"));
  }
})();
</script>
</body>
</html>
"""


def load_results(path):
    """JSON round-trips the size keys to strings; normalize back to ints."""
    raw = json.load(open(path))
    return {task: {int(n): cell for n, cell in sizes.items()}
            for task, sizes in raw.items()}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--tasks", help="comma-separated subset of tasks to (re)measure")
    ap.add_argument("--results", help="existing results.json to plot without sweeping")
    ap.add_argument("--out-dir", default=None,
                    help="output directory (default: alongside run.py)")
    args = ap.parse_args()

    out_dir = Path(args.out_dir) if args.out_dir else SCRIPT_DIR
    out_dir.mkdir(parents=True, exist_ok=True)
    results_path = Path(args.results) if args.results else out_dir / "results.json"

    if args.results:
        results = load_results(results_path)
    else:
        if not BIN.exists():
            sys.exit(f"benchmark binary not found at {BIN}; "
                     "build it with `cargo build --release --features gmp`")
        tasks = args.tasks.split(",") if args.tasks else list(CELLS)
        for task in tasks:
            if task not in CELLS:
                sys.exit(f"unknown task {task!r}; choose from {', '.join(CELLS)}")
        if results_path.exists():
            results = load_results(results_path)
        else:
            results = {}
        sweep(results, tasks, results_path)

    generate_chart(results, out_dir / "chart.html")


if __name__ == "__main__":
    main()
