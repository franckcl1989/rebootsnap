#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
CHANGELOG="$ROOT/CHANGELOG.md"
CHANGELOG_SOURCE="${CHANGELOG_SOURCE:-worktree}"
SUBJECT_RE='^(docs|design|feat|fix|refactor|test|chore|build|ci|perf|style|revert)\([a-z0-9]([a-z0-9-]*[a-z0-9])?\): [A-Za-z0-9]([ -~]*[A-Za-z0-9])?$'

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

changelog_subjects() {
  changelog_content | awk '
    /^- \*\*提交信息\*\*：`[^`]+`$/ {
      line=$0
      sub(/^- \*\*提交信息\*\*：`/, "", line)
      sub(/`$/, "", line)
      print line
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

validate_all_changelog_entries() {
  changelog_content | awk '
    function reset_entry() {
      has_type=0
      has_scope=0
      has_subject=0
      has_content=0
      has_impact=0
      has_verification=0
      content_bullet=0
      impact_bullet=0
      verification_bullet=0
      field_order=0
      section=""
    }

    function entry_error(message) {
      printf "change-management: CHANGELOG.md entry starting at line %d: %s\n", entry_line, message > "/dev/stderr"
      bad=1
    }

    function check_entry() {
      if (!has_type) {
        entry_error("missing bullet field: 类型")
      }
      if (!has_scope) {
        entry_error("missing bullet field: 范围")
      }
      if (!has_subject) {
        entry_error("missing bullet field: 提交信息")
      }
      if (!has_content) {
        entry_error("missing section: 变更内容")
      }
      if (!has_impact) {
        entry_error("missing section: 设计影响")
      }
      if (!has_verification) {
        entry_error("missing section: 验证")
      }
      if (!content_bullet) {
        entry_error("变更内容 section must contain a bullet")
      }
      if (!impact_bullet) {
        entry_error("设计影响 section must contain a bullet")
      }
      if (!verification_bullet) {
        entry_error("验证 section must contain a bullet")
      }
    }

    /^## / {
      if (in_entry) {
        check_entry()
      }
      in_entry=1
      entry_line=NR
      reset_entry()
      if ($0 !~ /^## [0-9]{4}-[0-9]{2}-[0-9]{2} - .+/) {
        entry_error("heading must use: ## YYYY-MM-DD - title")
      }
      next
    }

    !in_entry {
      next
    }

    /^- \*\*类型\*\*：/ {
      if (field_order != 0) {
        entry_error("类型 must be the first metadata bullet")
      }
      has_type=1
      field_order=1
      if ($0 !~ /^- \*\*类型\*\*：(文档|设计|实现|修复|重构|测试|工程化|构建|性能|样式|回滚)( \/ (文档|设计|实现|修复|重构|测试|工程化|构建|性能|样式|回滚))*$/) {
        entry_error("类型 uses unsupported value")
      }
      next
    }

    /^- \*\*范围\*\*：/ {
      if (field_order != 1) {
        entry_error("范围 must immediately follow 类型")
      }
      has_scope=1
      field_order=2
      if ($0 !~ /^- \*\*范围\*\*：.+`.+`/) {
        entry_error("范围 must include at least one backticked stable path or domain")
      }
      next
    }

    /^- \*\*提交信息\*\*：/ {
      if (field_order != 2) {
        entry_error("提交信息 must immediately follow 范围")
      }
      has_subject=1
      field_order=3
      if ($0 !~ /^- \*\*提交信息\*\*：`[^`]+`$/) {
        entry_error("提交信息 must be a single backticked commit subject")
      }
      next
    }

    $0 == "### 变更内容" {
      has_content=1
      section="content"
      next
    }

    $0 == "### 设计影响" {
      has_impact=1
      section="impact"
      next
    }

    $0 == "### 验证" {
      has_verification=1
      section="verification"
      next
    }

    /^### / {
      section=""
      next
    }

    /^- / {
      if (section == "content") {
        content_bullet=1
      } else if (section == "impact") {
        impact_bullet=1
      } else if (section == "verification") {
        verification_bullet=1
      }
    }

    END {
      if (in_entry) {
        check_entry()
      }
      exit bad ? 1 : 0
    }
  '
}

validate_changelog_subjects() {
  local subject

  while IFS= read -r subject; do
    validate_subject "$subject"
  done < <(changelog_subjects)
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

  validate_all_changelog_entries
  validate_changelog_subjects

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
