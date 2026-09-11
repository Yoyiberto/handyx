# ⚡ HandyX (Minimalist Rust Speech-to-Text)

Aplicación minimalista y ultra-rápida de dictado y transcripción de voz para **Ubuntu / Linux (Wayland y X11)** escrita en **Rust**. Combina la filosofía de **Handy** y **Whispering**, con soporte nativo para **Moonshine Base**, **Groq Whisper Large v3 Turbo**, **Pulido de Texto con IA (OpenRouter / Luna)**, **Icono en la Barra Superior (System Tray)**, **Pausa Automática de Música (MPRIS)** y **Pegado Inteligente en Terminal**.

---

## 🌟 Características Principales

1. **3 Modelos de Transcripción**:
   - 🏠 **`moonshine-en`**: Moonshine Base en Inglés (100% Local y Offline).
   - 🏠 **`moonshine-es`**: Moonshine Base en Español (100% Local y Offline).
   - ⚡ **`groq`**: Groq Whisper Large v3 Turbo (Cloud con Auto-Detección de Idioma).

2. **🎵 Pausa y Reanudación Automática de Música (MPRIS D-Bus)**:
   - Al hablar, pausa automáticamente reproductores activos (Spotify, Chrome/YouTube, Firefox, VLC, etc.).
   - Al terminar de grabar, reanuda únicamente las aplicaciones que estaban sonando.
   - Activación/desactivación al vuelo con `handyx toggle-media-pause` o desde el icono superior.

3. **🖥️ Pegado Inteligente en Terminal (GNOME Terminal / VTE)**:
   - Detecta automáticamente si la ventana activa es una terminal y envía `Ctrl + Shift + V` en lugar de `Ctrl + V`.
   - Sincroniza simultáneamente los buffers `CLIPBOARD` y `PRIMARY` para compatibilidad total.

4. **📌 Icono en la Barra Superior (System Tray / AppIndicator)**:
   - Estado visual en tiempo real (micrófono en espera / punto rojo grabando).
   - Menú contextual con clic para alternar modelos, pausar música, activar pulido con IA, abrir historial y salir.

5. **✨ Pulido de Texto con IA (AI Polish vía OpenRouter)**:
   - Modelo predeterminado: **`openai/gpt-5.6-luna`** (configurable a cualquier modelo de OpenRouter).
   - Corrige puntuación, gramática, mayúsculas y elimina muletillas manteniendo el idioma y sentido del audio.
   - Conmutable al instante con `handyx toggle-polish`.

6. **📁 Historial y Guardado de Grabaciones**:
   - Guarda automáticamente cada audio (`.wav`) y transcripción (`.json`) con marca de tiempo en `~/.local/share/handyx/recordings/`.
   - Registro cronológico en `~/.local/share/handyx/history.jsonl`.
   - Consulta rápida con `handyx history` y apertura de carpeta con `handyx open-history`.

7. **🛡️ Ejecución 100% en Segundo Plano & Autostart**:
   - Inicia con el sistema mediante `~/.config/autostart/handyx.desktop` y `systemd --user`.
   - Comandos para iniciar o reiniciar sin mantener terminales abiertas: `handyx restart` o `handyx daemon -d`.

---

## ⌨️ Comandos CLI Disponibles

| Comando | Descripción |
|---|---|
| `handyx daemon -d` | Inicia el servicio en segundo plano (detached, sin terminal) |
| `handyx restart` | Reinicia el daemon en segundo plano |
| `handyx stop-daemon` | Detiene el servicio en segundo plano |
| `handyx toggle` | Alterna grabación (usado por el atajo global de teclado) |
| `handyx switch-engine` | Rota entre los 3 modelos: `groq-turbo` ➔ `moonshine-en` ➔ `moonshine-es` |
| `handyx set-engine <MODEL>` | Selecciona modelo (`groq`, `moonshine-en`, `moonshine-es`) |
| `handyx toggle-media-pause` | Activa o desactiva la pausa automática de música |
| `handyx toggle-polish` | Activa o desactiva el pulido de texto con IA |
| `handyx set-openrouter-key <KEY>` | Guarda tu clave de API de OpenRouter de forma persistente |
| `handyx set-polish-model <MODEL>` | Configura el modelo de pulido (predeterminado: `openai/gpt-5.6-luna`) |
| `handyx history [N]` | Muestra las últimas `N` transcripciones |
| `handyx open-history` | Abre la carpeta de grabaciones en el explorador de archivos |
| `handyx set-mode <MODE>` | Configura el modo de atajo (`hybrid`, `toggle`, `hold`) |
| `handyx set-shortcut "<KEY>"` | Configura el atajo global de GNOME (ej. `<Control><Shift>space`) |
| `handyx set-key "<KEY>"` | Guarda tu clave de API de Groq de forma persistente |
| `handyx status` | Muestra la configuración activa y estado del daemon |

---

## ⚙️ Archivo de Configuración (`~/.config/handyx/config.toml`)

```toml
engine = "groq-turbo"           # "groq-turbo", "moonshine-en", o "moonshine-es"
shortcut = "<Control><Shift>space"
shortcut_mode = "hybrid"        # "hybrid", "toggle", o "hold"
auto_paste = true
paste_delay_ms = 60
pause_media_on_record = true

# Groq
groq_api_key = "gsk_..."
groq_model = "whisper-large-v3-turbo"

# AI Polish (OpenRouter)
enable_ai_polish = true
openrouter_api_key = "sk-or-..."
openrouter_model = "openai/gpt-5.6-luna"

# Historial y Grabaciones
save_history = true
save_audio = true
# history_dir = "/ruta/personalizada" (Opcional, por defecto ~/.local/share/handyx)
```
