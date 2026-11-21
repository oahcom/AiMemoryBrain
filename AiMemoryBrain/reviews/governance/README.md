# Governance Review Archive

## Purpose

This directory stores independent governance review reports as
standalone audit evidence.

Each archived report records the exact commit reviewed, the review
verdict, and the relevant merge state. Reports preserve the
independent review record and must not be treated as GitHub review
submissions or as substitutes for GitHub's approval mechanism.

## Archival constraints

1. Review reports are independent evidence artifacts.
2. Review reports MUST NOT be committed as part of the PR branch
   whose changes they review.
3. Reports MUST NOT claim or imply that an independent review verdict
   constitutes a GitHub `APPROVE` review submission.
4. The reviewed head SHA must be recorded exactly.
5. Archival commits must remain distinguishable from the reviewed
   PR's commits.
6. A report applies only to its explicitly identified frozen head SHA.
   Any change to that head invalidates the report's applicability
   to the changed head.

## Archived reports

| Report ID | Review target (frozen head SHA) | Verdict | Merge state at archival |
|---|---|---|---|
| REV-2026-11-20-009 | `f2bc4d81e66bf819ef6f9c2967c1a54332143b26` | APPROVE | MERGED — PR #4, merge commit `c12926b66fbd399ad2b3235c285f47469aa7ee2c` (merge method: merge) |

## Archival ordering and audit integrity

The archival of `REV-2026-11-20-009` takes place AFTER PR #4
was merged.

PR #4 was merged using a merge commit:

- PR: #4
- Reviewed frozen head: `f2bc4d81e66bf819ef6f9c2967c1a54332143b26`
- Merge commit: `c12926b66fbd399ad2b3235c285f47469aa7ee2c`
- Merge method: `merge`

Consequently, the archival commit cannot change the reviewed head SHA
of PR #4. The review target is the frozen head recorded above, not
the subsequent archival commit.

The archival operation is an independent, subsequent audit-record
operation and is not part of the reviewed PR's change set.

## Review-round coverage

This archive contains only the R9 report:

`REV-2026-11-20-009-independent-review.md`

R1–R8 reports have NOT been individually archived in this directory.

The conclusions of R1–R8 are represented through the R9 report
and the associated decision record:

`DEC-RD-2026-11-20-002.md`

Their mention does not imply that individual R1–R8 report files
have been archived here.

## GitHub approval boundary

An independent review verdict of `APPROVE` is an audit conclusion,
not a GitHub pull request approval.

The applicable GitHub ruleset has:

`required_approving_review_count = 0`

A single-user account cannot use its own GitHub approval as an
independent third-party review gate.

Accordingly, the reports in this directory must not be represented
as GitHub approval events.
