#!/usr/bin/env bash
#
# Configures branch protection on `main` via the GitHub CLI.
#
# Run once, after the remote repository exists:
#   gh auth login
#   tools/setup-branch-protection.sh [owner/repo]
#
# Idempotent: re-running updates the rules to match this file, which makes this
# script the source of truth for the repository's merge policy rather than a
# setting somebody changed in the web UI eighteen months ago.
set -euo pipefail

BOLD=$'\033[1m'; RESET=$'\033[0m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'
info() { echo "${BOLD}==>${RESET} $*"; }
ok()   { echo "  ${GREEN}✓${RESET} $*"; }
warn() { echo "  ${YELLOW}!${RESET} $*"; }

command -v gh >/dev/null 2>&1 || { echo "gh CLI not found: https://cli.github.com"; exit 1; }
gh auth status >/dev/null 2>&1 || { echo "Not authenticated. Run: gh auth login"; exit 1; }

REPO="${1:-$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || true)}"
[ -n "$REPO" ] || { echo "Could not determine the repository. Pass it as: $0 owner/repo"; exit 1; }

BRANCH="main"
info "Configuring branch protection for ${BOLD}${REPO}@${BRANCH}${RESET}"

# `ci-success` is the single aggregate job from ci.yml. Requiring that one check
# rather than each matrix leg means adding a platform to the matrix does not
# require touching this configuration — and a skipped or cancelled leg still
# fails the aggregate.
REQUIRED_CHECK="CI"

# GitHub does not allow you to approve your own pull request. On a solo project
# a non-zero review requirement is therefore an unmergeable branch, so the
# default is 0: CI is the gate, review is not yet available.
#
# Raise this the day a second maintainer joins:
#   REQUIRED_APPROVALS=1 tools/setup-branch-protection.sh
REQUIRED_APPROVALS="${REQUIRED_APPROVALS:-0}"

gh api \
    --method PUT \
    -H "Accept: application/vnd.github+json" \
    "/repos/${REPO}/branches/${BRANCH}/protection" \
    --input - <<EOF
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["${REQUIRED_CHECK}"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false,
    "required_approving_review_count": ${REQUIRED_APPROVALS}
  },
  "restrictions": null,
  "required_linear_history": true,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_conversation_resolution": true
}
EOF

ok "protection applied"
echo
info "Policy now in force on ${BRANCH}:"
echo "  • Pull request required; direct pushes rejected"
echo "  • CI must pass, and the branch must be up to date with main"
echo "  • Required approving reviews: ${REQUIRED_APPROVALS}"
echo "  • Linear history; no force pushes; no branch deletion"
echo "  • Conversations must be resolved before merge"
echo
if [ "$REQUIRED_APPROVALS" -eq 0 ]; then
    warn "Review requirement is 0 because you cannot approve your own pull request."
    warn "CI is the gate for now. When a second maintainer joins, re-run as:"
    warn "    REQUIRED_APPROVALS=1 $0"
    warn "and set require_code_owner_reviews to true in this script — the"
    warn "CODEOWNERS entries for wire contracts, storage, secrets, and the"
    warn "updater only mean something once somebody else can review them."
fi
echo
warn "enforce_admins is false, so you can bypass these rules in an emergency."
warn "Set it to true once a second maintainer exists; doing it now would lock"
warn "you out of your own repository with nobody available to let you back in."
