#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
CHANGELOG="$ROOT/CHANGELOG.md"
CHANGELOG_SOURCE="${CHANGELOG_SOURCE:-worktree}"
SUBJECT_RE='^(docs|design|feat|fix|refactor|test|chore|build|ci|perf|style|revert)\([a-z0-9]([a-z0-9-]*[a-z0-9])?\): [A-Za-z0-9]([ -~]*[A-Za-z0-9)])?$'

fail() {
  printf 'change-management: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat >&2 <<'USAGE'
Usage:
  scripts/validate-change-management.sh commit-msg <message-file> [--no-staged-check]
  scripts/validate-change-management.sh changelog [expected-subject]
  scripts/validate-change-management.sh head
  scripts/validate-change-management.sh range <base-ref> <head-ref>
USAGE
  exit 2
}

validate_subject() {
  local subject="$1"
  local summary

  [[ -n "$subject" ]] || fail "commit subject is empty"
  [[ "$subject" =~ $SUBJECT_RE ]] || fail "commit subject must match: type(scope): summary"

  summary="${subject#*: }"
  ((${#summary} <= 72)) || fail "commit summary must be 72 characters or fewer"
  [[ "$summary" != *. ]] || fail "commit summary must not end with a period"

  case "$(printf '%s' "$summary" | tr '[:upper:]' '[:lower:]')" in
    *"update things"*|*"misc changes"*|*"fix stuff"*)
      fail "commit summary is too vague"
      ;;
  esac
}

commit_subject_from_file() {
  awk '!/^#/ && !seen { print; seen=1; exit }' "$1"
}

changelog_content() {
  if [[ "$CHANGELOG_SOURCE" == "staged" ]]; then
    git show ':CHANGELOG.md'
  else
    cat "$CHANGELOG"
  fi
}

message_body_has_content() {
  awk '
    /^#/ { next }
    !seen { seen=1; next }
    NF { found=1 }
    END { exit found ? 0 : 1 }
  ' "$1"
}

validate_body_if_present() {
  local file="$1"
  local section

  if ! message_body_has_content "$file"; then
    return 0
  fi

  for section in Context: Changes: Impact: Verification:; do
    grep -qx "$section" "$file" || fail "commit body with content must include section: $section"
  done
}

top_changelog_entry() {
  awk '
    /^## [0-9]{4}-[0-9]{2}-[0-9]{2} - / {
      if (started) { exit }
      started=1
    }
    started { print }
  ' < <(changelog_content)
}

top_changelog_subject() {
  top_changelog_entry | awk '
    /^- \*\*提交信息\*\*：`[^`]+`$/ {
      line=$0
      sub(/^- \*\*提交信息\*\*：`/, "", line)
      sub(/`$/, "", line)
      print line
      exit
    }
  '
}

section_has_bullet() {
  local section="$1"

  top_changelog_entry | awk -v section="$section" '
    $0 == "### " section { inside=1; next }
    inside && /^### / { exit }
    inside && /^- / { found=1 }
    END { exit found ? 0 : 1 }
  '
}

validate_changelog_structure() {
  if [[ "$CHANGELOG_SOURCE" == "staged" ]]; then
    git cat-file -e ':CHANGELOG.md' 2>/dev/null || fail "staged CHANGELOG.md is missing"
  else
    [[ -f "$CHANGELOG" ]] || fail "CHANGELOG.md is missing"
  fi

  changelog_content | grep -Eq '^## [0-9]{4}-[0-9]{2}-[0-9]{2} - .+' \
    || fail "CHANGELOG.md must contain at least one dated entry"

  if changelog_content | grep -nE '^## ' | grep -vE '^[0-9]+:## [0-9]{4}-[0-9]{2}-[0-9]{2} - .+' >/dev/null; then
    fail "all CHANGELOG.md entries must use: ## YYYY-MM-DD - title"
  fi

  top_changelog_entry | grep -Eq '^- \*\*类型\*\*：.+' \
    || fail "top changelog entry must include bullet field: 类型"
  top_changelog_entry | grep -Eq '^- \*\*范围\*\*：.+`.+`' \
    || fail "top changelog entry must include bullet field: 范围"
  top_changelog_entry | grep -Eq '^- \*\*提交信息\*\*：`[^`]+`$' \
    || fail "top changelog entry must include bullet field: 提交信息"

  top_changelog_entry | grep -qx '### 变更内容' \
    || fail "top changelog entry must include section: 变更内容"
  top_changelog_entry | grep -qx '### 设计影响' \
    || fail "top changelog entry must include section: 设计影响"
  top_changelog_entry | grep -qx '### 验证' \
    || fail "top changelog entry must include section: 验证"

  section_has_bullet "变更内容" || fail "top changelog 变更内容 section must contain a bullet"
  section_has_bullet "设计影响" || fail "top changelog 设计影响 section must contain a bullet"
  section_has_bullet "验证" || fail "top changelog 验证 section must contain a bullet"
}

validate_changelog_subject() {
  local expected="$1"
  local actual

  actual="$(top_changelog_subject)"
  [[ -n "$actual" ]] || fail "top changelog commit subject is missing"
  [[ "$actual" == "$expected" ]] \
    || fail "top changelog commit subject must match commit subject: $expected"
}

require_staged_changelog() {
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || return 0

  git diff --cached --name-only --diff-filter=ACMR | grep -qx 'CHANGELOG.md' \
    || fail "CHANGELOG.md must be staged and updated for this commit"
}

validate_commit_msg() {
  local file="$1"
  local check_staged="${2:-yes}"
  local subject

  [[ -f "$file" ]] || fail "commit message file is missing: $file"
  subject="$(commit_subject_from_file "$file")"

  validate_subject "$subject"
  validate_body_if_present "$file"

  if [[ "$check_staged" == "yes" ]]; then
    require_staged_changelog
    CHANGELOG_SOURCE=staged validate_changelog_structure
    CHANGELOG_SOURCE=staged validate_changelog_subject "$subject"
  else
    validate_changelog_structure
    validate_changelog_subject "$subject"
  fi
}

validate_head() {
  local tmp

  tmp="$(mktemp)"
  git log -1 --pretty=%B >"$tmp"
  validate_commit_msg "$tmp" no
  rm -f "$tmp"
}

validate_range() {
  local base="$1"
  local head="$2"
  local sha
  local subject

  validate_changelog_structure

  while read -r sha; do
    [[ -n "$sha" ]] || continue
    subject="$(git log -1 --pretty=%s "$sha")"
    validate_subject "$subject"
    changelog_content | grep -Fqx -- "- **提交信息**：\`$subject\`" \
      || fail "CHANGELOG.md must contain commit subject: $subject"
  done < <(git rev-list --no-merges --reverse "$base..$head")
}

case "${1:-}" in
  commit-msg)
    [[ $# -ge 2 ]] || usage
    if [[ "${3:-}" == "--no-staged-check" ]]; then
      validate_commit_msg "$2" no
    else
      validate_commit_msg "$2" yes
    fi
    ;;
  changelog)
    validate_changelog_structure
    if [[ -n "${2:-}" ]]; then
      validate_changelog_subject "$2"
    fi
    ;;
  head)
    validate_head
    ;;
  range)
    [[ $# -eq 3 ]] || usage
    validate_range "$2" "$3"
    ;;
  *)
    usage
    ;;
esac
