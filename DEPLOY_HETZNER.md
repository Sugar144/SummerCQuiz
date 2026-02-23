# Deploy del Judge Server en Hetzner (Cloudflare + HTTPS)

Objetivo:
- Frontend (GitHub Pages) en `https://app.check4fun.app/`
- Backend judge (este repo) en `https://api.check4fun.app/` con endpoint `POST /api/judge/sync`

## 1) DNS en Cloudflare

En Cloudflare → DNS:
- Crea un `A` record:
  - **Name**: `api`
  - **IPv4**: `89.167.89.10`
  - **Proxy**: recomendado **DNS only** (nube gris) al menos mientras emites el certificado TLS

Opcional (IPv6): crea también un `AAAA api -> <TU IPv6 EXACTA>`.
Nota: Cloudflare no acepta un `/64`; necesita una IPv6 concreta (una dirección).

Cuando ya funcione, puedes cambiar a **Proxied** (nube naranja) si quieres. En pruebas, deja DNS only.

## 2) Preparar el VPS (Ubuntu/Debian)

En el VPS, como root:

Instala Docker + Compose plugin:

```bash
apt update
apt install -y ca-certificates curl git

curl -fsSL https://get.docker.com | sh

# compose plugin (en distros modernas suele venir con docker)
docker --version
docker compose version
```

Abre firewall (si usas ufw):

```bash
ufw allow 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
ufw enable
```

## 3) Subir y levantar el backend

Clona el repo:

```bash
git clone <TU_REPO_URL> summer-quiz
cd summer-quiz
```

Edita el `Caddyfile` si tu dominio no es `judge.check4fun.app`.

Levanta todo:

```bash
docker compose up -d --build
```

Ver logs:

```bash
docker compose logs -f --tail=200
```

Comprobar health:

```bash
curl -sS https://api.check4fun.app/health
```

## 4) Unir frontend + backend

En tu `index.html` del frontend, asegúrate de tener:

```html
<meta name="summer-quiz-judge-endpoint" content="https://api.check4fun.app/api/judge/sync" />
```

Luego haz tu deploy normal del frontend (tu `deploy.sh`).

## 5) Diagnóstico rápido

- Si en el navegador ves errores CORS: confirma que el backend responde a `OPTIONS` (tu server ya lo hace) y que Caddy está apuntando bien.
- Si Cloudflare está en modo Proxied y no emite TLS: prueba primero DNS only.

