#!/bin/bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

echo "==> Compilando HadyX en modo Release..."
cd "$DIR"
cargo build --release

echo "==> Instalando binario en ~/.local/bin/hadyx..."
mkdir -p ~/.local/bin
cp target/release/hadyx ~/.local/bin/hadyx
chmod +x ~/.local/bin/hadyx

echo "==> Registrando servicio systemd de usuario..."
mkdir -p ~/.config/systemd/user/
cp "$DIR/scripts/hadyx.service" ~/.config/systemd/user/hadyx.service 2>/dev/null || true

echo "==> ¡Instalación completada con éxito!"
echo "Puedes iniciar el daemon ejecutando: hadyx daemon"
echo "O mediante systemd: systemctl --user enable --now hadyx"
