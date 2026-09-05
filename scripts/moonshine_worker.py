import sys
import os
import json
import base64
import numpy as np
import io

transcribers = {}

def get_transcriber(lang="es"):
    global transcribers
    lang = lang.strip().lower() if lang else "es"
    if lang not in ["es", "en"]:
        lang = "es"
    if lang in transcribers:
        return transcribers[lang]
    try:
        from moonshine_voice import Transcriber, ModelArch, get_model_for_language
        path, arch = get_model_for_language(lang, ModelArch.BASE)
        t = Transcriber(path, arch)
        transcribers[lang] = t
        return t
    except Exception as e:
        sys.stderr.write(f"Error loading Moonshine for {lang}: {e}\n")
        return None

# Preload default language (Spanish)
t_init = get_transcriber("es")
if t_init:
    sys.stdout.write("READY\n")
    sys.stdout.flush()
else:
    sys.stderr.write("Failed to preload Spanish Moonshine\n")
    sys.exit(1)

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    if line == "PING":
        sys.stdout.write("PONG\n")
        sys.stdout.flush()
        continue
    try:
        req = json.loads(line)
        audio_b64 = req.get("audio_b64", "")
        lang = req.get("language", "es")
        audio_bytes = base64.b64decode(audio_b64)
        
        # Read WAV bytes
        import wave
        with io.BytesIO(audio_bytes) as bio:
            with wave.open(bio, 'rb') as wf:
                framerate = wf.getframerate()
                nframes = wf.getnframes()
                raw_data = wf.readframes(nframes)
                samples = np.frombuffer(raw_data, dtype=np.int16).astype(np.float32) / 32768.0

        t = get_transcriber(lang)
        if not t:
            raise RuntimeError(f"Could not load transcriber for {lang}")

        transcript = t.transcribe_without_streaming(samples.tolist(), sample_rate=16000)
        text = ""
        if transcript and hasattr(transcript, 'lines'):
            text = " ".join([l.text for l in transcript.lines]).strip()
        elif transcript and hasattr(transcript, 'text'):
            text = transcript.text.strip()
        else:
            text = str(transcript).strip()

        resp = json.dumps({"status": "ok", "text": text})
        sys.stdout.write(resp + "\n")
        sys.stdout.flush()
    except Exception as e:
        resp = json.dumps({"status": "error", "error": str(e)})
        sys.stdout.write(resp + "\n")
        sys.stdout.flush()
