#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR"
GH_PAGES_DIR="$REPO_ROOT/../gh-pages"
DIST_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$DIST_DIR"
}
trap cleanup EXIT


if [[ ! -e "$GH_PAGES_DIR/.git" ]]; then
  echo "❌ No se encontró el worktree/repo gh-pages en: $GH_PAGES_DIR"
  exit 1
fi

# 1) Build (sin crear ./dist dentro del repo)
trunk build --release --public-url /SummerCQuiz/ --dist "$DIST_DIR"

# 2) Sync al worktree gh-pages
rsync -av --delete --exclude=".git" --exclude=".nojekyll" "$DIST_DIR/" "$GH_PAGES_DIR/"
# 3) Commit & push
cd "$GH_PAGES_DIR"
touch .nojekyll
git add .

if git diff --cached --quiet; then
  echo "ℹ️ No hay cambios para desplegar"
  exit 0
fi

git commit -m "Deploy $(date +'%Y-%m-%d %H:%M')"
git push origin gh-pages

echo "✅ Deploy completado: https://sugar144.github.io/SummerCQuiz/"
