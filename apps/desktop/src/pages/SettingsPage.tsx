/** SettingsPage — Fase 15: Configuración de la app.
 *  Incluye: servidor VPS, alertas, ping automático, seguridad, respaldo.
 *  Fase 14: persistencia de autobloqueo en localStorage + Rust.
 */
import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import es from "../i18n/es.json";

const DEFAULT_SETTINGS = {
  serverUrl: "http://198.211.97.243:3010",
  deviceId: "mi-pc",
  pollInterval: 60,
  ntfyTopic: "",
  toastEnabled: true,
  pingHour: 8,
  pingMaxDaily: 2,
  autolockMinutes: 0, // 0 = desactivado (opt-in, desactivado por defecto)
  dpapiEnabled: false,
  theme: "dark" as "dark" | "light",
};

type Settings = typeof DEFAULT_SETTINGS;

export default function SettingsPage() {
  const [settings, setSettings] = useState<Settings>(() => {
    // Cargar ajustes persistidos en localStorage al iniciar
    try {
      const stored = localStorage.getItem("akv_settings");
      if (stored) return { ...DEFAULT_SETTINGS, ...JSON.parse(stored) };
    } catch { /* si está corrupto, usar defaults */ }
    return DEFAULT_SETTINGS;
  });
  const [saved, setSaved] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [showDpapiWarning, setShowDpapiWarning] = useState(false);

  // Sincronizar autolockMinutes con la clave que usa UnlockPage y DashboardPage
  useEffect(() => {
    localStorage.setItem("autolock_minutes", String(settings.autolockMinutes));
  }, [settings.autolockMinutes]);

  function set<K extends keyof Settings>(key: K, value: Settings[K]) {
    setSettings(s => ({ ...s, [key]: value }));
    setSaved(false);
  }

  function handleDpapiToggle(enabled: boolean) {
    if (enabled) {
      setShowDpapiWarning(true);
    } else {
      set("dpapiEnabled", false);
    }
  }

  function confirmDpapi() {
    set("dpapiEnabled", true);
    setShowDpapiWarning(false);
  }

  async function saveSettings() {
    setSaveError(null);
    const cleanAutolock = Number.isFinite(settings.autolockMinutes)
      ? Math.max(0, Math.min(120, settings.autolockMinutes))
      : 0;
    const toSave = { ...settings, autolockMinutes: cleanAutolock };
    setSettings(toSave);

    // Persistir ajustes en localStorage
    localStorage.setItem("akv_settings", JSON.stringify(toSave));
    localStorage.setItem("autolock_minutes", String(cleanAutolock));

    // Aplicar autobloqueo en Rust si está activo
    try {
      if (cleanAutolock > 0) {
        await invoke("start_autolock_timer", { minutes: cleanAutolock });
      } else {
        await invoke("cancel_autolock_timer");
      }
      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    } catch (e) {
      setSaveError(`No se pudo aplicar el autobloqueo: ${String(e)}`);
    }
  }

  return (
    <div style={styles.page}>
      <h2 style={styles.title}>{es.ajustes.titulo}</h2>

      {/* Servidor */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>{es.ajustes.seccion_servidor}</h3>
        <Row label={es.ajustes.device_id}>
          <input
            style={styles.input}
            value={settings.deviceId}
            onChange={e => set("deviceId", e.target.value)}
          />
        </Row>
        <Row label={es.ajustes.intervalo_polling}>
          <input
            type="number"
            style={{ ...styles.input, width: 100 }}
            value={settings.pollInterval}
            min={30}
            max={3600}
            onChange={e => set("pollInterval", parseInt(e.target.value))}
          />
          <span style={styles.hint}>seg</span>
        </Row>
      </section>

      {/* Alertas */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>{es.ajustes.seccion_alertas}</h3>
        <Row label={es.ajustes.ntfy_topico}>
          <input
            style={styles.input}
            placeholder={es.ajustes.ntfy_placeholder}
            value={settings.ntfyTopic}
            onChange={e => set("ntfyTopic", e.target.value)}
          />
        </Row>
        <Row label={es.ajustes.toast_habilitado}>
          <Toggle
            value={settings.toastEnabled}
            onChange={v => set("toastEnabled", v)}
          />
        </Row>
      </section>

      {/* Ping automático */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>{es.ajustes.seccion_ping}</h3>
        <Row label={es.ajustes.ping_hora}>
          <input
            type="number"
            style={{ ...styles.input, width: 80 }}
            value={settings.pingHour}
            min={0}
            max={23}
            onChange={e => set("pingHour", parseInt(e.target.value))}
          />
          <span style={styles.hint}>h (hora local)</span>
        </Row>
        <Row label={es.ajustes.ping_max_diario}>
          <input
            type="number"
            style={{ ...styles.input, width: 80 }}
            value={settings.pingMaxDaily}
            min={1}
            max={10}
            onChange={e => set("pingMaxDaily", parseInt(e.target.value))}
          />
        </Row>
      </section>

      {/* Seguridad */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>{es.ajustes.seccion_seguridad}</h3>
        <Row label={es.ajustes.autobloqueo_minutos}>
          <input
            type="number"
            style={{ ...styles.input, width: 80 }}
            value={Number.isFinite(settings.autolockMinutes) ? settings.autolockMinutes : 0}
            min={0}
            max={120}
            onChange={e => {
              const parsed = parseInt(e.target.value, 10);
              set("autolockMinutes", Number.isNaN(parsed) ? 0 : parsed);
            }}
          />
          <span style={styles.hint}>min</span>
        </Row>
        <div style={styles.dpapiRow}>
          <div>
            <div style={styles.label}>{es.ajustes.dpapi_titulo}</div>
            <div style={styles.dpapiDesc}>{es.ajustes.dpapi_descripcion}</div>
          </div>
          <Toggle value={settings.dpapiEnabled} onChange={handleDpapiToggle} />
        </div>
        <AutostartRow />
      </section>

      {/* Modal confirmación DPAPI */}
      {showDpapiWarning && (
        <div style={styles.modalOverlay}>
          <div style={styles.modal}>
            <h3 style={{ color: "var(--color-danger)", margin: "0 0 12px" }}>⚠️ Advertencia de seguridad</h3>
            <p style={{ fontSize: 13, lineHeight: 1.6 }}>
              {es.ajustes.dpapi_descripcion}
            </p>
            <p style={{ fontSize: 13, fontWeight: 600 }}>
              ¿Quieres activar el desbloqueo rápido?
            </p>
            <div style={{ display: "flex", gap: 10, justifyContent: "flex-end", marginTop: 16 }}>
              <button style={styles.btnOutline} onClick={() => setShowDpapiWarning(false)}>
                {es.comun.cancelar}
              </button>
              <button
                style={{ ...styles.btnOutline, borderColor: "var(--color-danger)", color: "var(--color-danger)" }}
                onClick={confirmDpapi}
              >
                Activar de todas formas
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Respaldo */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>{es.ajustes.seccion_respaldo}</h3>
        <Row label={es.ajustes.ultimo_respaldo}>
          <LastBackupStatus />
        </Row>
        <Row label="Sincronizar bóveda con VPS">
          <VaultSyncButtons />
        </Row>
        <Row label="Sesión en esta PC">
          <GlobalLogoutButton />
        </Row>
      </section>

      {/* Guardar */}
      {saveError && (
        <div style={{ padding: "10px 14px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 12, color: "#991B1B" }}>
          ⚠️ {saveError}
        </div>
      )}
      <div style={styles.saveRow}>
        <button style={styles.saveBtn} onClick={saveSettings}>
          {saved ? `✅ ${es.ajustes.guardado_ok}` : es.ajustes.guardar}
        </button>
      </div>
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div style={rowStyles.row}>
      <label style={rowStyles.label}>{label}</label>
      <div style={rowStyles.control}>{children}</div>
    </div>
  );
}

/** Toggle «Iniciar con Windows (en bandeja)» (Fase 19.3, D-03). */
function AutostartRow() {
  const [enabled, setEnabled] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>("get_autostart_enabled")
      .then(setEnabled)
      .catch((e) => setError(`No se pudo leer inicio con Windows: ${String(e)}`));
  }, []);

  async function handleToggle(next: boolean) {
    setBusy(true);
    setError(null);
    try {
      await invoke("set_autostart_enabled", { enabled: next });
      setEnabled(next);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, paddingTop: 10, borderTop: "1px solid var(--color-border)" }}>
      <div style={styles.dpapiRow}>
        <div>
          <div style={styles.label}>Iniciar con Windows (en bandeja)</div>
          <div style={styles.dpapiDesc}>
            Abre AI KeyVault minimizado en la bandeja del sistema al iniciar sesión en Windows.
          </div>
        </div>
        <Toggle value={enabled} onChange={(v) => { if (!busy) handleToggle(v); }} />
      </div>
      {error && <span style={{ fontSize: 12, color: "var(--color-danger)" }}>⚠️ {error}</span>}
    </div>
  );
}

/** Botón de cierre de sesión global en esta PC (Fase 19.2, D-12). */
function GlobalLogoutButton() {
  const navigate = useNavigate();
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [logoutError, setLogoutError] = useState<string | null>(null);

  async function executeLogout(forceWithoutSync: boolean) {
    setBusy(true);
    setLogoutError(null);
    try {
      await invoke("global_logout", { forceWithoutSync });
      localStorage.removeItem("akv_settings");
      localStorage.removeItem("autolock_minutes");
      navigate("/unlock", { replace: true });
    } catch (e) {
      setLogoutError(
        `No se pudo subir la bóveda pendiente antes de salir (${String(e)}). ¿Deseas cerrar sesión local sin sincronizar?`
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8, alignItems: "flex-start" }}>
      {!confirming ? (
        <button
          onClick={() => setConfirming(true)}
          style={{
            padding: "6px 12px",
            fontSize: 12,
            border: "1px solid var(--color-danger)",
            borderRadius: "var(--radius-md)",
            background: "transparent",
            color: "var(--color-danger)",
            cursor: "pointer",
            fontWeight: 600,
          }}
        >
          Cerrar sesión global en esta PC
        </button>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
            Borra la sesión y bóveda locales de esta PC (no borra nada en el VPS ni tus cuentas de agy/Codex).
          </span>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <button
              disabled={busy}
              onClick={() => executeLogout(false)}
              style={{
                padding: "6px 12px",
                fontSize: 12,
                border: "none",
                borderRadius: "var(--radius-md)",
                background: "var(--color-danger)",
                color: "#fff",
                cursor: "pointer",
                fontWeight: 600,
              }}
            >
              {busy ? "Cerrando…" : "Confirmar cierre de sesión"}
            </button>
            <button
              disabled={busy}
              onClick={() => { setConfirming(false); setLogoutError(null); }}
              style={{
                padding: "6px 12px",
                fontSize: 12,
                border: "1px solid var(--color-border)",
                borderRadius: "var(--radius-md)",
                background: "var(--color-bg)",
                color: "var(--color-text-secondary)",
                cursor: "pointer",
              }}
            >
              Cancelar
            </button>
          </div>
          {logoutError && (
            <div style={{ fontSize: 12, color: "var(--color-danger)", display: "flex", flexDirection: "column", gap: 6 }}>
              <span>⚠️ {logoutError}</span>
              <button
                disabled={busy}
                onClick={() => executeLogout(true)}
                style={{
                  alignSelf: "flex-start",
                  padding: "5px 10px",
                  fontSize: 11,
                  border: "1px solid var(--color-danger)",
                  borderRadius: "var(--radius-sm)",
                  background: "transparent",
                  color: "var(--color-danger)",
                  cursor: "pointer",
                }}
              >
                Forzar cierre local sin sincronizar
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/** Muestra la fecha del último backup cifrado en ai_keyvault_data (D-04). */
function LastBackupStatus() {
  const [status, setStatus] = useState<string>("Consultando…");
  const [loading, setLoading] = useState(false);

  async function refresh() {
    setLoading(true);
    try {
      const info = await invoke<{ available: boolean; last_backup_at: string | null; message: string }>(
        "get_last_backup_info"
      );
      if (info.available && info.last_backup_at) {
        setStatus(`${new Date(info.last_backup_at).toLocaleString("es-MX")} (${info.message})`);
      } else {
        setStatus(info.message);
      }
    } catch (e) {
      setStatus(`Error: ${String(e)}`);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
      <span style={styles.hint}>{status}</span>
      <button
        onClick={refresh}
        disabled={loading}
        style={{ padding: "4px 8px", fontSize: 11, border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", background: "var(--color-bg-secondary)", color: "var(--color-text)", cursor: "pointer" }}
      >
        {loading ? "⏳" : "↻"}
      </button>
    </div>
  );
}

/** Botones de sincronización manual de la bóveda con el VPS (Fase 15). */
function VaultSyncButtons() {
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
  const [syncing, setSyncing] = useState(false);

  async function doSync(direction: "upload" | "download") {
    setSyncing(true);
    setSyncMsg(null);
    try {
      const cmd = direction === "upload" ? "upload_vault_sync" : "download_vault_sync";
      const result = await invoke<{ success: boolean; direction: string; error: string | null }>(cmd);
      if (result.success) {
        const label = result.direction === "none"
          ? "Sin cambios en el servidor"
          : `${result.direction === "upload" ? "⬆️ Subida" : "⬇️ Descarga"} completada`;
        setSyncMsg(`✅ ${label}`);
      } else {
        setSyncMsg(`❌ ${result.error ?? "Error desconocido"}`);
      }
    } catch (e) {
      setSyncMsg(`❌ ${String(e)}`);
    } finally {
      setSyncing(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8, alignItems: "flex-start" }}>
      <div style={{ display: "flex", gap: 8 }}>
        <button
          style={{ padding: "6px 12px", fontSize: 12, border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", cursor: "pointer", background: "var(--color-bg-secondary)", color: "var(--color-text)" }}
          onClick={() => doSync("upload")}
          disabled={syncing}
        >
          ⬆️ Subir bóveda al VPS
        </button>
        <button
          style={{ padding: "6px 12px", fontSize: 12, border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", cursor: "pointer", background: "var(--color-bg-secondary)", color: "var(--color-text)" }}
          onClick={() => doSync("download")}
          disabled={syncing}
        >
          ⬇️ Descargar bóveda del VPS
        </button>
      </div>
      {syncMsg && (
        <span style={{ fontSize: 12, color: syncMsg.startsWith("✅") ? "var(--color-success)" : "var(--color-danger)" }}>
          {syncMsg}
        </span>
      )}
    </div>
  );
}

function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      onClick={() => onChange(!value)}
      style={{
        width: 42,
        height: 24,
        borderRadius: 999,
        border: "none",
        cursor: "pointer",
        background: value ? "var(--color-primary)" : "var(--color-border)",
        position: "relative",
        flexShrink: 0,
        transition: "background var(--transition-fast)",
      }}
    >
      <span
        style={{
          position: "absolute",
          top: 3,
          left: value ? 20 : 3,
          width: 18,
          height: 18,
          borderRadius: "50%",
          background: "#fff",
          transition: "left var(--transition-fast)",
        }}
      />
    </button>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 20, maxWidth: 640 },
  title: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  section: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: "18px 20px", display: "flex", flexDirection: "column", gap: 14 },
  sectionTitle: { fontSize: 14, fontWeight: 600, color: "var(--color-text)", margin: 0 },
  input: { padding: "7px 10px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text)", fontSize: 13, minWidth: 220 },
  hint: { fontSize: 12, color: "var(--color-text-disabled)" },
  label: { fontSize: 13, fontWeight: 500, color: "var(--color-text)" },
  dpapiRow: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16 },
  dpapiDesc: { fontSize: 12, color: "var(--color-text-secondary)", marginTop: 4, maxWidth: 420, lineHeight: 1.5 },
  saveRow: { display: "flex", justifyContent: "flex-end" },
  saveBtn: { padding: "10px 20px", border: "none", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 14, cursor: "pointer", fontWeight: 600 },
  modalOverlay: { position: "fixed" as const, inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 999 },
  modal: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-xl)", padding: 28, maxWidth: 420, width: "90%" },
  btnOutline: { padding: "8px 14px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "transparent", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer" },
};

const rowStyles: Record<string, React.CSSProperties> = {
  row: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 16 },
  label: { fontSize: 13, color: "var(--color-text)", flexShrink: 0 },
  control: { display: "flex", alignItems: "center", gap: 8 },
};
