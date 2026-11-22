# AI KeyVault — Cooldown Logger

Script de Python para capturar lecturas de cuota de las cuentas de agy y Codex
cada N minutos. Usado para el experimento de cooldown (H1/H2/H3) de la Fase 0.

## Requisitos

```
Python 3.10+
pip install requests
pip install keyring  # opcional, para leer el token de Credential Manager automáticamente
```

## Configuración

1. Copia el archivo de ejemplo:
   ```
   copy config.json.example config.json
   ```

2. Edita `config.json` y completa los tokens reales.
   - Para **agy**: el `access_token` está en Windows Credential Manager bajo `gemini:antigravity`.
   - Para **Codex**: el `access_token` está en `~/.codex/auth.json`.

3. `config.json` está en `.gitignore` — nunca lo subas al repo.

## Uso

```bash
# Consultar cada 5 minutos (por defecto)
python cooldown_logger.py

# Consultar cada 2 minutos
python cooldown_logger.py --interval 2

# Especificar archivo de salida
python cooldown_logger.py --output mi-experimento.csv

# Con config en otra ruta
python cooldown_logger.py --config ../mi-config.json
```

## Salida

Genera un archivo CSV `cooldown-data.csv` con columnas:

| Campo | Descripción |
|-------|-------------|
| `timestamp` | Hora de la lectura (UTC ISO8601) |
| `provider` | `agy` o `codex` |
| `account_alias` | Nombre de la cuenta (definido en config.json) |
| `pool` | Pool o ventana: `gemini-5h`, `primary`, etc. |
| `tokens_used` | Tokens usados en esta ventana |
| `tokens_limit` | Límite de la ventana |
| `percent_used` | Porcentaje usado |
| `reset_time` | Cuándo se reinicia la ventana (ISO8601) |
| `source` | Siempre `client` para este script |

## Interpretación del experimento

Una vez que tengas datos, abre el CSV en Excel o con Python/pandas y busca:

1. **¿`reset_time` está definido ANTES del primer uso?**
   - Sí y no cambia → **H3** (horario fijo)
   - No está o está en NULL → continúa al siguiente punto

2. **¿`reset_time` cambia justo después del primer uso (mensaje enviado)?**
   - Sí → **H1** (el reloj empieza con el primer uso)

3. **¿`reset_time` solo aparece/cambia cuando `percent_used` llega al 100%?**
   - Sí → **H2** (el reloj empieza al llegar al límite)

Registra tus conclusiones en `docs/research/04-experimento-cooldown.md`.

## Seguridad

- El script **nunca** escribe tokens, contraseñas ni correos en el log ni en el CSV.
- El CSV solo contiene números (tokens, porcentajes) y timestamps.
- `config.json` (con tokens reales) está en `.gitignore`.
