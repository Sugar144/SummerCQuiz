#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist"
PAGES_DIR="$SCRIPT_DIR/../gh-pages"

# 1) Build
trunk build --release --public-url /SummerCQuiz/

# 2) Sync a gh-pages (worktree)
rsync -av --delete --exclude=".git" "$DIST_DIR/" "$PAGES_DIR/"

# 3) Commit & push (solo si hubo cambios)
cd "$PAGES_DIR"
touch .nojekyll
git add .

if git diff --cached --quiet; then
  echo "ℹ️ No hay cambios para publicar en gh-pages."
else
  git commit -m "Deploy $(date +'%Y-%m-%d %H:%M')"
  git push origin gh-pages
fi

# 4) Volver al repo principal
cd "$SCRIPT_DIR"
echo "✅ Deploy completado: https://sugar144.github.io/SummerCQuiz/"
