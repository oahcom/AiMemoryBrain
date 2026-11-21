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

| Report ID | Review target (frozen head SHA) | Verdict | Merge state at archival | Archival commit SHA | Archival PR merge commit SHA |
|---|---|---|---|---|---|
| REV-2026-11-20-009 | `f2bc4d81e66bf819ef6f9c2967c1a54332143b26` | APPROVE | MERGED — PR #4, merge commit `c12926b66fbd399ad2b3235c285f47469aa7ee2c` (merge method: merge) | `94c3326c0c4c0166876fc86a62946516ee40600e` | `0df3d146dc54002668a2c8cfb26af9fbf9b5743c` (PR #7) |

### Archived artifact integrity

These digests describe the artifacts as committed in
`94c3326`; they are not digests of the current files.

| File | Git blob SHA | File size (bytes) | SHA-256 |
|---|---|---|---|
| `reviews/governance/REV-2026-11-20-009-independent-review.md` | `148e37e331c49579d8e8f7e7758f3a8c6a90e772` | 7262 | `c30c72ee4161ed6dc0b21f5900deb327aca3a90508ebfdd6e53fbd2c98ce0ee5` |

The report archival commit is:

`94c3326c0c4c0166876fc86a62946516ee40600e`

This commit added two files without modifying or deleting
existing files:

- `reviews/governance/README.md`
- `reviews/governance/REV-2026-11-20-009-independent-review.md`

The README version included in that archival commit did not
record the archival commit SHA. That traceability entry is
being added by the subsequent governance index update.

The archival commit was subsequently incorporated into `main`
through PR #7.

PR #7 was merged using the merge method. Its merge commit is:

`0df3d146dc54002668a2c8cfb26af9fbf9b5743c`

The merge commit has the archival commit as one of its parents.
The archival commit is therefore an ancestor of the current main
history.

The commit that introduced the report into the main branch is
the PR #7 merge commit identified above. The archival commit
identifies the original report-and-index-file addition, not
the subsequent pull-request merge commit.

### Governance structural check evidence

The `Governance structural checks` check run on archival commit
`94c3326c0c4c0166876fc86a62946516ee40600e` completed with
conclusion `success`.

The PR #7 merge commit
`0df3d146dc54002668a2c8cfb26af9fbf9b5743c` has no associated
check-run record.

The workflow `.github/workflows/governance-gate.yml` listens
only to `pull_request` events:

- `opened`
- `synchronize`
- `reopened`
- `ready_for_review`

It does not listen to `push` events.

Consequently, the absence of a check-run record on the PR #7
merge commit is consistent with the workflow's configured
event triggers. It does not establish either a successful
or a failed check on that merge commit.

The successful check-run evidence applies to archival commit
`94c3326`, not to merge commit `0df3d14`.

This archive therefore does not claim that the Governance
Gate ran on the main-branch merge commit or that a main-branch
gate passed.

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

The archival commit was subsequently merged into `main` through
PR #7. Its merge commit is:

`0df3d146dc54002668a2c8cfb26af9fbf9b5743c`

The archival commit and the PR #7 merge commit are distinct
commits with distinct roles in the archival traceability chain.

The report's frozen review target remains PR #4 head
`f2bc4d81e66bf819ef6f9c2967c1a54332143b26`.

Neither the archival commit nor the PR #7 merge commit is
substituted for that reviewed head.

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

Archiving an independent review report does not change the
GitHub approval configuration or establish an independent
GitHub approval event.
