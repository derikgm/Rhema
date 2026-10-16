# TODO.md — Rhema (app de dictado local en tiempo real)

## Contexto del proyecto

App de escritorio y Android para **dictado por voz con transcripción local en tiempo real**.
El usuario habla y el texto aparece a medida que habla. Además debe poder **escribir manualmente**
(puntuación, correcciones) sin que se rompa el flujo del dictado.

Nombre en clave del proyecto: **Rhema** (del griego ῥῆμα, "palabra hablada").
El repositorio ya existe con el cascarón básico de Tauri. Este archivo define
el trabajo a completar.

---

## Stack técnico obligatorio

- **Rust** (backend Tauri)
- **Tauri 2.x**
- **React** (frontend)
- **TypeScript** en el frontend
- **Tailwind CSS v4** (usar `@import "tailwindcss";` en el CSS, no `tailwind.config.js` clásico)
- **Vite** como bundler
- **sherpa-onnx** (crate de Rust) para reconocimiento de voz **por streaming**
- **VAD** (Voice Activity Detection) incluido en sherpa-onnx
- Empaquetado multiplataforma: Linux, Windows, macOS, Android

**NO usar** `tauri-plugin-stt` — es una solución push-to-talk (bufferiza todo y
transcribe al soltar). No sirve para transcripción en vivo.

---

## Requisitos funcionales

### 1. Transcripción en tiempo real
- El usuario pulsa un botón de micrófono y empieza a hablar.
- El texto debe ir apareciendo **mientras habla**, no al terminar.
- Flujo interno: micrófono → VAD (detecta segmentos de habla) → sherpa-onnx
  OfflineRecognizer/StreamingRecognizer → evento IPC al frontend → append al editor.
- La inferencia debe correr en un hilo aparte (Rust async o thread), nunca bloquear la UI.

### 2. Escritura manual intercalada
- Mientras se dicta, el usuario puede escribir con el teclado.
- **Caso de uso clave**: si el usuario presiona `,` o `.` mientras habla, ese
  carácter debe insertarse en el texto en el punto correcto.
- Implementación: un único `<textarea>` o editor (p. ej. `contenteditable` o
  `textarea` controlado) que recibe **ambas fuentes de entrada**:
  - Voz → append al final del buffer.
  - Teclado → inserción normal en la posición del cursor.
- Evitar duplicaciones: si un segmento de voz está "pendiente de cerrar",
  al teclear un signo de puntuación, primero se debe confirmar el texto de voz
  y luego insertar el signo. Mantener una **máquina de estados** simple:
  `idle | listening | transcribing-segment`.

### 3. Comandos de puntuación por voz (deseable, fase 2)
- Referencia: dictado nativo de macOS/Windows.
- Decir "coma" → `,`, "punto" → `.`, "nueva línea" → `\n`.
- Implementar como post-procesado del texto que llega del ASR.

### 4. Exportación
- Formatos mínimos:
  - **Markdown** con timestamps por segmento (formato principal).
  - **TXT plano** (fallback).
- Ejemplo de salida Markdown:
  ```
  # Dictado — 2026-01-15 14:32

  - [14:32:01] Hola, esto es una prueba.
  - [14:32:05] Y esto es la segunda línea.
  ```
- Diálogo nativo de "Guardar como…" usando el plugin de diálogos de Tauri.

### 5. Persistencia local
- Todo el audio y los modelos se procesan **en el dispositivo**.
- Nada de APIs externas, nada de telemetría.
- Guardar borradores en `appDataDir/drafts/` por si la app se cierra.

---

## Requisitos de UI/UX

- **Paleta de colores: tonalidades azules.** Definir variables CSS en `:root`:
  - Fondo principal: azul muy oscuro (`#0a1628` o similar).
  - Superficie / tarjetas: azul oscuro medio (`#14213d`).
  - Acento primario: azul brillante (`#3b82f6` / `#60a5fa`).
  - Texto principal: azul muy claro casi blanco (`#e6f0ff`).
  - Texto secundario: azul grisáceo (`#94a3b8`).
  - Éxito / activo: cian suave (`#22d3ee`).
- Tailwind v4: definir el tema con `@theme` en el CSS principal.
- Modo oscuro por defecto (es una app de dictado, se usa mucho de noche).
- Indicador visual claro de **"grabando"** (punto pulsante o waveform simple).
- Botón de micrófono grande, accesible, con atajo de teclado (p. ej. `Ctrl+Space`).
- El editor debe ocupar el centro de la pantalla, tipografía legible (mínimo 16px).

---

## Estructura sugerida del repo

```
rhema/
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs              # descarga .so de sherpa-onnx para Android
│   ├── src/
│   │   ├── main.rs
│   │   ├── audio.rs          # captura de micrófono (cpal)
│   │   ├── asr.rs            # wrapper de sherpa-onnx + VAD
│   │   ├── commands.rs       # comandos #[tauri::command]
│   │   └── models.rs         # descarga/gestión de modelos ONNX
│   ├── models/               # modelos ONNX (gitignore, se descargan al build)
│   └── tauri.conf.json
├── src/
│   ├── App.tsx
│   ├── main.tsx
│   ├── styles.css            # Tailwind v4 + @theme azul
│   ├── components/
│   │   ├── Editor.tsx        # textarea + lógica de doble canal
│   │   ├── MicButton.tsx
│   │   └── StatusBar.tsx
│   ├── hooks/
│   │   ├── useTranscription.ts
│   │   └── useKeyboardPunctuation.ts
│   └── state/
│       └── dictationMachine.ts   # máquina de estados idle/listening/...
├── package.json
├── vite.config.ts
└── TODO.md
```

---

## Lista de tareas (orden de ejecución)

### Fase 0 — Setup
- [ ] Verificar que el cascarón de Tauri arranca en Linux (`cargo tauri dev`).
- [x] Instalar y configurar **Tailwind CSS v4** con Vite.
- [x] Definir paleta azul en `styles.css` usando `@theme` de Tailwind v4.
- [x] Configurar `tsconfig.json` estricto y alias `@/` → `src/`.
- [ ] Añadir `cargo fmt`, `cargo clippy` y ESLint al repo.

### Fase 1 — Captura de audio en Rust
- [x] Añadir dependencia `cpal` para captura de micrófono multiplataforma.
- [x] Implementar `audio.rs`: captura con `cpal`, mezcla a mono y remuestreo a
      16 kHz f32 (el VAD de sherpa-onnx exige 16 kHz mono).
- [x] Exponer comandos `start_listening` / `stop_listening` desde Tauri.
- [x] Emitir evento `audio-level` al frontend para el indicador visual
      (waveform o punto pulsante).

### Fase 2 — Integración de sherpa-onnx
- [x] Añadir `sherpa-rs` o `sherpa-onnx` al `Cargo.toml`.
- [ ] Elegir modelo: se usa **Dolphin** (offline, multilingüe). Pendiente evaluar
      un modelo **streaming** en español para transcripción continua (el actual
      entrega el texto al cerrar cada segmento de voz).
- [x] Implementar `asr.rs`: VAD (Silero) + reconocedor en un hilo aparte; emite
      `final-transcript` al frontend.
- [x] Descarga/gestión de modelos desde la UI (`models.rs` + panel de Ajustes).
- [ ] Verificar latencia: el texto parcial debe aparecer en < 500ms.

### Fase 3 — Frontend de dictado
- [x] Componente `Editor.tsx`: `textarea` controlado por estado de React.
- [x] Escuchar eventos de transcripción y hacer append al editor (integrado en
      `App.tsx`; el hook `useTranscription.ts` queda como código heredado).
- [ ] Hook `useKeyboardPunctuation.ts`: interceptar `,` `.` `?` `!` mientras
      está en modo dictado y asegurar que se insertan correctamente.
- [ ] Máquina de estados en `dictationMachine.ts`:
      `idle → listening → transcribing-segment → listening → idle`.
- [x] Componente `MicButton.tsx` con estados visuales (idle/grabando/procesando).
- [x] Componente `StatusBar.tsx` mostrando palabras y caracteres.

### Fase 4 — Exportación
- [x] Comandos `save_text` y `save_draft` en Rust para guardar/exportar.
- [x] Exportación a Markdown desde `ExportButtons.tsx`.
- [x] Exportación a TXT desde `ExportButtons.tsx`.
- [x] Diálogo nativo "Guardar como…" con `tauri-plugin-dialog`.
- [x] Autoguardado de borrador en `appDataDir/drafts/` cada 30s.

### Fase 5 — Comandos de voz
- [x] Post-procesador de texto que reemplace "coma", "punto", "nueva línea",
      "signo de interrogación" por sus símbolos (`postprocessPunctuation.ts`).
- [x] Hacerlo configurable (on/off) desde ajustes.

### Fase 6 — Android
- [ ] Configurar `build.rs` para descargar `.so` precompilado de sherpa-onnx.
- [ ] Copiar `.so` a `src-tauri/gen/android/app/src/main/jniLibs/`.
- [ ] Ajustar permisos de micrófono en `AndroidManifest.xml`.
- [ ] Probar `cargo tauri android build --apk --debug` en dispositivo real.
- [ ] Verificar latencia y consumo de batería en Android.

### Fase 7 — Pulido
- [x] Atajo global `Ctrl+Space` para iniciar/detener dictado.
- [ ] Modo siempre-encima (mini ventana flotante) — opcional.
- [ ] Exportación a `.srt` si se graban timestamps por palabra — opcional.
- [ ] Documentar en README cómo compilar en cada plataforma.
- [ ] Verificar que ningún dato sale del dispositivo (auditoría de red).

---

## Reglas para la IA que trabaje en este repo

1. **No introducir dependencias cloud.** Todo debe funcionar offline.
2. **No usar `tauri-plugin-stt`.** Ya se descartó por ser push-to-talk.
3. **Mantener la paleta azul** definida en `styles.css`. No introducir
   otros colores de acento sin avisar.
4. **Tailwind v4**: usar `@theme` y `@import "tailwindcss";`. No crear
   `tailwind.config.js`.
5. **Rust**: correr `cargo clippy` antes de dar una tarea por terminada.
6. **Frontend**: correr `tsc --noEmit` y ESLint antes de dar por terminada.
7. **Commits pequeños**, uno por tarea marcada arriba.
8. Si una tarea requiere un modelo ONNX que pese > 100 MB, dejarlo en
   `.gitignore` y descargarlo en build time.
9. Documentar cualquier decisión técnica relevante en `docs/decisions/`.
10. Si algo del plan no es viable, **parar y preguntar** antes de improvisar.

---

## Criterios de aceptación finales

- [ ] En Linux, Windows, macOS y Android la app arranca.
- [ ] Al pulsar el micrófono, el texto aparece en pantalla mientras hablo.
- [ ] Puedo escribir `,` o `.` con el teclado durante el dictado y se
      insertan correctamente.
- [ ] Puedo exportar el resultado a Markdown y a TXT.
- [ ] La app no realiza ninguna petición de red saliente durante el uso
      normal (verificar con `strace`/`netstat`/Wireshark).
- [ ] La UI respeta la paleta azul definida.