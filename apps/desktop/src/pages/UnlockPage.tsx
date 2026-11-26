/** UnlockPage — Fase 12: pantalla de desbloqueo con bóveda local real.
 *  - Primer uso → flujo de creación con clave maestra
 *  - Uso normal → desbloqueo con clave maestra
 *  - Recuperación → clave de respaldo AKVR-...
 */
import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useServerHealth } from "../hooks/useServerHealth";
import { useCommand } from "../hooks/useCommand";
import ServerStatusBadge from "../components/ServerStatusBadge";
import type { VaultStatus } from "../types/api";
import es from "../i18n/es.json";

type ScreenMode = "unlock" | "create" | "recovery" | "show_recovery_key";

interface UnlockResult { success: boolean; error: string | null; accounts_count: number; }
interface CreateVaultResult { success: boolean; recovery_key: string | null; error: string | null; }

export default function UnlockPage() {
  const navigate = useNavigate();
  const health = useServerHealth();

  const [mode, setMode] = useState<ScreenMode>("unlock");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [recoveryInput, setRecoveryInput] = useState("");
  const [newPasswordForRecovery, setNewPasswordForRecovery] = useState("");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [recoveryKeyToShow, setRecoveryKeyToShow] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [checkingCloud, setCheckingCloud] = useState(false);
  const [cloudNotice, setCloudNotice] = useState<string | null>(null);

  // Verificar si existe bóveda en disco
  const { data: vaultStatus, refetch: refetchVaultStatus } = useCommand<VaultStatus>("get_vault_status");

  // Si no hay bóveda local, intentar descargarla del VPS antes de mostrar "Crear bóveda"
  async function checkCloudVault() {
    setCheckingCloud(true);
    setErrorMsg(null);
    try {
      const res = await invoke<{ success: boolean; direction: string; error: string | null }>(
        "download_vault_sync"
      );
      if (res.success && res.direction === "download") {
        await refetchVaultStatus();
        setMode("unlock");
        setCloudNotice("☁️ Bóveda encontrada en la nube. Ingresa tu clave maestra para desbloquear.");
      } else {
        setMode("create");
      }
    } catch (e) {
      setErrorMsg(`No se pudo consultar la bóveda en la nube: ${String(e)}`);
      setMode("create");
    } finally {
      setCheckingCloud(false);
    }
  }

  useEffect(() => {
    if (vaultStatus && !vaultStatus.initialized) {
      checkCloudVault();
    } else if (vaultStatus && vaultStatus.initialized) {
      setMode("unlock");
    }
  }, [vaultStatus?.initialized]);

  // ── Crear bóveda nueva ──────────────────────────────────────────────────────
  async function handleCreate() {
    if (!password) return setErrorMsg("Ingresa una clave maestra.");
    if (password !== confirmPassword) return setErrorMsg("Las claves no coinciden.");
    if (password.length < 12) return setErrorMsg("La clave debe tener al menos 12 caracteres.");

    setLoading(true);
    setErrorMsg(null);
    try {
      const result = await invoke<CreateVaultResult>("create_vault", { password });
      if (result.success && result.recovery_key) {
        setRecoveryKeyToShow(result.recovery_key);
        setMode("show_recovery_key");
      } else {
        setErrorMsg(result.error ?? "Error desconocido al crear la bóveda.");
      }
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setLoading(false);
    }
  }

  // ── Desbloquear bóveda ──────────────────────────────────────────────────────
  async function handleUnlock() {
    if (!password) return setErrorMsg("Ingresa tu clave maestra.");
    setLoading(true);
    setErrorMsg(null);
    try {
      const result = await invoke<UnlockResult>("unlock_vault", { password });
      if (result.success) {
        // Fase 14: iniciar timer de autobloqueo (0 = desactivado por defecto)
        const autolockMinutes = parseInt(localStorage.getItem("autolock_minutes") ?? "0");
        if (autolockMinutes > 0) {
          try {
            await invoke("start_autolock_timer", { minutes: autolockMinutes });
          } catch (e) {
            setErrorMsg(`Bóveda desbloqueada, pero falló el timer de autobloqueo: ${String(e)}`);
          }
        }
        // Sincronizar bóveda del VPS antes de entrar al dashboard
        try {
          const syncRes = await invoke<{ success: boolean; error: string | null }>("download_vault_sync");
          if (!syncRes.success && syncRes.error) {
            setCloudNotice(`Aviso de sincronización: ${syncRes.error}`);
          }
        } catch (e) {
          setCloudNotice(`No se pudo sincronizar con el VPS: ${String(e)}`);
        }
        navigate("/dashboard");
      } else {
        setErrorMsg(result.error ?? "Clave incorrecta.");
      }
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setLoading(false);
    }
  }

  // ── Recuperar bóveda ────────────────────────────────────────────────────────
  async function handleRecover() {
    if (!recoveryInput) return setErrorMsg("Ingresa la clave de recuperación.");
    if (!newPasswordForRecovery || newPasswordForRecovery.length < 12)
      return setErrorMsg("La nueva clave debe tener al menos 12 caracteres.");
    setLoading(true);
    setErrorMsg(null);
    try {
      const result = await invoke<UnlockResult>("recover_vault", {
        recoveryKeyStr: recoveryInput,
        newPassword: newPasswordForRecovery,
      });
      if (result.success) {
        // Tras recuperación también iniciamos sesión limpia
        const autolockMinutes = parseInt(localStorage.getItem("autolock_minutes") ?? "0");
        if (autolockMinutes > 0) {
          invoke("start_autolock_timer", { minutes: autolockMinutes }).catch(() => {});
        }
        navigate("/dashboard");
      } else {
        setErrorMsg(result.error ?? "Clave de recuperación incorrecta.");
      }
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setLoading(false);
    }
  }

  // ── Copiar clave de recuperación ────────────────────────────────────────────
  async function copyRecoveryKey() {
    if (!recoveryKeyToShow) return;
    try {
      await navigator.clipboard.writeText(recoveryKeyToShow);
      setCopied(true);
      setTimeout(() => setCopied(false), 3000);
    } catch {
      setErrorMsg("No se pudo copiar. Escríbela manualmente.");
    }
  }

  // ── Render ──────────────────────────────────────────────────────────────────
  return (
    <div style={styles.container}>
      <div style={styles.card}>
        {/* Header */}
        <div style={styles.header}>
          <span style={styles.icon}>🔐</span>
          <h1 style={styles.title}>{es.app.nombre}</h1>
          <p style={styles.subtitle}>
            {mode === "create" && es.vault.primer_uso}
            {mode === "unlock" && es.unlock.subtitulo}
            {mode === "recovery" && "Recuperación con clave de respaldo"}
            {mode === "show_recovery_key" && "Guarda tu clave de recuperación"}
          </p>
        </div>

        {/* Estado del servidor */}
        <div style={styles.statusRow}>
          <ServerStatusBadge health={health} />
          <button style={styles.recheckBtn} onClick={health.recheck} disabled={health.checking}>↻</button>
        </div>

        {/* Error / aviso nube */}
        {checkingCloud && (
          <div style={styles.warningBox}>☁️ Verificando si ya tienes una bóveda en la nube…</div>
        )}
        {cloudNotice && !errorMsg && (
          <div style={{ ...styles.warningBox, background: "#ECFDF5", borderColor: "#A7F3D0", color: "#065F46" }}>
            {cloudNotice}
          </div>
        )}
        {errorMsg && <div style={styles.errorBox}>⚠️ {errorMsg}</div>}

        {/* ── PANTALLA: MOSTRAR CLAVE DE RECUPERACIÓN ── */}
        {mode === "show_recovery_key" && recoveryKeyToShow && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={styles.warningBox}>
              <strong>⚠️ Guarda esta clave ahora mismo.</strong><br />
              Esta es tu única forma de recuperar la bóveda si olvidas la clave maestra.
              <strong> No se mostrará de nuevo.</strong>
            </div>
            <div style={styles.recoveryKeyBox}>
              <code style={styles.recoveryKeyText}>{recoveryKeyToShow}</code>
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button style={{ ...styles.btn, flex: 1 }} onClick={copyRecoveryKey}>
                {copied ? "✅ Copiado" : "📋 Copiar al portapapeles"}
              </button>
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnPrimary }}
              onClick={() => navigate("/dashboard")}
            >
              Ya guardé la clave → Continuar
            </button>
          </div>
        )}

        {/* ── PANTALLA: CREAR BÓVEDA ── */}
        {mode === "create" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>{es.vault.clave_maestra}</label>
              <input
                type="password"
                placeholder="Mínimo 12 caracteres"
                style={styles.input}
                value={password}
                onChange={e => setPassword(e.target.value)}
                onKeyDown={e => e.key === "Enter" && handleCreate()}
                autoFocus
              />
            </div>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>Confirmar clave maestra</label>
              <input
                type="password"
                placeholder="Repite la clave"
                style={styles.input}
                value={confirmPassword}
                onChange={e => setConfirmPassword(e.target.value)}
                onKeyDown={e => e.key === "Enter" && handleCreate()}
              />
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnPrimary }}
              onClick={handleCreate}
              disabled={loading || checkingCloud}
            >
              {loading ? "Creando…" : es.vault.crear_boveda}
            </button>
            <button
              style={styles.btnLink}
              onClick={checkCloudVault}
              disabled={checkingCloud}
            >
              ☁️ ¿Ya tienes bóveda en la nube? Reintentar descarga
            </button>
          </div>
        )}

        {/* ── PANTALLA: DESBLOQUEAR ── */}
        {mode === "unlock" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>{es.vault.clave_maestra}</label>
              <input
                type="password"
                placeholder="••••••••••••"
                style={styles.input}
                value={password}
                onChange={e => setPassword(e.target.value)}
                onKeyDown={e => e.key === "Enter" && handleUnlock()}
                autoFocus
              />
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnPrimary }}
              onClick={handleUnlock}
              disabled={loading}
            >
              {loading ? "Verificando…" : es.vault.desbloquear}
            </button>
            <button
              style={styles.btnLink}
              onClick={() => { setMode("recovery"); setErrorMsg(null); }}
            >
              {es.unlock.enlace_recuperacion}
            </button>
          </div>
        )}

        {/* ── PANTALLA: RECUPERACIÓN ── */}
        {mode === "recovery" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>Clave de recuperación (AKVR-…)</label>
              <input
                type="text"
                placeholder="AKVR-XXXX-XXXX-XXXX-…"
                style={{ ...styles.input, fontFamily: "var(--font-mono)", fontSize: 12 }}
                value={recoveryInput}
                onChange={e => setRecoveryInput(e.target.value)}
                autoFocus
              />
            </div>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>Nueva clave maestra</label>
              <input
                type="password"
                placeholder="Mínimo 12 caracteres"
                style={styles.input}
                value={newPasswordForRecovery}
                onChange={e => setNewPasswordForRecovery(e.target.value)}
                onKeyDown={e => e.key === "Enter" && handleRecover()}
              />
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnPrimary }}
              onClick={handleRecover}
              disabled={loading}
            >
              {loading ? "Verificando…" : "Recuperar bóveda"}
            </button>
            <button
              style={styles.btnLink}
              onClick={() => { setMode("unlock"); setErrorMsg(null); }}
            >
              ← Volver al desbloqueo normal
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    minHeight: "100vh",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    background: "var(--color-bg-secondary)",
    padding: 24,
  },
  card: {
    background: "var(--color-bg)",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-xl)",
    padding: 40,
    maxWidth: 460,
    width: "100%",
    display: "flex",
    flexDirection: "column",
    gap: 20,
    boxShadow: "var(--shadow-lg)",
  },
  header: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: 8,
    textAlign: "center",
  },
  icon: { fontSize: 48, lineHeight: 1 },
  title: { fontSize: 22, fontWeight: 700, color: "var(--color-primary)", margin: 0 },
  subtitle: { fontSize: 13, color: "var(--color-text-secondary)", margin: 0 },
  statusRow: {
    display: "flex",
    gap: 8,
    alignItems: "center",
    padding: "8px 12px",
    background: "var(--color-bg-secondary)",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-border-subtle)",
  },
  recheckBtn: {
    marginLeft: "auto",
    background: "none",
    border: "none",
    cursor: "pointer",
    color: "var(--color-text-secondary)",
    fontSize: 14,
    padding: "2px 6px",
  },
  errorBox: {
    padding: "10px 14px",
    background: "#FEF2F2",
    border: "1px solid #FECACA",
    borderRadius: "var(--radius-md)",
    fontSize: 12,
    color: "#991B1B",
  },
  warningBox: {
    padding: "12px 14px",
    background: "#FFFBEB",
    border: "1px solid #FDE68A",
    borderRadius: "var(--radius-md)",
    fontSize: 12,
    color: "#92400E",
    lineHeight: 1.5,
  },
  recoveryKeyBox: {
    padding: "14px 16px",
    background: "var(--color-bg-secondary)",
    border: "2px solid var(--color-secondary)",
    borderRadius: "var(--radius-md)",
    textAlign: "center",
  },
  recoveryKeyText: {
    fontFamily: "var(--font-mono)",
    fontSize: 12,
    color: "var(--color-text)",
    wordBreak: "break-all",
    letterSpacing: "0.05em",
  },
  inputGroup: { display: "flex", flexDirection: "column", gap: 6 },
  inputLabel: { fontSize: 13, fontWeight: 500, color: "var(--color-text)" },
  input: {
    padding: "10px 14px",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-md)",
    fontSize: 15,
    outline: "none",
    background: "var(--color-bg)",
    color: "var(--color-text)",
  },
  btn: {
    padding: "11px 16px",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-border)",
    background: "var(--color-bg-secondary)",
    color: "var(--color-text)",
    fontSize: 14,
    cursor: "pointer",
    fontWeight: 500,
  },
  btnPrimary: {
    background: "var(--color-primary)",
    color: "#fff",
    border: "none",
  },
  btnLink: {
    background: "none",
    border: "none",
    color: "var(--color-text-secondary)",
    fontSize: 12,
    cursor: "pointer",
    textAlign: "center" as const,
    textDecoration: "underline",
  },
};
