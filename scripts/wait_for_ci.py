#!/usr/bin/env python3
"""
Wait for GitHub Actions CI to complete for the current branch.

This script:
1. Detects the current git branch
2. Finds the latest GitHub Actions workflow run for that branch
3. Waits for it to complete
4. If it fails, fetches and displays the error logs

Usage:
    python scripts/wait_for_ci.py

Environment:
    GITHUB_TOKEN: Optional for public repos, required for private repos.
                  Provides higher rate limits (5000/hr vs 60/hr unauthenticated).
                  Can be a personal access token or GITHUB_TOKEN from Actions.

Example:
    git push origin my-branch
    python scripts/wait_for_ci.py

    # Or with authentication for higher rate limits:
    export GITHUB_TOKEN=ghp_xxxxxxxxxxxx
    python scripts/wait_for_ci.py
"""

import os
import sys
import time
import subprocess
import zipfile
import io
import re
from typing import Optional

try:
    import requests
except ImportError:
    print("Error: 'requests' library is required.")
    print("Install with: pip install requests")
    sys.exit(1)


# Configuration
POLL_INTERVAL_SECONDS = 60
MAX_WAIT_SECONDS = 3600  # 1 hour max wait
WORKFLOW_NAME = "Tests"  # Name from .github/workflows/tests.yml


def get_github_token() -> Optional[str]:
    """Get GitHub token from environment, if available."""
    return os.environ.get("GITHUB_TOKEN")


def run_git_command(args: list[str]) -> str:
    """Run a git command and return the output."""
    result = subprocess.run(
        ["git"] + args,
        capture_output=True,
        text=True,
        check=True
    )
    return result.stdout.strip()


def get_current_branch() -> str:
    """Get the current git branch name."""
    return run_git_command(["rev-parse", "--abbrev-ref", "HEAD"])


def get_repo_info() -> tuple[str, str]:
    """Get the GitHub owner and repo from the remote URL."""
    remote_url = run_git_command(["remote", "get-url", "origin"])

    # Handle various URL formats:
    # SSH: git@github.com:owner/repo.git
    # HTTPS: https://github.com/owner/repo.git
    # Proxy: http://proxy@host:port/git/owner/repo
    if remote_url.startswith("git@"):
        # SSH format
        match = re.search(r"git@github\.com:(.+)/(.+?)(?:\.git)?$", remote_url)
    elif "github.com" in remote_url:
        # HTTPS format
        match = re.search(r"github\.com/(.+)/(.+?)(?:\.git)?$", remote_url)
    else:
        # Try to extract from path (e.g., proxy URLs like /git/owner/repo)
        match = re.search(r"/git/(.+)/(.+?)(?:\.git)?$", remote_url)

    if not match:
        print(f"Error: Could not parse GitHub repo from remote URL: {remote_url}")
        sys.exit(1)

    return match.group(1), match.group(2)


def get_latest_head_sha() -> str:
    """Get the HEAD commit SHA."""
    return run_git_command(["rev-parse", "HEAD"])


class GitHubAPI:
    """Simple GitHub API client."""

    def __init__(self, owner: str, repo: str, token: Optional[str] = None):
        self.token = token
        self.owner = owner
        self.repo = repo
        self.base_url = f"https://api.github.com/repos/{owner}/{repo}"
        self.headers = {
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        }
        if token:
            self.headers["Authorization"] = f"Bearer {token}"

    def get(self, endpoint: str, params: Optional[dict] = None) -> dict:
        """Make a GET request to the GitHub API."""
        url = f"{self.base_url}{endpoint}"
        response = requests.get(url, headers=self.headers, params=params)
        response.raise_for_status()
        return response.json()

    def get_raw(self, endpoint: str) -> bytes:
        """Make a GET request and return raw bytes."""
        url = f"{self.base_url}{endpoint}"
        headers = self.headers.copy()
        headers["Accept"] = "application/vnd.github+json"
        response = requests.get(url, headers=headers)
        response.raise_for_status()
        return response.content

    def get_workflow_runs(self, branch: str, head_sha: Optional[str] = None) -> list[dict]:
        """Get workflow runs for a branch, optionally filtered by commit SHA."""
        params = {
            "branch": branch,
            "per_page": 10,
        }
        if head_sha:
            params["head_sha"] = head_sha

        result = self.get("/actions/runs", params=params)
        return result.get("workflow_runs", [])

    def get_workflow_run(self, run_id: int) -> dict:
        """Get a specific workflow run."""
        return self.get(f"/actions/runs/{run_id}")

    def get_workflow_jobs(self, run_id: int) -> list[dict]:
        """Get jobs for a workflow run."""
        result = self.get(f"/actions/runs/{run_id}/jobs")
        return result.get("jobs", [])

    def get_job_logs(self, job_id: int) -> str:
        """Get logs for a specific job."""
        url = f"{self.base_url}/actions/jobs/{job_id}/logs"
        response = requests.get(url, headers=self.headers, allow_redirects=True)
        response.raise_for_status()
        return response.text

    def get_run_logs(self, run_id: int) -> dict[str, str]:
        """Download and extract logs for a workflow run.

        Returns a dict mapping job names to their log content.
        """
        url = f"{self.base_url}/actions/runs/{run_id}/logs"
        response = requests.get(url, headers=self.headers, allow_redirects=True)
        response.raise_for_status()

        logs = {}
        with zipfile.ZipFile(io.BytesIO(response.content)) as zf:
            for name in zf.namelist():
                # Log files are named like "build/1_Build.txt"
                logs[name] = zf.read(name).decode("utf-8", errors="replace")
        return logs


def find_latest_run_for_branch(api: GitHubAPI, branch: str, head_sha: str) -> Optional[dict]:
    """Find the latest workflow run for the given branch and commit."""
    runs = api.get_workflow_runs(branch, head_sha)

    if not runs:
        # Try without head_sha filter in case the push just happened
        runs = api.get_workflow_runs(branch)

    # Filter to the Tests workflow and find the latest
    test_runs = [r for r in runs if r.get("name") == WORKFLOW_NAME]

    if not test_runs:
        return None

    # Sort by created_at descending and return the first
    test_runs.sort(key=lambda r: r.get("created_at", ""), reverse=True)
    return test_runs[0]


def format_duration(seconds: int) -> str:
    """Format seconds as a human-readable duration."""
    if seconds < 60:
        return f"{seconds}s"
    minutes = seconds // 60
    secs = seconds % 60
    if minutes < 60:
        return f"{minutes}m {secs}s"
    hours = minutes // 60
    mins = minutes % 60
    return f"{hours}h {mins}m {secs}s"


def extract_failed_step_logs(logs: dict[str, str]) -> str:
    """Extract relevant portions of logs for failed steps."""
    output_lines = []

    for filename, content in sorted(logs.items()):
        # Skip setup steps, focus on build/test steps
        if any(skip in filename.lower() for skip in ["checkout", "cache", "setup"]):
            continue

        lines = content.split("\n")

        # Look for error indicators
        error_sections = []
        in_error = False
        error_start = 0

        for i, line in enumerate(lines):
            lower_line = line.lower()

            # Detect error starts
            if any(indicator in lower_line for indicator in [
                "error[", "error:", "failed", "panicked",
                "assertion failed", "thread 'main' panicked",
                "cannot find", "not found", "fatal:"
            ]):
                if not in_error:
                    in_error = True
                    # Include some context before the error
                    error_start = max(0, i - 5)

            # Collect error context
            if in_error:
                # Stop if we've collected enough lines or hit a new section
                if i - error_start > 100 or (
                    line.startswith("##[group]") and i > error_start + 10
                ):
                    error_sections.append((error_start, i))
                    in_error = False

        # Close any open error section
        if in_error:
            error_sections.append((error_start, min(len(lines), error_start + 100)))

        # Extract error sections
        if error_sections:
            output_lines.append(f"\n{'='*60}")
            output_lines.append(f"File: {filename}")
            output_lines.append("=" * 60)

            for start, end in error_sections:
                output_lines.extend(lines[start:end])
                output_lines.append("...")

    if not output_lines:
        # If no errors found, return last 50 lines of each log
        for filename, content in sorted(logs.items()):
            lines = content.split("\n")
            output_lines.append(f"\n{'='*60}")
            output_lines.append(f"File: {filename} (last 50 lines)")
            output_lines.append("=" * 60)
            output_lines.extend(lines[-50:])

    return "\n".join(output_lines)


def print_status(run: dict, elapsed: int) -> None:
    """Print the current status of a workflow run."""
    status = run.get("status", "unknown")
    conclusion = run.get("conclusion")
    run_url = run.get("html_url", "")

    status_str = status
    if conclusion:
        status_str = f"{status} ({conclusion})"

    print(f"\r[{format_duration(elapsed)}] Status: {status_str}    ", end="", flush=True)


def wait_for_completion(api: GitHubAPI, run: dict) -> dict:
    """Wait for a workflow run to complete."""
    run_id = run["id"]
    run_url = run.get("html_url", "")

    print(f"\nWorkflow run: {run_url}")
    print(f"Commit: {run.get('head_sha', 'unknown')[:8]}")
    print(f"Waiting for completion...\n")

    start_time = time.time()
    last_status = None

    while True:
        elapsed = int(time.time() - start_time)

        if elapsed > MAX_WAIT_SECONDS:
            print(f"\n\nTimeout: Workflow did not complete within {format_duration(MAX_WAIT_SECONDS)}")
            sys.exit(1)

        run = api.get_workflow_run(run_id)
        status = run.get("status")
        conclusion = run.get("conclusion")

        print_status(run, elapsed)

        if status == "completed":
            print()  # New line after status
            return run

        time.sleep(POLL_INTERVAL_SECONDS)


def print_job_summary(api: GitHubAPI, run: dict) -> None:
    """Print a summary of jobs in the workflow run."""
    run_id = run["id"]
    jobs = api.get_workflow_jobs(run_id)

    print("\nJob Summary:")
    print("-" * 40)

    for job in jobs:
        name = job.get("name", "unknown")
        conclusion = job.get("conclusion", "unknown")

        # Emoji for status
        if conclusion == "success":
            icon = "✓"
        elif conclusion == "failure":
            icon = "✗"
        elif conclusion == "cancelled":
            icon = "○"
        else:
            icon = "?"

        print(f"  {icon} {name}: {conclusion}")

    print()


def show_failure_logs(api: GitHubAPI, run: dict) -> None:
    """Download and display logs for a failed run."""
    run_id = run["id"]

    print("Downloading logs...")
    try:
        logs = api.get_run_logs(run_id)
        print("\n" + "=" * 60)
        print("FAILURE LOGS")
        print("=" * 60)

        error_logs = extract_failed_step_logs(logs)
        print(error_logs)

    except Exception as e:
        print(f"Error downloading logs: {e}")

        # Fallback: try to get individual job logs
        print("\nTrying to get job logs individually...")
        jobs = api.get_workflow_jobs(run_id)

        for job in jobs:
            if job.get("conclusion") == "failure":
                try:
                    job_logs = api.get_job_logs(job["id"])
                    print(f"\n{'='*60}")
                    print(f"Job: {job['name']}")
                    print("=" * 60)
                    # Print last 100 lines
                    lines = job_logs.split("\n")
                    print("\n".join(lines[-100:]))
                except Exception as je:
                    print(f"  Could not get logs for {job['name']}: {je}")


def main():
    """Main entry point."""
    print("GitHub Actions CI Watcher")
    print("=" * 40)

    # Get configuration
    token = get_github_token()
    branch = get_current_branch()
    owner, repo = get_repo_info()
    head_sha = get_latest_head_sha()

    print(f"Repository: {owner}/{repo}")
    print(f"Branch: {branch}")
    print(f"Commit: {head_sha[:8]}")
    if token:
        print("Auth: Using GITHUB_TOKEN")
    else:
        print("Auth: Unauthenticated (lower rate limits)")

    api = GitHubAPI(owner, repo, token)

    # Wait a moment for GitHub to register the push
    print("\nWaiting for workflow to start...")

    run = None
    for attempt in range(10):  # Try for up to 30 seconds
        run = find_latest_run_for_branch(api, branch, head_sha)
        if run:
            break
        time.sleep(3)

    if not run:
        print(f"\nNo workflow run found for branch '{branch}'")
        print("Make sure you've pushed your changes and the workflow is triggered.")
        sys.exit(1)

    # Wait for completion
    run = wait_for_completion(api, run)

    conclusion = run.get("conclusion", "unknown")
    run_url = run.get("html_url", "")

    # Print job summary
    print_job_summary(api, run)

    if conclusion == "success":
        print("=" * 60)
        print("✓ CI PASSED")
        print("=" * 60)
        print(f"\nWorkflow URL: {run_url}")
        sys.exit(0)

    elif conclusion == "failure":
        print("=" * 60)
        print("✗ CI FAILED")
        print("=" * 60)
        print(f"\nWorkflow URL: {run_url}")

        # Show failure logs
        show_failure_logs(api, run)
        sys.exit(1)

    else:
        print(f"\nWorkflow completed with conclusion: {conclusion}")
        print(f"Workflow URL: {run_url}")
        sys.exit(1)


if __name__ == "__main__":
    main()
