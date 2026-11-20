"""Fail-closed governance gate for AiMemoryBrain."""

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GIT_ROOT = ROOT.parent

ORIGINAL = "governance/decisions/DEC-RD-2026-11-20-001.md"

BASELINE = [
    "governance/N01-reuse-first-principle.md",
    "governance/N06-risk-tiered-decision-depth.md",
    ORIGINAL,
]


def git(*args):
    return subprocess.run(
        ["git", "-C", str(GIT_ROOT), *args],
        capture_output=True,
        text=True,
    )


def check(name, condition, results):
    results.append(bool(condition))
    print(f"{'PASS' if condition else 'FAIL'}: {name}")
    return int(not condition)


def main():
    base = os.environ.get("BASE_SHA", "").strip()
    head = os.environ.get("HEAD_SHA", "").strip()

    if not base or not head:
        print("ERROR: BASE_SHA and HEAD_SHA are required")
        return 2

    root = git("rev-parse", "--show-toplevel")
    if root.returncode or Path(root.stdout.strip()).resolve() != GIT_ROOT:
        print("ERROR: script placement does not match Git root contract")
        return 2

    for ref in (base, head):
        result = git("cat-file", "-e", f"{ref}^{{commit}}")
        if result.returncode:
            print(f"ERROR: invalid commit reference: {ref}")
            return 2

    failures = 0
    results = []

    # Verify baseline files in both actual commit trees.
    for path in BASELINE:
        git_path = f"AiMemoryBrain/{path}"

        historical = git("cat-file", "-s", f"{base}:{git_path}")
        current = git("cat-file", "-s", f"{head}:{git_path}")

        valid = (
            historical.returncode == 0
            and current.returncode == 0
            and historical.stdout.strip().isdigit()
            and current.stdout.strip().isdigit()
            and int(historical.stdout.strip()) > 0
            and int(current.stdout.strip()) > 0
        )

        failures += check(
            f"PR base/head 基线文件存在且非空：{path}",
            valid,
            results,
        )

    # Verify decision directory in the committed head tree.
    directory_path = "AiMemoryBrain/governance/decisions"

    directory = git("cat-file", "-e", f"{head}:{directory_path}")

    failures += check(
        "head 决策目录存在",
        directory.returncode == 0,
        results,
    )

    # Enumerate committed decision records, excluding untracked files.
    listing = git(
        "ls-tree",
        "-r",
        "--name-only",
        head,
        "--",
        directory_path,
    )

    if listing.returncode:
        print(f"ERROR: git ls-tree failed: {listing.stderr.strip()}")
        return 2

    records = [
        name
        for name in listing.stdout.splitlines()
        if name.startswith(directory_path + "/")
        and Path(name).name.startswith("DEC-")
        and name.endswith(".md")
    ]

    failures += check(
        "head 决策记录集合非空",
        bool(records),
        results,
    )

    # G-04: compare committed PR base and head only.
    changed = git(
        "diff",
        "--quiet",
        base,
        head,
        "--",
        f"AiMemoryBrain/{ORIGINAL}",
    )

    if changed.returncode not in (0, 1):
        print(f"ERROR: git diff failed: {changed.stderr.strip()}")
        return 2

    failures += check(
        "本次 PR 未修改原始决策记录正文",
        changed.returncode == 0,
        results,
    )

    # Check committed PR changes.
    committed = git("diff", "--check", base, head)

    if committed.returncode not in (0,):
        failures += check(
            "提交范围 Git diff 空白及补丁格式检查",
            False,
            results,
        )
    else:
        failures += check(
            "提交范围 Git diff 空白及补丁格式检查",
            True,
            results,
        )

    # Check differences between HEAD and working tree.
    worktree_diff = git("diff", "--check", head)

    failures += check(
        "HEAD 与工作区 Git diff 空白及补丁格式检查",
        worktree_diff.returncode == 0,
        results,
    )

    print(
        f"\n结果: {len(results)} 检查, "
        f"{failures} 失败"
    )

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
