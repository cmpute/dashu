# Hegel bug-report triage

Triage of the bug reports produced by the Hegel report runs against dashu at
commit `a0fc535` (339 reports across 5 runs). The goal is to establish which
reports are distinct, which are real, and to land a regression test and a fix
for each real one.

## Report IDs

Each report is identified by a run letter plus the bug number within that run:

| Letter | Run directory | Reports |
|---|---|---|
| A | `20261123-152612-dashu-7crhcz` | 13 |
| B | `20261123-160800-dashu-cz3tj9` | 12 |
| C | `20261123-163245-dashu-fsqtq2` | 5 |
| D | `20261123-183941-dashu-wdhear` | 43 |
| E | `20261123-204245-dashu-3w8n3r` | 266 |
| F | `20261125-083220-dashu-ty8dga` | 96 |

(`20261123-172656-dashu-fy45qc` failed before producing reports; the `arctic`
run is a different project.)

`index.tsv` lists every report: id, triage group, the run's own
input/consequence classification, the report path and originating test patch
(both relative to the runs directory), and the title.

Runs A–E were against `a0fc535`. Run F was against `8137b6e`, the tree with the
fixes for A–E applied; its notes are the `F-<group>.md` files, and each cluster
there also records its provenance (introduced by the earlier fixes, or already
present at `a0fc535` and missed or masked).

## Files

- `index.tsv` — the full report list and group assignment.
- `<group>.md` — per-group notes: one entry per report with its cluster,
  verdict, test location and fix.
- `SUMMARY.md` — the consolidated list of distinct issues and verdicts.

## Verdict vocabulary

- **real, fixed** — reproduced, regression test added, fix landed.
- **real, not fixed** — reproduced and tested (test `#[ignore]`d with a reason), fix out of scope; says why.
- **duplicate of X** — same root cause as cluster X (the cluster entry carries the verdict).
- **not a bug** — behaviour is intended/documented; says why. A test pinning the intended behaviour is added where useful.
- **docs** — the code is right and the documentation was wrong (docs fixed).
- **not reproducible** — could not reproduce at this commit.
