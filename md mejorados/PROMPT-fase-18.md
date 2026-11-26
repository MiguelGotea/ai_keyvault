# Prompt para Claude Code — Fase 18 (Reconexión)

Pega esto tal cual al abrir una sesión nueva en la raíz del repo:

---

Antes de tocar nada, lee `CLAUDE.md` completo (especialmente §1.1 y §1.2) y de `PLAN2.md` las secciones «Cómo leer este documento», «Decisiones» (cerradas y abiertas), «Fase 18» y la «Matriz de cumplimiento».

Contexto: la app de escritorio (v0.2.4) tiene toda la UI, pero el flujo de datos poller → VPS → app no funciona, y sospecho que durante la etapa 2 se modificó o eliminó funcionalidad útil del backend. Esta sesión es **solo diagnóstico**.

Las bases de datos (VPS y locales) fueron limpiadas a propósito: parto de cero y no hay nada en producción. Que el VPS esté vacío es lo esperado, no un hallazgo.

Haz únicamente el sub-paso **18.0**:

1. No modifiques código. Solo lectura, `git` y comandos de consulta.
2. Ejecuta los 6 puntos de 18.0 y escribe el informe `docs/research/06-diagnostico-fase18.md` con la tabla «esperado / real / causa». Cada afirmación debe llevar `[VERIFICADO]` (con el comando y la salida, sin secretos) o `[SUPUESTO]`.
3. En el punto 1, lista **todo** lo eliminado o modificado en `server/`, `crates/`, `providers/` y `.github/` desde el último commit anterior a `apps/desktop`, y di si era necesario.
4. Las decisiones D-01 a D-12 ya están cerradas en `PLAN2.md`: no las replantees. Si el diagnóstico las contradice (por ejemplo, el poller ya lee otras cuentas, o el servidor no soporta algo), repórtalo como pregunta concreta para mí. **No implementes nada en esta sesión.**
5. Actualiza el estado de la Fase 18 en `PLAN2.md` y detente.

Si encuentras una contradicción entre documentos o entre documentos y código, no la resuelvas en silencio: anótala en el informe y pregúntame.

Recuerda: el sandbox no tiene red; los comandos con `cargo` o `curl` al VPS me los pides y yo los corro. Nunca hagas commit; yo corro `.\gitpush.ps1`.

---

## Orden sugerido de las sesiones siguientes

| Sesión | Sub-paso | Condición para empezar |
|---|---|---|
| 1 | 18.0 Diagnóstico | — |
| 2 | 18.1 Revisión del informe 18.0 contra las decisiones | Informe 18.0 revisado por ti |
| 3 | 18.2 Reparar identidad y cruce (D-01, D-02, D-06) | Informe aprobado |
| 4 | 18.3 Contrato API + tests | 18.2 terminado |
| 5 | 18.4 Errores visibles + 18.5 Respaldo | 18.3 en verde |
| 6 | 18.6 Verificación en uso real | Todo lo anterior |
| 7 | Fase 19 (sesión y primer arranque), luego 20 | Fase 18 verificada en uso real |
