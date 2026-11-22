#!/usr/bin/env python3
"""
AI KeyVault — Cooldown Logger
Registra la cuota de cada cuenta de agente IA cada N minutos
para el experimento de cooldown (H1/H2/H3).

Uso:
    python cooldown_logger.py --config config.json --interval 5

Requisitos: Python 3.10+, requests, keyring (opcional para Credential Manager)

IMPORTANTE:
- Este script NUNCA escribe secretos en el log ni en pantalla.
- El config.json.example muestra el formato con datos falsos.
- El config.json real (con tokens reales) está en .gitignore.
"""

import argparse
import csv
import json
import logging
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

import requests

# ─────────────────────────────────────────────
# Configuración de logging (sin secretos)
# ─────────────────────────────────────────────
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%Y-%m-%dT%H:%M:%S",
)
log = logging.getLogger("cooldown-logger")


# ─────────────────────────────────────────────
# Leer token de Credential Manager de Windows
# ─────────────────────────────────────────────
def read_agy_token_from_credential_manager(account_name: str) -> Optional[dict]:
    """
    Lee el blob de credenciales de agy desde Windows Credential Manager.
    Requiere el módulo keyring con backend de Windows.
    Devuelve el dict parseado o None si no se puede leer.
    """
    try:
        import keyring
        blob = keyring.get_password("gemini:antigravity", account_name)
        if blob:
            return json.loads(blob)
    except ImportError:
        log.warning("keyring no disponible — usar token desde config directamente")
    except Exception as e:
        log.error("Error leyendo Credential Manager: %s", type(e).__name__)
    return None


# ─────────────────────────────────────────────
# Consulta de cuota para agy
# ─────────────────────────────────────────────
def query_agy_quota(account: dict) -> Optional[list[dict]]:
    """
    Consulta la cuota de una cuenta de agy via cloudcode-pa.googleapis.com.
    Devuelve lista de lecturas por pool o None si falla.
    
    NOTA: El access_token NO se loguea nunca.
    """
    # El token puede venir del Credential Manager o del config cifrado
    token = account.get("access_token")
    if not token:
        log.warning("[agy:%s] Sin access_token configurado", account.get("alias", "?"))
        return None

    try:
        response = requests.post(
            "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
            },
            json={},
            timeout=10,
        )
        response.raise_for_status()
        data = response.json()

        readings = []
        quota_summary = data.get("quotaSummary", {})
        pools = quota_summary.get("pools", [])

        for pool in pools:
            pool_id = pool.get("poolId", "unknown")
            tokens_used = pool.get("tokensUsed", 0)
            tokens_limit = pool.get("tokensLimit", 1)
            reset_time = pool.get("resetTime")

            percent_used = round((tokens_used / tokens_limit) * 100, 2) if tokens_limit else 0

            readings.append({
                "timestamp": datetime.now(timezone.utc).isoformat(),
                "provider": "agy",
                "account_alias": account.get("alias", "unknown"),
                "pool": pool_id,
                "tokens_used": tokens_used,
                "tokens_limit": tokens_limit,
                "percent_used": percent_used,
                "reset_time": reset_time,
                "source": "client",
            })

        return readings

    except requests.exceptions.Timeout:
        log.warning("[agy:%s] Timeout al consultar cuota", account.get("alias", "?"))
    except requests.exceptions.HTTPError as e:
        # Solo loguear el status code, nunca el token
        log.error("[agy:%s] HTTP %s al consultar cuota", account.get("alias", "?"), e.response.status_code)
    except Exception as e:
        log.error("[agy:%s] Error inesperado: %s", account.get("alias", "?"), type(e).__name__)

    return None


# ─────────────────────────────────────────────
# Consulta de cuota para Codex
# ─────────────────────────────────────────────
def query_codex_quota(account: dict) -> Optional[list[dict]]:
    """
    Consulta la cuota de una cuenta de Codex via /backend-api/wham/profiles/me.
    Devuelve lista de lecturas por ventana o None si falla.
    """
    token = account.get("access_token")
    if not token:
        log.warning("[codex:%s] Sin access_token configurado", account.get("alias", "?"))
        return None

    try:
        response = requests.get(
            "https://api.openai.com/backend-api/wham/profiles/me",
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
            },
            timeout=10,
        )
        response.raise_for_status()
        data = response.json()

        readings = []
        now = datetime.now(timezone.utc).isoformat()
        alias = account.get("alias", "unknown")

        for window_key in ("primary_window", "secondary_window"):
            window = data.get(window_key, {})
            if not window:
                continue

            tokens_used = window.get("tokens_used", 0)
            tokens_limit = window.get("tokens_limit", 1)
            reset_at = window.get("reset_at")

            percent_used = round((tokens_used / tokens_limit) * 100, 2) if tokens_limit else 0

            readings.append({
                "timestamp": now,
                "provider": "codex",
                "account_alias": alias,
                "pool": window_key.replace("_window", ""),
                "tokens_used": tokens_used,
                "tokens_limit": tokens_limit,
                "percent_used": percent_used,
                "reset_time": reset_at,
                "source": "client",
            })

        return readings

    except requests.exceptions.Timeout:
        log.warning("[codex:%s] Timeout", account.get("alias", "?"))
    except requests.exceptions.HTTPError as e:
        log.error("[codex:%s] HTTP %s", account.get("alias", "?"), e.response.status_code)
    except Exception as e:
        log.error("[codex:%s] Error: %s", account.get("alias", "?"), type(e).__name__)

    return None


# ─────────────────────────────────────────────
# Escritura del CSV
# ─────────────────────────────────────────────
CSV_FIELDS = [
    "timestamp", "provider", "account_alias", "pool",
    "tokens_used", "tokens_limit", "percent_used", "reset_time", "source"
]

def write_readings(output_path: Path, readings: list[dict]):
    """Agrega lecturas al CSV de salida."""
    file_exists = output_path.exists()
    with open(output_path, "a", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=CSV_FIELDS)
        if not file_exists:
            writer.writeheader()
        for reading in readings:
            writer.writerow({k: reading.get(k, "") for k in CSV_FIELDS})


# ─────────────────────────────────────────────
# Loop principal
# ─────────────────────────────────────────────
def run_loop(config: dict, interval_min: int, output_path: Path):
    """Loop principal: consulta cuota y escribe cada `interval_min` minutos."""
    accounts = config.get("accounts", [])
    if not accounts:
        log.error("No hay cuentas configuradas en config.json")
        sys.exit(1)

    log.info("Iniciando cooldown-logger | Intervalo: %d min | Cuentas: %d", interval_min, len(accounts))
    log.info("Salida: %s", output_path)
    log.info("Presiona Ctrl+C para detener")

    while True:
        all_readings = []

        for account in accounts:
            provider = account.get("provider", "").lower()
            alias = account.get("alias", "sin-alias")

            log.info("Consultando [%s:%s]...", provider, alias)

            readings = None
            if provider == "agy":
                readings = query_agy_quota(account)
            elif provider == "codex":
                readings = query_codex_quota(account)
            else:
                log.warning("[%s] Proveedor no soportado en esta versión", provider)

            if readings:
                all_readings.extend(readings)
                for r in readings:
                    # Detectar cambios en reset_time (para el experimento de cooldown)
                    log.info(
                        "  ✓ [%s/%s] %s%% | reset_time=%s",
                        provider, r["pool"], r["percent_used"], r["reset_time"]
                    )

        if all_readings:
            write_readings(output_path, all_readings)
            log.info("Guardadas %d lecturas", len(all_readings))

        log.info("Esperando %d minutos...", interval_min)
        try:
            time.sleep(interval_min * 60)
        except KeyboardInterrupt:
            log.info("Detenido por el usuario")
            break


# ─────────────────────────────────────────────
# Entrypoint
# ─────────────────────────────────────────────
def main():
    parser = argparse.ArgumentParser(description="AI KeyVault — Cooldown Logger")
    parser.add_argument("--config", default="config.json", help="Ruta al archivo de configuración")
    parser.add_argument("--interval", type=int, default=5, help="Intervalo en minutos (default: 5)")
    parser.add_argument("--output", default="cooldown-data.csv", help="Archivo CSV de salida")
    args = parser.parse_args()

    config_path = Path(args.config)
    if not config_path.exists():
        log.error("No se encuentra config.json. Copia config.json.example y completa los tokens.")
        sys.exit(1)

    with open(config_path, encoding="utf-8") as f:
        config = json.load(f)

    output_path = Path(args.output)
    run_loop(config, args.interval, output_path)


if __name__ == "__main__":
    main()
