# ⚡ HadyX (Minimalist Rust Speech-to-Text)

Aplicación minimalista y ultra-rápida de dictado y transcripción de voz para **Ubuntu / Linux (Wayland y X11)** escrita en **Rust**. Combina la filosofía minimalista de **Handy** y **Whispering**, con soporte nativo para **Moonshine Base**, **Groq Whisper Large v3 Turbo**, **Pulido de Texto con IA (OpenRouter / Luna)** y **Guardado de Historial y Grabaciones**.

---

## 🌟 Novedades y Características Principales

1. **3 Modelos de Transcripción**:
   - 🏠 **`moonshine-en`**: Moonshine Base en Inglés (100% Local y Offline).
   - 🏠 **`moonshine-es`**: Moonshine Base en Español (100% Local y Offline).
   - ⚡ **`groq`**: Groq Whisper Large v3 Turbo (Cloud con Auto-Detección de Idioma).

2. **✨ Pulido de Texto con IA (AI Polish vía OpenRouter)**:
   - Modelo predeterminado: **`openai/gpt-5.6-luna`** (configurable a cualquier modelo de OpenRouter).
   - Corrige puntuación, gramática, mayúsculas y elimina muletillas/titubeos manteniendo fielmente el idioma y sentido del audio.
   - HUD visual: Muestra `✨ Puliendo texto... [Luna]`.
   - Se puede activar/desactivar al vuelo con `hadyx toggle-polish`.

3. **📁 Historial y Guardado de Grabaciones**:
   - Guarda automáticamente cada audio (`.wav`) y transcripción (`.json`) con marca de tiempo en `~/.local/share/hadyx/recordings/`.
   - Registro cronológico en `~/.local/share/hadyx/history.jsonl` guardando el texto original (*raw*) y el texto pulido (*polished*).
   - Comando CLI para consultar historial (`hadyx history`) y abrir la carpeta en el explorador de archivos (`hadyx open-history`).

4. **Inyección y Pegado Fiable en Wayland/X11**:
   - Inyección de teclas mediante `enigo` nativo en Rust + `wtype` / `xclip` / `wl-copy`.
   - Ventana HUD pasiva que **nunca roba el foco** de tu editor, navegador o terminal.

5. **Modos de Grabación Flexibles**:
   - `hybrid` (Predeterminado: tap = toggle, hold = push-to-talk)
   - `toggle` (Presionar para iniciar, presionar para detener)
   - `hold` (Mantener presionado para hablar)

---

## ⌨️ Comandos CLI Disponibles

| Comando | Descripción |
|---|---|
| `hadyx daemon` | Inicia el servicio en segundo plano con el HUD flotante |
| `hadyx toggle` | Alterna grabación (usado por el atajo de teclado) |
| `hadyx switch-engine` | Rota entre los 3 modelos: `groq-turbo` ➔ `moonshine-en` ➔ `moonshine-es` |
| `hadyx set-engine <MODEL>` | Selecciona modelo (`groq`, `moonshine-en`, `moonshine-es`) |
| `hadyx toggle-polish` | Activa o desactiva el pulido de texto con IA |
| `hadyx set-openrouter-key <KEY>` | Guarda tu clave de API de OpenRouter |
| `hadyx set-polish-model <MODEL>` | Configura el modelo de pulido (predeterminado: `openai/gpt-5.6-luna`) |
| `hadyx history [N]` | Muestra las últimas `N` transcripciones (con audio y texto pulido) |
| `hadyx open-history` | Abre la carpeta de grabaciones en tu explorador de archivos |
| `hadyx set-mode <MODE>` | Configura el modo de atajo (`hybrid`, `toggle`, `hold`) |
| `hadyx set-shortcut "<KEY>"` | Configura el atajo global de GNOME (ej. `<Control><Shift>space`) |
| `hadyx set-key "<KEY>"` | Guarda tu clave de API de Groq |
| `hadyx status` | Muestra la configuración activa y estado del daemon |

---

## ⚙️ Archivo de Configuración (`~/.config/hadyx/config.toml`)

```toml
engine = "groq-turbo"           # "groq-turbo", "moonshine-en", o "moonshine-es"
shortcut = "<Control><Shift>space"
shortcut_mode = "hybrid"        # "hybrid", "toggle", o "hold"
auto_paste = true
paste_delay_ms = 60

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
# history_dir = "/ruta/personalizada" (Opcional, por defecto ~/.local/share/hadyx)
```
