#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

fail() {
  printf 'verify: %s\n' "$*" >&2
  exit 1
}

require_file() {
  local file="$1"
  [[ -f "$ROOT/$file" ]] || fail "required file is missing: $file"
}

require_contains() {
  local file="$1"
  local text="$2"
  grep -Fq "$text" "$ROOT/$file" || fail "$file must contain: $text"
}

check_required_files() {
  local file
  local files=(
    AGENTS.md
    README.md
    CHANGELOG.md
    docs/index.md
    docs/project-governance.md
    docs/project-map.yml
    docs/glossary.md
    docs/change-management.md
    docs/linux-runtime-info-categories.md
    docs/linux-runtime-info-subcategories.md
    docs/linux-runtime-info-collection-items.md
    docs/linux-runtime-info-collection-decision.md
    docs/collector-security-governance.md
    docs/collector-testing-governance.md
    docs/collector-architecture.md
    docs/phase-a-review.md
    docs/decisions/README.md
    docs/decisions/0001-runtime-info-boundary.md
    docs/decisions/0002-human-ai-governance.md
    docs/decisions/0003-implementation-tech-stack.md
    docs/decisions/0004-output-format.md
    docs/templates/changelog-entry.md
    docs/templates/decision-record.md
    scripts/setup-dev.sh
    scripts/validate-change-management.sh
    scripts/verify.sh
    .githooks/commit-msg
    .github/workflows/change-management.yml
  )

  for file in "${files[@]}"; do
    require_file "$file"
  done
}

check_navigation() {
  local doc

  require_contains README.md "docs/index.md"
  require_contains README.md "docs/project-governance.md"
  require_contains README.md "docs/change-management.md"
  require_contains README.md "CHANGELOG.md"

  require_contains AGENTS.md "docs/index.md"
  require_contains AGENTS.md "docs/project-governance.md"
  require_contains AGENTS.md "docs/change-management.md"
  require_contains AGENTS.md "docs/glossary.md"
  require_contains AGENTS.md "docs/project-map.yml"
  require_contains AGENTS.md "scripts/verify.sh"

  while IFS= read -r doc; do
    doc="${doc#$ROOT/}"
    require_contains docs/index.md "$doc"
  done < <(find "$ROOT/docs" -type f -name '*.md' | sort)
}

check_project_map() {
  local path
  local paths=(
    AGENTS.md
    README.md
    CHANGELOG.md
    docs/index.md
    docs/project-governance.md
    docs/project-map.yml
    docs/glossary.md
    docs/change-management.md
    docs/linux-runtime-info-categories.md
    docs/linux-runtime-info-subcategories.md
    docs/linux-runtime-info-collection-items.md
    docs/linux-runtime-info-collection-decision.md
    docs/collector-security-governance.md
    docs/collector-testing-governance.md
    docs/collector-architecture.md
    docs/phase-a-review.md
    docs/decisions/README.md
    docs/decisions/0001-runtime-info-boundary.md
    docs/decisions/0002-human-ai-governance.md
    docs/decisions/0003-implementation-tech-stack.md
    docs/decisions/0004-output-format.md
    docs/templates/changelog-entry.md
    docs/templates/decision-record.md
    scripts/setup-dev.sh
    scripts/validate-change-management.sh
    scripts/verify.sh
    .githooks/commit-msg
    .github/workflows/change-management.yml
  )

  for path in "${paths[@]}"; do
    require_contains docs/project-map.yml "$path"
  done
}

check_markdown_links() {
  local file
  local link
  local target
  local full_target

  while IFS= read -r file; do
    while IFS= read -r link; do
      target="${link#*](}"
      target="${target%)}"
      case "$target" in
        http://*|https://*|mailto:*|"#"*|"")
          continue
          ;;
      esac
      target="${target%%#*}"
      if [[ "$target" == /* ]]; then
        full_target="$ROOT$target"
      else
        full_target="$(dirname "$file")/$target"
      fi
      [[ -e "$full_target" ]] || fail "$file links to missing path: $target"
    done < <(awk '
      /^```/ { fenced = !fenced; next }
      !fenced { print }
    ' "$file" | grep -oE '\[[^]]+\]\([^)]+\)' || true)
  done < <(
    printf '%s\n' "$ROOT/AGENTS.md" "$ROOT/README.md" "$ROOT/CHANGELOG.md"
    find "$ROOT/docs" -type f -name '*.md' | sort
  )
}

check_shell_executables() {
  [[ -x "$ROOT/scripts/validate-change-management.sh" ]] \
    || fail "scripts/validate-change-management.sh must be executable"
  [[ -x "$ROOT/scripts/setup-dev.sh" ]] \
    || fail "scripts/setup-dev.sh must be executable"
  [[ -x "$ROOT/scripts/verify.sh" ]] \
    || fail "scripts/verify.sh must be executable"
  [[ -x "$ROOT/.githooks/commit-msg" ]] \
    || fail ".githooks/commit-msg must be executable"
}

main() {
  check_required_files
  check_navigation
  check_project_map
  check_markdown_links
  check_shell_executables

  "$ROOT/scripts/validate-change-management.sh" changelog
  git -C "$ROOT" diff --check

  if ! git -C "$ROOT" diff --cached --quiet --; then
    git -C "$ROOT" diff --cached --check
  fi
}

main "$@"
