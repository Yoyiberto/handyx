# ⚡ HadyX (Minimalist Rust Speech-to-Text)

Aplicación minimalista y ultra-rápida de dictado y transcripción de voz para **Ubuntu / Linux (Wayland y X11)** escrita en **Rust**. Inspirada en la arquitectura de **Handy** de CJ Pais, eliminando sobrecarga innecesaria para ofrecer máxima velocidad, pegado fiable y soporte para **3 modelos seleccionables**.

---

## 🧠 Los 3 Modelos Disponibles

1. 🏠 **`moonshine-en`** (**Moonshine Base English - Local/Offline**):
   - Modelo neuronal de UsefulSensors/Moonshine AI especializado en inglés.
   - 100% offline, ejecución directa en tu máquina sin conexión ni APIs.
2. 🏠 **`moonshine-es`** (**Moonshine Base Español - Local/Offline**):
   - Modelo neuronal de UsefulSensors/Moonshine AI especializado en español.
   - 100% offline, optimizado para pronunciación y vocabulario en español.
3. ⚡ **`groq-turbo`** (**Groq Whisper Large v3 Turbo - Cloud/Auto-Detect**):
   - Modelo de OpenAI servido en Groq LPU.
   - **Auto-detección de idioma**: Puedes hablar en español, inglés o mezclar ambos idiomas sin forzar nada; Whisper detecta y transcribe automáticamente con latencia < 200 ms.

---

## ⌨️ Comandos Rápidos

### Alternar o Seleccionar Modelo
```bash
# Rotar cíclicamente entre los 3 modelos:
# (Groq Turbo Auto -> Moonshine English -> Moonshine Spanish -> Groq Turbo...)
hadyx switch-engine

# O seleccionar directamente el que quieras:
hadyx set-engine moonshine-en   # Moonshine Base Inglés (Local)
hadyx set-engine moonshine-es   # Moonshine Base Español (Local)
hadyx set-engine groq           # Groq Whisper Turbo (Cloud Auto-Detect)
```

### Modos de Grabación (Toggle vs Push-to-Talk / Hold)
```bash
# Modo Híbrido (Predeterminado: toque rápido = toggle; mantener presionado = push-to-talk)
hadyx set-mode hybrid

# Modo Toggle estricto (un toque para iniciar, otro para detener)
hadyx set-mode toggle

# Modo Hold estricto (mantener presionado mientras hablas)
hadyx set-mode hold
```

### Atajo Global de Teclado
```bash
# Atajo predeterminado: Ctrl + Shift + Espacio (evita conflictos)
hadyx set-shortcut "<Control><Shift>space"

# O si prefieres Ctrl + Espacio:
hadyx set-shortcut "<Control>space"
```

### Iniciar el Daemon
```bash
hadyx daemon
```
