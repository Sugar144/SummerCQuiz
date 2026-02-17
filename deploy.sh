#!/bin/bash
set -e

# 1) Build
trunk build --release

# 2) Sync a gh-pages (worktree)
rsync -av --delete --exclude=".git" dist/ ../gh-pages/

# 3) Commit & push
cd ../gh-pages
touch .nojekyll
git add .
git commit -m "Deploy $(date +'%Y-%m-%d %H:%M')"
git push origin gh-pages

# 4) Volver al repo principal
cd ../summer_quiz
echo "✅ Deploy completado: https://sugar144.github.io/SummerCQuiz/"
