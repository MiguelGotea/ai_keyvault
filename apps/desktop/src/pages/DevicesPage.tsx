/**
 * DevicesPage — Pantalla «Mis dispositivos» (Fase 25).
 * Lista los dispositivos registrados en el VPS y permite revocar acceso.
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import type { DeviceItem } from "../types/api";

export default function DevicesPage() {
  const devices = useCommand<DeviceItem[]>("get_devices", {});
  const [revoking, setRevoking] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  async function handleRevoke(device: DeviceItem) {
    if (!confirm(`¿Revocar el acceso de '${device.name}'?`)) return;
    setRevoking(device.id);
    setMsg(null);
    try {
      await invoke("revoke_device", { deviceId: device.id });
      setMsg(`✅ Dispositivo '${device.name}' revocado.`);
      devices.refetch();
    } catch (e) {
      setMsg(`❌ Error al revocar: ${String(e)}`);
    } finally {
      setRevoking(null);
    }
  }

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div>
          <h2 style={styles.title}>💻 Mis dispositivos</h2>
          <p style={styles.subtitle}>
            PCs registradas que han sincronizado bóveda o lecturas con el VPS.
          </p>
        </div>
        <button style={styles.refreshBtn} onClick={devices.refetch} disabled={devices.loading}>
          {devices.loading ? "⏳" : "↻"} Actualizar
        </button>
      </div>

      {msg && (
        <div style={{
          ...styles.banner,
          color: msg.startsWith("✅") ? "var(--color-success)" : "var(--color-danger)",
        }}>
          <span>{msg}</span>
          <button style={styles.closeBtn} onClick={() => setMsg(null)}>✕</button>
        </div>
      )}

      {devices.error && <div style={styles.error}>⚠️ {devices.error}</div>}
      {devices.loading && <div style={styles.hint}>Cargando dispositivos…</div>}

      {!devices.loading && !devices.error && (devices.data ?? []).length === 0 && (
        <div style={styles.empty}>No hay dispositivos registrados aún.</div>
      )}

      {(devices.data ?? []).length > 0 && (
        <div style={styles.list}>
          {(devices.data ?? []).map(d => (
            <div key={d.id} style={styles.row}>
              <div style={styles.info}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <strong style={styles.name}>{d.name}</strong>
                  <span style={{
                    ...styles.badge,
                    color: d.is_active ? "var(--color-success)" : "var(--color-danger)",
                    borderColor: d.is_active ? "var(--color-success)" : "var(--color-danger)",
                  }}>
                    {d.is_active ? "Activo" : "Revocado"}
                  </span>
                </div>
                <span style={styles.meta}>
                  ID: {d.id} · Última actividad: {new Date(d.last_seen_at).toLocaleString("es-MX")}
                </span>
              </div>
              {d.is_active && (
                <button
                  style={styles.revokeBtn}
                  disabled={revoking === d.id}
                  onClick={() => handleRevoke(d)}
                >
                  {revoking === d.id ? "Revocando…" : "Revocar acceso"}
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 16 },
  header: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12 },
  title: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  subtitle: { fontSize: 12, color: "var(--color-text-disabled)", margin: "4px 0 0 0" },
  refreshBtn: { padding: "7px 14px", border: "1px solid var(--color-btn-border)", borderRadius: "var(--radius-md)", background: "var(--color-btn-bg)", color: "var(--color-btn-text)", fontSize: 13, cursor: "pointer" },
  banner: { padding: "10px 14px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 13, display: "flex", justifyContent: "space-between", alignItems: "center" },
  closeBtn: { background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-disabled)" },
  error: { padding: "10px 14px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 12, color: "#991B1B" },
  hint: { fontSize: 13, color: "var(--color-text-secondary)" },
  empty: { padding: 40, textAlign: "center", fontSize: 13, color: "var(--color-text-disabled)" },
  list: { display: "flex", flexDirection: "column", gap: 8 },
  row: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "12px 16px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", gap: 12, flexWrap: "wrap" as const },
  info: { display: "flex", flexDirection: "column", gap: 4 },
  name: { fontSize: 14, color: "var(--color-text)" },
  badge: { fontSize: 10, fontWeight: 700, padding: "1px 8px", borderRadius: 999, border: "1px solid" },
  meta: { fontSize: 11, color: "var(--color-text-disabled)", fontFamily: "var(--font-mono)" },
  revokeBtn: { padding: "6px 12px", border: "1px solid var(--color-danger-border)", borderRadius: "var(--radius-md)", background: "var(--color-danger-bg)", color: "var(--color-danger-text)", fontSize: 12, cursor: "pointer", fontWeight: 600 },
};
