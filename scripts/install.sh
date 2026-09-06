#!/bin/bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

echo "==> Compilando HandyX en modo Release..."
cd "$DIR"
cargo build --release

echo "==> Instalando binario en ~/.local/bin/handyx..."
mkdir -p ~/.local/bin
pkill -9 handyx 2>/dev/null || true
rm -f ~/.local/bin/handyx
cp target/release/handyx ~/.local/bin/handyx
chmod +x ~/.local/bin/handyx

# Symlink hadyx -> handyx for backwards compatibility
ln -sf ~/.local/bin/handyx ~/.local/bin/hadyx

echo "==> Configurando autostart y servicio en segundo plano..."
~/.local/bin/handyx autostart --enable

echo "==> ¡Instalación de HandyX completada con éxito!"
echo "HandyX ahora se inicia automáticamente con el sistema y aparece en la barra superior."
