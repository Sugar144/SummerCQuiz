#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR"
GH_PAGES_DIR="$REPO_ROOT/../gh-pages"
DIST_DIR="$(mktemp -d)"

cleanup() { rm -rf "$DIST_DIR"; }
trap cleanup EXIT

if [[ ! -e "$GH_PAGES_DIR/.git" ]]; then
  echo "❌ No se encontró el worktree/repo gh-pages en: $GH_PAGES_DIR"
  exit 1
fi

# --- 0) Asegurar rama correcta y sincronizada en el worktree ---
# (evita el desastre de commitear en master dentro del worktree)
git -C "$GH_PAGES_DIR" fetch origin

# Crear/actualizar rama local gh-pages desde origin/gh-pages y cambiar a ella
git -C "$GH_PAGES_DIR" switch -C gh-pages origin/gh-pages

# Dejar el worktree EXACTAMENTE como origin/gh-pages (limpia divergencias locales)
git -C "$GH_PAGES_DIR" reset --hard origin/gh-pages

# --- 1) Build (sin crear ./dist dentro del repo) ---
# Si usas dominio propio (app.check4fun.app), public-url "/" está bien.
# Si publicas en GitHub Pages bajo /SummerCQuiz/, cámbialo a "/SummerCQuiz/".
trunk build --release --public-url / --dist "$DIST_DIR"

# --- 2) Sync al worktree gh-pages ---
rsync -av --delete --exclude=".git" "$DIST_DIR/" "$GH_PAGES_DIR/"
touch "$GH_PAGES_DIR/.nojekyll"

# ✅ Mantener dominio custom en cada deploy
echo "app.check4fun.app" > "$GH_PAGES_DIR/CNAME"

# --- 3) Commit & push ---
git -C "$GH_PAGES_DIR" add .

if git -C "$GH_PAGES_DIR" diff --cached --quiet; then
  echo "ℹ️ No hay cambios para desplegar"
  exit 0
fi

git -C "$GH_PAGES_DIR" commit -m "Deploy $(date +'%Y-%m-%d %H:%M')"

# Para gh-pages, normalmente es lo correcto asegurar que remoto = tu build actual
git -C "$GH_PAGES_DIR" push --force-with-lease origin gh-pages

echo "✅ Deploy completado: https://app.check4fun.app/"