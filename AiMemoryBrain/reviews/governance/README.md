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
| REV-2026-11-20-001 | `ff1b72a19a55f973ad200665aa74f6eec53764f8` | REVISE | MERGED — PR #1, merge commit `341bb04b` | `a7b7e1f8d49f238361c3b5d42f73aec7ec401888` | `91cd8237ebfb7617c279fa52db536396e968e741` (PR #12) |
| REV-2026-11-20-009 | `f2bc4d81e66bf819ef6f9c2967c1a54332143b26` | APPROVE | MERGED — PR #4, merge commit `c12926b66fbd399ad2b3235c285f47469aa7ee2c` (merge method: merge) | `94c3326c0c4c0166876fc86a62946516ee40600e` | `0df3d146dc54002668a2c8cfb26af9fbf9b5743c` (PR #7) |

### Archived artifact integrity

The following digests describe each review report as committed
in the archival commit that introduced it. They are not digests
of the current files.

| File | Archival commit | Git blob SHA | File size (bytes) | SHA-256 |
|---|---|---|---|---|
| `reviews/governance/REV-2026-11-20-009-independent-review.md` | `94c3326c0c4c0166876fc86a62946516ee40600e` | `148e37e331c49579d8e8f7e7758f3a8c6a90e772` | 7262 | `c30c72ee4161ed6dc0b21f5900deb327aca3a90508ebfdd6e53fbd2c98ce0ee5` |
| `reviews/governance/REV-2026-11-20-001-independent-review.md` | `a7b7e1f8d49f238361c3b5d42f73aec7ec401888` | `cf947b529ae2fb2b5a3d316ffee89593413a536e` | 20177 | `51db503cfd6c3406665620b4081ba16ac175b8eef40785cc58aa22865646f105` |

The review reports were introduced by two separate archival
commits.

The REV-2026-11-20-009 archival commit is:

`94c3326c0c4c0166876fc86a62946516ee40600e`

This commit added two files without modifying or deleting
existing files:

- `reviews/governance/README.md`
- `reviews/governance/REV-2026-11-20-009-independent-review.md`

The REV-2026-11-20-001 archival commit is:

`a7b7e1f8d49f238361c3b5d42f73aec7ec401888`

This commit changed four files: two files were added and two
existing files were modified. The changes included the
REV-2026-11-20-001 report and its associated governance
decision amendment.

The README version introduced by the REV-009 archival commit
did not record that archival commit's SHA. The traceability
entry was subsequently recorded in the governance index.
The REV-001 archival commit and its PR merge commit are also
recorded in the index.

The two archival commits were subsequently incorporated into
`main` through separate pull requests.

The REV-2026-11-20-009 archival commit was incorporated through
PR #7.

PR #7 was merged using the merge method. Its merge commit is:

`0df3d146dc54002668a2c8cfb26af9fbf9b5743c`

The merge commit has the REV-009 archival commit as one of its
parents. The archival commit is therefore an ancestor of the
current main history.

The REV-2026-11-20-001 archival commit was incorporated through
PR #12.

PR #12 was merged using the merge method. Its merge commit is:

`91cd8237ebfb7617c279fa52db536396e968e741`

For each report, the commit that introduced it into the main
branch is the corresponding PR merge commit identified above.
The archival commit identifies the original report-and-index
change, not the subsequent pull-request merge commit.

### Governance structural check evidence

The `Governance structural checks` check run on archival commit
`94c3326c0c4c0166876fc86a62946516ee40600e` completed with
conclusion `success`.

The PR #7 merge commit
`0df3d146dc54002668a2c8cfb26af9fbf9b5743c` has no associated
check-run record.

The `Governance structural checks` check run on archival commit
`a7b7e1f8d49f238361c3b5d42f73aec7ec401888` completed with
conclusion `success`.

The PR #12 merge commit
`91cd8237ebfb7617c279fa52db536396e968e741` has no associated
check-run record.

The workflow `.github/workflows/governance-gate.yml` listens
only to `pull_request` events:

- `opened`
- `synchronize`
- `reopened`
- `ready_for_review`

It does not listen to `push` events.

Consequently, the absence of check-run records on the PR #7
and PR #12 merge commits is consistent with the workflow's
configured event triggers. It establishes neither a successful
nor a failed check on either merge commit.

The successful check-run evidence applies to the two archival
commits identified above, not to either PR merge commit.

This archive therefore does not claim that the Governance
Gate ran on either main-branch merge commit or that a
main-branch gate passed.

## Archival ordering and audit integrity

The archival of `REV-2026-11-20-009` took place after PR #4
was merged.

PR #4 was merged using a merge commit:

- PR: #4
- Reviewed frozen head: `f2bc4d81e66bf819ef6f9c2967c1a54332143b26`
- Merge commit: `c12926b66fbd399ad2b3235c285f47469aa7ee2c`
- Merge method: `merge`

Consequently, the REV-009 archival commit cannot change the
reviewed head SHA of PR #4. The review target is the frozen
head recorded above, not the subsequent archival commit.

The archival operation is an independent, subsequent
audit-record operation and is not part of the reviewed PR's
change set.

The REV-009 archival commit was subsequently merged into
`main` through PR #7. Its merge commit is:

`0df3d146dc54002668a2c8cfb26af9fbf9b5743c`

The REV-009 archival commit and the PR #7 merge commit are
distinct commits with distinct roles in the archival
traceability chain.

The REV-009 report's frozen review target remains PR #4 head
`f2bc4d81e66bf819ef6f9c2967c1a54332143b26`.

Neither the REV-009 archival commit nor the PR #7 merge commit
is substituted for that reviewed head.

The archival of `REV-2026-11-20-001` also took place after
the reviewed PR had been merged.

PR #1 was merged at `2026-11-20T09:26:15Z`. Its reviewed
frozen head was:

`ff1b72a19a55f973ad200665aa74f6eec53764f8`

The REV-001 archival commit is:

`a7b7e1f8d49f238361c3b5d42f73aec7ec401888`

Its parent is commit `ce78d6fc1c1027578f039c76857e6b1266ccc6ed`,
the PR #11 merge commit. This places the REV-001 archival
commit after the PR #1 merge.

The REV-001 report explicitly records in Section 13:

本次评审不满足 L3「独立评审通过后才合并」的时序要求。

本报告是合并后的事后独立评审，只能形成问题记录、补救证据和后续整改约束。

The REV-001 archival commit was subsequently incorporated
into `main` through PR #12. Its merge commit is:

`91cd8237ebfb7617c279fa52db536396e968e741`

The REV-001 archival commit and the PR #12 merge commit are
distinct commits with distinct roles in the archival
traceability chain.

The REV-001 report's reviewed target remains PR #1 head
`ff1b72a19a55f973ad200665aa74f6eec53764f8`. Neither the
REV-001 archival commit nor the PR #12 merge commit is
substituted for that reviewed head.

These two reports therefore document post-merge archival
operations. Neither archival operation changes the frozen
review target or makes the review a pre-merge approval.

## Review-round coverage

This archive contains the following individually archived
review reports:

- **R1:** `REV-2026-11-20-001-independent-review.md`
  - Review target: PR #1 frozen head
    `ff1b72a19a55f973ad200665aa74f6eec53764f8`
  - Verdict: `VERDICT: REVISE`
- **R9:** `REV-2026-11-20-009-independent-review.md`

R2–R8 review reports were never produced. No individual
report files exist for those review rounds.

The existence of the R1 and R9 reports does not imply
that individual R2–R8 report files exist or have been
archived.

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
