# `repo-activity-summary.py`

This summarizes PR/issues during a time period.

This can be helpful for showing progress and activity in the repo,
including by the community, in presentations,
so this automates it and gets it precisely right.

It uses the GitHub CLI `gh` to query the GitHub API
and summarize all the opened/merged PRs and opened/closed issues,
including grouping them between collaborators vs. the community.

## Usage

```sh
❯ ./repo_activity_summary.py --help
usage: repo_activity_summary.py [-h] [--repo REPO] [--after AFTER] [--before BEFORE] [--list] [--datetime-format DATETIME_FORMAT] [--cache]

summarize repo activity (PR/issues) during a time period (requires gh)

options:
  -h, --help            show this help message and exit
  --repo REPO           the GitHub repo (defaults to the current repo)
  --after AFTER         summarize after this date
  --before BEFORE       summarize before this date
  --list                list each PR/issue
  --datetime-format DATETIME_FORMAT
                        a strftime format string
  --cache               cache gh API results
```

## Example

```sh
❯ ./repo_activity_summary.py --repo immunant/c2rust --after 9/9/2022

opened 74 PRs
        by collaborators: 62
        by community: 12

merged 54 PRs
        by collaborators: 48
        by community: 6

opened 55 issues
        by collaborators: 24
        by community: 31

closed 24 issues
        by collaborators: 8
        by community: 16
```
