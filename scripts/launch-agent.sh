#!/usr/bin/env bash
# ==============================================================================
# Multi-Account Agent Launcher for some-stripe
# ==============================================================================
# Usage:
#   ./scripts/launch-agent.sh 1 [microkernel-architect] [--worktree <branch>]
#   ./scripts/launch-agent.sh 2 [security-webhook-engineer] [--worktree <branch>]
#   ./scripts/launch-agent.sh 3 [domain-codegen-specialist] [--worktree <branch>]
#   ./scripts/launch-agent.sh agent-a [agent_name] [--worktree <branch>]
#   ./scripts/launch-agent.sh --list-rules
#   ./scripts/launch-agent.sh --list-agents
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RULES_DIR="${WORKSPACE_ROOT}/.agents/rules"
AGENTS_DIR="${WORKSPACE_ROOT}/.agents/agents"

BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Helper to list rules
list_rules() {
    echo -e "${BOLD}${BLUE}Available Workspace Rules in some-stripe:${NC}"
    if [[ -d "${RULES_DIR}" ]]; then
        for rule_file in "${RULES_DIR}"/*.md; do
            if [[ -f "${rule_file}" ]]; then
                rule_name="$(basename "${rule_file}" .md)"
                rule_title="$(head -n 1 "${rule_file}" | sed 's/^# //')"
                echo -e "  • ${GREEN}${rule_name}${NC}: ${rule_title}"
            fi
        done
    fi
    if [[ -f "${WORKSPACE_ROOT}/AGENTS.md" ]]; then
        echo -e "  • ${GREEN}AGENTS.md${NC}: Root workspace agents constitution and quality standards"
    fi
}

# Helper to list agents
list_agents() {
    echo -e "${BOLD}${BLUE}Available Agents in some-stripe:${NC}"
    if [[ -d "${AGENTS_DIR}" ]]; then
        for agent_file in "${AGENTS_DIR}"/*.md; do
            if [[ -f "${agent_file}" ]]; then
                agent_name="$(basename "${agent_file}" .md)"
                echo -e "  • ${CYAN}${agent_name}${NC}"
            fi
        done
    fi
}

# Pre-parse flags for list options
if [[ "${1:-}" == "--list-rules" ]]; then
    list_rules
    exit 0
fi

if [[ "${1:-}" == "--list-agents" ]]; then
    list_agents
    exit 0
fi

# Extract optional --worktree argument from anywhere in args
WORKTREE_BRANCH=""
FILTERED_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --worktree)
            WORKTREE_BRANCH="${2:-}"
            shift 2
            ;;
        *)
            FILTERED_ARGS+=("$1")
            shift
            ;;
    esac
done

TARGET="${FILTERED_ARGS[0]:-}"
AGENT_OVERRIDE="${FILTERED_ARGS[1]:-}"
REMAINING_ARGS=("${FILTERED_ARGS[@]:2}")

# Normalize account number or profile name
ACCOUNT_NUM=""
case "$TARGET" in
    1|agent-a|account1)
        ACCOUNT_NUM="1"
        PROFILE_NAME="agent-a"
        DEFAULT_AGENT="microkernel-architect"
        ;;
    2|agent-b|account2)
        ACCOUNT_NUM="2"
        PROFILE_NAME="agent-b"
        DEFAULT_AGENT="security-webhook-engineer"
        ;;
    3|agent-c|account3)
        ACCOUNT_NUM="3"
        PROFILE_NAME="domain-codegen-specialist"
        ;;
    *)
        echo -e "${YELLOW}Usage: $0 <account_number: 1|2|3 | profile: agent-a|agent-b|agent-c> [agent_name] [--worktree <branch>]${NC}"
        echo ""
        echo -e "${BOLD}Profiles:${NC}"
        echo -e "  1 | agent-a -> microkernel-architect      (Account 1: Core Runtime & Serialization)"
        echo -e "  2 | agent-b -> security-webhook-engineer  (Account 2: Cryptographic Engine & Edge Security)"
        echo -e "  3 | agent-c -> domain-codegen-specialist  (Account 3: OpenAPI Domain Plugins & Tests)"
        echo ""
        list_rules
        echo ""
        echo "Use --list-agents to view all available agent roles."
        exit 1
        ;;
esac

AGENT_NAME="${AGENT_OVERRIDE:-$DEFAULT_AGENT}"
PROFILE_DIR="${HOME}/.gemini-profiles/account${ACCOUNT_NUM}"

mkdir -p "${PROFILE_DIR}/.gemini/antigravity-cli"
mkdir -p "${PROFILE_DIR}/.gemini/config/rules"

# 1. Sync global settings if available
if [[ -f "${HOME}/.gemini/antigravity-cli/settings.json" ]]; then
    cp "${HOME}/.gemini/antigravity-cli/settings.json" "${PROFILE_DIR}/.gemini/antigravity-cli/settings.json"
fi

if [[ -f "${HOME}/.gemini/config/mcp_config.json" ]]; then
    cp "${HOME}/.gemini/config/mcp_config.json" "${PROFILE_DIR}/.gemini/config/mcp_config.json"
fi

# 2. Sync and link all workspace rules into the agent profile config
RULE_COUNT=0
if [[ -d "${RULES_DIR}" ]]; then
    for rule_file in "${RULES_DIR}"/*.md; do
        if [[ -f "${rule_file}" ]]; then
            rule_basename="$(basename "${rule_file}")"
            ln -sf "${rule_file}" "${PROFILE_DIR}/.gemini/config/rules/${rule_basename}"
            RULE_COUNT=$((RULE_COUNT + 1))
        fi
    done
fi

if [[ -f "${WORKSPACE_ROOT}/AGENTS.md" ]]; then
    ln -sf "${WORKSPACE_ROOT}/AGENTS.md" "${PROFILE_DIR}/.gemini/config/rules/AGENTS.md"
    RULE_COUNT=$((RULE_COUNT + 1))
fi

# 3. Worktree isolation setup
EXEC_DIR="${WORKSPACE_ROOT}"
if [[ -n "${WORKTREE_BRANCH}" ]]; then
    EXEC_DIR="$("${WORKSPACE_ROOT}/scripts/worktree-manager.sh" create "${WORKTREE_BRANCH}")"
fi

echo -e "${BOLD}${BLUE}======================================================================${NC}"
echo -e "${BOLD}${BLUE} Launching Agent Profile for Google AI Pro Account #${ACCOUNT_NUM} (${PROFILE_NAME})${NC}"
echo -e " Agent:       ${CYAN}${AGENT_NAME}${NC}"
echo -e " Profile Dir: ${PROFILE_DIR}"
echo -e " Worktree:    ${GREEN}${EXEC_DIR}${NC}"
echo -e " Active Rules:${GREEN} ${RULE_COUNT} rules loaded and enforced:${NC}"
if [[ -d "${RULES_DIR}" ]]; then
    for rule_file in "${RULES_DIR}"/*.md; do
        if [[ -f "${rule_file}" ]]; then
            echo -e "   ✔ $(basename "${rule_file}" .md)"
        fi
    done
fi
if [[ -f "${WORKSPACE_ROOT}/AGENTS.md" ]]; then
    echo -e "   ✔ AGENTS.md (Root Guidelines & Quality Standards)"
fi
echo -e "${BOLD}${BLUE}======================================================================${NC}"
echo -e "Note: If this is the first run for Account #${ACCOUNT_NUM}, you will be"
echo -e "prompted to authenticate with that account's Google credentials."
echo -e "${BOLD}${BLUE}======================================================================${NC}"

export HOME="${PROFILE_DIR}"
export PATH="${PATH}:/home/codespace/.cargo/bin:/home/codespace/.local/bin"

cd "${EXEC_DIR}"
exec agy --agent "${AGENT_NAME}" "${REMAINING_ARGS[@]}"
