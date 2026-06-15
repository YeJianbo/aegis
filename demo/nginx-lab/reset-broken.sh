#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
cp "$ROOT/nginx/conf.d/default.conf.fixed" "$ROOT/nginx/conf.d/default.conf"
python - "$ROOT/nginx/conf.d/default.conf" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text()
path.write_text(text.replace("root /usr/share/nginx/html;", "root /usr/share/nginx/html"))
PY
cd "$ROOT"
docker compose down
