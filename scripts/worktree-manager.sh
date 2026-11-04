#!/usr/bin/env bash
# ==============================================================================
# some-stripe — Git Worktree Isolation Manager
# ==============================================================================
# Manages dedicated, isolated git worktrees for concurrent multi-agent execution.
# Prevents file-lock contention, index corruption, and cross-agent workspace collisions.
#
# Usage:
#   ./scripts/worktree-manager.sh create <branch> [base_branch]
#   ./scripts/worktree-manager.sh remove <branch_or_id>
#   ./scripts/worktree-manager.sh list
#   ./scripts/worktree-manager.sh exec <branch_or_id> <command...>
#   ./scripts/worktree-manager.sh clean
# ==============================================================================

set -euo pipefail

export PATH="${PATH}:/home/codespace/.cargo/bin:/home/codespace/.local/bin"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKTREES_DIR="${WORKSPACE_ROOT}/.worktrees"

# Color helpers
BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

mkdir -p "${WORKTREES_DIR}"

sanitize_id() {
    echo "$1" | tr '/' '-' | tr '_' '-' | tr -cd '[:alnum:]-'
}

cmd_create() {
    local branch="${1:-}"
    local base="${2:-main}"

    if [[ -z "${branch}" ]]; then
        echo -e "${RED}Error: branch name required.${NC}"
        echo "Usage: $0 create <branch> [base_branch]"
        exit 1
    fi

    local id
    id="$(sanitize_id "${branch}")"
    local wt_path="${WORKTREES_DIR}/${id}"

    if [[ -d "${wt_path}" ]]; then
        echo -e "${YELLOW}Worktree already exists:${NC} ${wt_path}"
        echo "${wt_path}"
        return 0
    fi

    echo -e "${BLUE}Creating isolated worktree for branch '${CYAN}${branch}${BLUE}' based on '${base}'...${NC}"

    cd "${WORKSPACE_ROOT}"
    if git show-ref --verify --quiet "refs/heads/${branch}"; then
        git worktree add "${wt_path}" "${branch}"
    else
        git worktree add -b "${branch}" "${wt_path}" "${base}"
    fi

    echo -e "${GREEN}✓ Worktree ready:${NC} ${wt_path}"
    echo "${wt_path}"
}

cmd_remove() {
    local target="${1:-}"
    if [[ -z "${target}" ]]; then
        echo -e "${RED}Error: worktree target required.${NC}"
        echo "Usage: $0 remove <branch_or_id>"
        exit 1
    fi

    local id
    id="$(sanitize_id "${target}")"
    local wt_path="${WORKTREES_DIR}/${id}"

    if [[ ! -d "${wt_path}" && -d "${target}" ]]; then
        wt_path="${target}"
    fi

    if [[ -d "${wt_path}" ]]; then
        echo -e "${BLUE}Removing worktree:${NC} ${wt_path}"
        cd "${WORKSPACE_ROOT}"
        git worktree remove --force "${wt_path}" || rm -rf "${wt_path}"
        git worktree prune
        echo -e "${GREEN}✓ Worktree removed.${NC}"
    else
        echo -e "${YELLOW}Worktree not found: ${wt_path}${NC}"
    fi
}

cmd_list() {
    echo -e "${BOLD}${BLUE}Active Git Worktrees for some-stripe:${NC}"
    cd "${WORKSPACE_ROOT}"
    git worktree list
}

cmd_exec() {
    local target="${1:-}"
    shift || true

    if [[ -z "${target}" || $# -eq 0 ]]; then
        echo -e "${RED}Error: target and command required.${NC}"
        echo "Usage: $0 exec <branch_or_id> <command...>"
        exit 1
    fi

    local id
    id="$(sanitize_id "${target}")"
    local wt_path="${WORKTREES_DIR}/${id}"

    if [[ ! -d "${wt_path}" && -d "${target}" ]]; then
        wt_path="${target}"
    fi

    if [[ ! -d "${wt_path}" ]]; then
        echo -e "${RED}Worktree not found: ${wt_path}${NC}"
        exit 1
    fi

    cd "${wt_path}"
    exec "$@"
}

cmd_clean() {
    echo -e "${BLUE}Cleaning up stale worktrees and pruning metadata...${NC}"
    cd "${WORKSPACE_ROOT}"
    git worktree prune -v
    echo -e "${GREEN}✓ Pruning complete.${NC}"
}

ACTION="${1:-list}"
shift || true

case "${ACTION}" in
    create)
        cmd_create "$@"
        ;;
    remove|rm)
        cmd_remove "$@"
        ;;
    list|ls)
        cmd_list
        ;;
    exec)
        cmd_exec "$@"
        ;;
    clean|prune)
        cmd_clean
        ;;
    *)
        echo "Usage: $0 {create|remove|list|exec|clean}"
        exit 1
        ;;
esac
