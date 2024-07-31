#!/usr/bin/env python3

"""
Summarizes recent PRs based on their GitHub labels.

The result can be copy-pasted into CHANGELOG.md,
though it often needs some manual editing too.
"""

import argparse
import multiprocessing
import os
import re
import sys

from collections import defaultdict
from datetime import date
from dataclasses import dataclass
from typing import Any, List, Optional

import requests
from git import Repo  # pip install GitPython
from tqdm import tqdm

OWNER = "emilk"
REPO = "egui"
INCLUDE_LABELS = False  # It adds quite a bit of visual noise


@dataclass
class PrInfo:
    pr_number: int
    gh_user_name: str
    title: str
    labels: List[str]


@dataclass
class CommitInfo:
    hexsha: str
    title: str
    pr_number: Optional[int]


def get_github_token() -> str:
    import os

    token = os.environ.get("GH_ACCESS_TOKEN", "")
    if token != "":
        return token

    home_dir = os.path.expanduser("~")
    token_file = os.path.join(home_dir, ".githubtoken")

    try:
        with open(token_file, "r") as f:
            token = f.read().strip()
        return token
    except Exception:
        pass

    print(
        "ERROR: expected a GitHub token in the environment variable GH_ACCESS_TOKEN or in ~/.githubtoken"
    )
    sys.exit(1)


# Slow
def fetch_pr_info_from_commit_info(commit_info: CommitInfo) -> Optional[PrInfo]:
    if commit_info.pr_number is None:
        return None
    else:
        return fetch_pr_info(commit_info.pr_number)


# Slow
def fetch_pr_info(pr_number: int) -> Optional[PrInfo]:
    url = f"https://api.github.com/repos/{OWNER}/{REPO}/pulls/{pr_number}"
    gh_access_token = get_github_token()
    headers = {"Authorization": f"Token {gh_access_token}"}
    response = requests.get(url, headers=headers)
    json = response.json()

    # Check if the request was successful (status code 200)
    if response.status_code == 200:
        labels = [label["name"] for label in json["labels"]]
        gh_user_name = json["user"]["login"]
        return PrInfo(pr_number=pr_number, gh_user_name=gh_user_name, title=json["title"], labels=labels)
    else:
        print(f"ERROR {url}: {response.status_code} - {json['message']}")
        return None


def get_commit_info(commit: Any) -> CommitInfo:
    match = re.match(r"(.*) \(#(\d+)\)", commit.summary)
    if match:
        title = str(match.group(1))
        pr_number = int(match.group(2))
        return CommitInfo(hexsha=commit.hexsha, title=title, pr_number=pr_number)
    else:
        return CommitInfo(hexsha=commit.hexsha, title=commit.summary, pr_number=None)


def pr_summary(pr: PrInfo, crate_name: Optional[str] = None) -> str:
    title = pr.title

    if crate_name is not None:
        # Remove crate name prefix (common in PR titles):
        title = remove_prefix(title, f"[{crate_name}] ")
        title = remove_prefix(title, f"{crate_name}: ")
        title = remove_prefix(title, f"`{crate_name}`: ")

    # Upper-case first letter:
    title = title[0].upper() + title[1:]

    # Remove trailing periods:
    title = title.rstrip(".")

    summary = f"{title} [#{pr.pr_number}](https://github.com/{OWNER}/{REPO}/pull/{pr.pr_number})"

    if INCLUDE_LABELS and 0 < len(pr.labels):
        summary += f" ({', '.join(pr.labels)})"

    summary += f" by [@{pr.gh_user_name}](https://github.com/{pr.gh_user_name})"

    return summary


def pr_info_section(prs: List[PrInfo], *, crate_name: str, heading: Optional[str] = None) -> str:
    result = ""
    if 0 < len(prs):
        if heading is not None:
            result += f"### {heading}\n"
        for pr in prs:
            result += f"* {pr_summary(pr, crate_name)}\n"
        result += "\n"
    return result


def changelog_from_prs(pr_infos: List[PrInfo], crate_name: str) -> str:
    if len(pr_infos) == 0:
        return "Nothing new"

    if len(pr_infos) <= 5:
        # For small crates, or small releases
        return pr_info_section(pr_infos, crate_name=crate_name)


    fixed = []
    added = []
    rest = []
    for pr in pr_infos:
        summary = pr_summary(pr, crate_name)
        if "bug" in pr.labels:
            fixed.append(pr)
        elif summary.startswith("Add") or "feature" in pr.labels:
            added.append(pr)
        else:
            rest.append(pr)

    result = ""

    result += pr_info_section(added, crate_name=crate_name, heading="⭐ Added")
    result += pr_info_section(rest, crate_name=crate_name, heading="🔧 Changed")
    result += pr_info_section(fixed, crate_name=crate_name, heading="🐛 Fixed")

    return result.rstrip()


def remove_prefix(text, prefix):
    if text.startswith(prefix):
        return text[len(prefix) :]
    return text  # or whatever


def print_section(heading: str, content: str) -> None:
    if content != "":
        print(f"## {heading}")
        print(content)
        print()


def changelog_filepath(crate: str) -> str:
    scripts_dirpath = os.path.dirname(os.path.realpath(__file__))
    if crate == "egui":
        file_path = f"{scripts_dirpath}/../CHANGELOG.md"
    else:
        file_path = f"{scripts_dirpath}/../crates/{crate}/CHANGELOG.md"
    return os.path.normpath(file_path)


def add_to_changelog_file(crate: str, content: str, version: str) -> None:
    insert_text = f"\n## {version} - {date.today()}\n"
    insert_text += content.strip()
    insert_text += "\n\n"

    file_path = changelog_filepath(crate)

    with open(file_path, 'r') as file:
        content = file.read()

    position = content.find('\n##')
    assert position != -1

    content = content[:position] + insert_text + content[position:]

    with open(file_path, 'w') as file:
        file.write(content)


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate a changelog.")
    parser.add_argument("--commit-range", help="e.g. 0.24.0..HEAD", required=True)
    parser.add_argument("--write", help="Write into the different changelogs?", action="store_true")
    parser.add_argument("--version", help="What release is this?")
    args = parser.parse_args()

    if args.write and not args.version:
        print("ERROR: --version is required when --write is used")
        sys.exit(1)

    crate_names = [
        "ecolor",
        "eframe",
        "egui_extras",
        "egui_glow",
        "egui-wgpu",
        "egui-winit",
        "egui",
        "epaint",
        "epaint_default_fonts",
    ]

    # We read all existing changelogs to remove duplicate entries.
    # For instance: the PRs that were part of 0.27.2 would also show up in the diff for `0.27.0..HEAD`
    # when its time for a 0.28 release. We can't do `0.27.2..HEAD` because we would miss PRs that were
    # merged before in `0.27.0..0.27.2` that were not cherry-picked into `0.27.2`.
    all_changelogs = ""
    for crate in crate_names:
        file_path = changelog_filepath(crate)
        with open(file_path, 'r') as file:
            all_changelogs += file.read()

    repo = Repo(".")
    commits = list(repo.iter_commits(args.commit_range))
    commits.reverse()  # Most recent last
    commit_infos = list(map(get_commit_info, commits))

    pool = multiprocessing.Pool()
    pr_infos = list(
        tqdm(
            pool.imap(fetch_pr_info_from_commit_info, commit_infos),
            total=len(commit_infos),
            desc="Fetch PR info commits",
        )
    )

    ignore_labels = ["CI", "dependencies"]

    crate_sections = defaultdict(list)
    unsorted_prs = []
    unsorted_commits = []

    for commit_info, pr_info in zip(commit_infos, pr_infos):
        hexsha = commit_info.hexsha
        title = commit_info.title
        title = title.rstrip(".").strip()  # Some PR end with an unnecessary period
        pr_number = commit_info.pr_number

        if pr_number is None:
            # Someone committed straight to main:
            summary = f"{title} [{hexsha[:7]}](https://github.com/{OWNER}/{REPO}/commit/{hexsha})"
            unsorted_commits.append(summary)
        else:
            if f"[#{pr_number}]" in all_changelogs:
                print(f"* Ignoring PR that is already in the changelog: [#{pr_number}](https://github.com/{OWNER}/{REPO}/pull/{pr_number})")
                continue

            assert pr_info is not None

            if "exclude from changelog" in pr_info.labels:
                continue
            if "typo" in pr_info.labels:
                # We get so many typo PRs. Let's not flood the changelog with them.
                continue

            added = False

            for crate in crate_names:
                if crate in pr_info.labels:
                    crate_sections[crate].append(pr_info)
                    added = True

            if not added:
                if not any(label in pr_info.labels for label in ignore_labels):
                    unsorted_prs.append(pr_summary(pr_info))


    print()
    print(f"Full diff at https://github.com/emilk/egui/compare/{args.commit_range}")
    print()
    for crate in crate_names:
        if crate in crate_sections:
            prs = crate_sections[crate]
            print_section(crate, changelog_from_prs(prs, crate))
    print_section("Unsorted PRs", "\n".join([f"* {item}" for item in unsorted_prs]))
    print_section("Unsorted commits", "\n".join([f"* {item}" for item in unsorted_commits]))

    if args.write:
        for crate in crate_names:
            items = changelog_from_prs(crate_sections[crate], crate)
            add_to_changelog_file(crate, items, args.version)



if __name__ == "__main__":
    main()
