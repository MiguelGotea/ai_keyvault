/** UnlockPage — Fase 19.2: Primer arranque estilo Bitwarden + desbloqueo en arranques siguientes.
 *  - Sin sesión local → Iniciar sesión (URL + usuario + clave maestra) o Crear usuario nuevo.
 *  - Con sesión local → solo clave maestra (mostrando nombre del usuario).
 *  - Recuperación → clave de respaldo AKVR-...
 */
import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useServerHealth } from "../hooks/useServerHealth";
import { useCommand } from "../hooks/useCommand";
import ServerStatusBadge from "../components/ServerStatusBadge";
import { useTheme } from "../hooks/useTheme";
import logoIconTextLight from "../assets/ai-keyvault-icon-text.svg";
import logoIconTextDark from "../assets/ai-keyvault-icon-text-dark.svg";
import es from "../i18n/es.json";

type ScreenMode = "login" | "register" | "unlock" | "recovery" | "show_recovery_key";

interface SessionInfo {
  has_session: boolean;
  username: string | null;
  display_name: string | null;
  server_url: string;
}

interface UnlockResult { success: boolean; error: string | null; accounts_count: number; }
interface CreateVaultResult { success: boolean; recovery_key: string | null; error: string | null; }

export default function UnlockPage() {
  const navigate = useNavigate();
  const health = useServerHealth();
  const theme = useTheme();

  const [mode, setMode] = useState<ScreenMode>("login");
  const [serverUrl, setServerUrl] = useState("http://198.211.97.243:3010");
  const [displayName, setDisplayName] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [recoveryInput, setRecoveryInput] = useState("");
  const [newPasswordForRecovery, setNewPasswordForRecovery] = useState("");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [recoveryKeyToShow, setRecoveryKeyToShow] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [cloudNotice, setCloudNotice] = useState<string | null>(null);

  const { data: sessionInfo } = useCommand<SessionInfo>("get_session_info");

  useEffect(() => {
    if (!sessionInfo) return;
    setServerUrl(sessionInfo.server_url);
    if (sessionInfo.has_session) {
      setMode("unlock");
    } else {
      setMode("login");
    }
  }, [sessionInfo]);

  async function startAutolockIfConfigured() {
    const autolockMinutes = parseInt(localStorage.getItem("autolock_minutes") ?? "0", 10);
    if (autolockMinutes > 0) {
      await invoke("start_autolock_timer", { minutes: autolockMinutes });
    }
  }

  async function startVaultAutosync() {
    // Intervalo configurable; default 15 minutos (mínimo 5)
    try {
      const stored = localStorage.getItem("akv_settings");
      const intervalMinutes = stored ? (JSON.parse(stored).vaultSyncIntervalMinutes ?? 15) : 15;
      await invoke("start_vault_autosync", { intervalMinutes });
    } catch (e) {
      console.warn("No se pudo iniciar auto-sync de bóveda:", e);
    }
  }

  // ── Iniciar sesión (traer usuario y bóveda desde el VPS) ────────────────────
  async function handleLogin() {
    if (!serverUrl.trim() || !username.trim() || !password) {
      return setErrorMsg("Completa URL del servidor, usuario y clave maestra.");
    }
    setLoading(true);
    setErrorMsg(null);
    try {
      const result = await invoke<UnlockResult>("login_user_and_sync", {
        serverUrl: serverUrl.trim(),
        username: username.trim(),
        password,
      });
      if (result.success) {
        try {
          await startAutolockIfConfigured();
        } catch (e) {
          setCloudNotice(`Sesión iniciada, pero falló el timer de autobloqueo: ${String(e)}`);
        }
        await startVaultAutosync();
        navigate("/dashboard");
      } else {
        setErrorMsg(result.error ?? "Credenciales inválidas.");
      }
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setLoading(false);
    }
  }

  // ── Crear usuario nuevo + bóveda ────────────────────────────────────────────
  async function handleRegister() {
    if (!displayName.trim() || !username.trim()) {
      return setErrorMsg("Ingresa tu nombre y nombre de usuario.");
    }
    if (!password) return setErrorMsg("Ingresa una clave maestra.");
    if (password !== confirmPassword) return setErrorMsg("Las claves no coinciden.");
    if (password.length < 12) return setErrorMsg("La clave debe tener al menos 12 caracteres.");

    setLoading(true);
    setErrorMsg(null);
    try {
      const result = await invoke<CreateVaultResult>("register_user_and_vault", {
        serverUrl: serverUrl.trim(),
        displayName: displayName.trim(),
        username: username.trim(),
        password,
      });
      if (result.success && result.recovery_key) {
        setRecoveryKeyToShow(result.recovery_key);
        setMode("show_recovery_key");
      } else {
        setErrorMsg(result.error ?? "Error desconocido al crear usuario y bóveda.");
      }
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setLoading(false);
    }
  }

  // ── Desbloquear bóveda (arranques siguientes) ───────────────────────────────
  async function handleUnlock() {
    if (!password) return setErrorMsg("Ingresa tu clave maestra.");
    setLoading(true);
    setErrorMsg(null);
    try {
      const result = await invoke<UnlockResult>("unlock_vault", { password });
      if (result.success) {
        try {
          await startAutolockIfConfigured();
        } catch (e) {
          setErrorMsg(`Bóveda desbloqueada, pero falló el timer de autobloqueo: ${String(e)}`);
        }
        await startVaultAutosync();
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
          <img
            src={theme === "dark" ? logoIconTextDark : logoIconTextLight}
            alt={es.app.nombre}
            style={styles.logo}
          />
          <p style={styles.subtitle}>
            {mode === "login" && "Inicia sesión para traer tu bóveda"}
            {mode === "register" && "Crea tu usuario y bóveda nueva"}
            {mode === "unlock" && (
              sessionInfo?.display_name
                ? `Hola, ${sessionInfo.display_name} (@${sessionInfo.username})`
                : es.unlock.subtitulo
            )}
            {mode === "recovery" && "Recuperación con clave de respaldo"}
            {mode === "show_recovery_key" && "Guarda tu clave de recuperación"}
          </p>
        </div>

        {/* Estado del servidor */}
        <div style={styles.statusRow}>
          <ServerStatusBadge health={health} />
          <button style={styles.recheckBtn} onClick={health.recheck} disabled={health.checking}>↻</button>
        </div>

        {/* Selector Iniciar sesión / Crear usuario cuando no hay sesión local */}
        {(mode === "login" || mode === "register") && (
          <div style={{ display: "flex", gap: 6 }}>
            <button
              style={{
                ...styles.btn,
                flex: 1,
                ...(mode === "login" ? styles.btnPrimary : {}),
              }}
              onClick={() => { setMode("login"); setErrorMsg(null); }}
            >
              Iniciar sesión
            </button>
            <button
              style={{
                ...styles.btn,
                flex: 1,
                ...(mode === "register" ? styles.btnPrimary : {}),
              }}
              onClick={() => { setMode("register"); setErrorMsg(null); }}
            >
              Crear usuario nuevo
            </button>
          </div>
        )}

        {/* Error / aviso nube */}
        {cloudNotice && !errorMsg && (
          <div style={{ ...styles.warningBox, background: "var(--color-bg-secondary)", borderColor: "var(--color-primary)", color: "var(--color-text)" }}>
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

        {/* ── PANTALLA: INICIAR SESIÓN ── */}
        {mode === "login" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>Usuario</label>
              <input
                type="text"
                placeholder="tu-usuario"
                style={styles.input}
                value={username}
                onChange={e => setUsername(e.target.value)}
                onKeyDown={e => e.key === "Enter" && handleLogin()}
                autoFocus
              />
            </div>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>{es.vault.clave_maestra}</label>
              <input
                type="password"
                placeholder="••••••••••••"
                style={styles.input}
                value={password}
                onChange={e => setPassword(e.target.value)}
                onKeyDown={e => e.key === "Enter" && handleLogin()}
              />
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnPrimary }}
              onClick={handleLogin}
              disabled={loading}
            >
              {loading ? "Iniciando sesión…" : "Iniciar sesión"}
            </button>
          </div>
        )}

        {/* ── PANTALLA: CREAR USUARIO NUEVO ── */}
        {mode === "register" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>Nombre</label>
              <input
                type="text"
                placeholder="Tu nombre"
                style={styles.input}
                value={displayName}
                onChange={e => setDisplayName(e.target.value)}
                autoFocus
              />
            </div>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>Usuario</label>
              <input
                type="text"
                placeholder="tu-usuario"
                style={styles.input}
                value={username}
                onChange={e => setUsername(e.target.value)}
              />
            </div>
            <div style={styles.inputGroup}>
              <label style={styles.inputLabel}>{es.vault.clave_maestra}</label>
              <input
                type="password"
                placeholder="Mínimo 12 caracteres"
                style={styles.input}
                value={password}
                onChange={e => setPassword(e.target.value)}
                onKeyDown={e => e.key === "Enter" && handleRegister()}
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
                onKeyDown={e => e.key === "Enter" && handleRegister()}
              />
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnPrimary }}
              onClick={handleRegister}
              disabled={loading}
            >
              {loading ? "Creando usuario y bóveda…" : "Crear usuario y bóveda"}
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
  logo: { height: 96, width: "auto", borderRadius: 12, objectFit: "contain" },
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
    border: "1px solid var(--color-btn-border)",
    background: "var(--color-btn-bg)",
    color: "var(--color-btn-text)",
    fontSize: 14,
    cursor: "pointer",
    fontWeight: 500,
  },
  btnPrimary: {
    background: "var(--color-primary)",
    color: "var(--color-on-primary)",
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
