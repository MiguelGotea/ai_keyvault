/**
 * AccountsPage — Bóveda de cuentas AI.
 *
 * FLUJO REAL DE "AGREGAR CUENTA":
 * 1. Clic en "+ Agregar cuenta" → se pide un alias.
 * 2. Se llama al comando Rust `add_agy_account_oauth`.
 * 3. Rust lanza `agy login` → se abre el navegador con el login de Google.
 * 4. El usuario se loguea normalmente en Google/Antigravity.
 * 5. Rust hace polling del Credential Manager cada 2 s.
 * 6. Cuando detecta la nueva credencial, la cifra y guarda en la bóveda.
 * 7. La cuenta aparece en la lista.
 *
 * No hay formulario manual. No hay campos de email/password/token.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import type { AccountInfo } from "../types/api";

export default function AccountsPage() {
  const [showModal, setShowModal] = useState(false);
  const [alias, setAlias] = useState("");
  const [waiting, setWaiting] = useState(false);  // esperando que el usuario haga login
  const [launched, setLaunched] = useState(false); // browser ya fue abierto
  const [error, setError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  const accounts = useCommand<AccountInfo[]>("list_vault_accounts", {});

  const openModal = () => {
    setShowModal(true);
    setAlias("");
    setError(null);
    setWaiting(false);
    setLaunched(false);
  };

  // Paso 1: usuario pone el alias y hace clic en "Iniciar login"
  const handleStartOAuth = useCallback(async () => {
    if (!alias.trim()) {
      setError("Escribe un alias para identificar esta cuenta (ej: trabajo, personal, miguel@empresa).");
      return;
    }
    setError(null);
    setWaiting(true);
    setLaunched(true);

    try {
      // Este comando:
      //   1. Lee credencial previa
      //   2. Lanza `agy login` (abre navegador)
      //   3. Hace polling hasta que llega la nueva credencial (~120 s max)
      //   4. Cifra y guarda en bóveda
      const result = await invoke<{ success: boolean; id: string | null; error: string | null }>(
        "add_agy_account_oauth",
        { alias: alias.trim() }
      );
      if (result.success) {
        setShowModal(false);
        setAlias("");
        accounts.refetch();
        setActionMsg(`✅ Cuenta '${alias.trim()}' agregada a la bóveda correctamente.`);
      } else {
        setError(result.error ?? "Error desconocido al agregar la cuenta.");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setWaiting(false);
    }
  }, [alias, accounts]);

  const handleDelete = useCallback(async (account: AccountInfo) => {
    if (!confirm(`¿Eliminar '${account.alias}' de la bóveda? No se puede deshacer.`)) return;
    setDeleting(account.id);
    try {
      await invoke("delete_vault_account", { accountUuid: account.id });
      accounts.refetch();
      setActionMsg(`🗑️ Cuenta '${account.alias}' eliminada.`);
    } catch (e) {
      setActionMsg(`❌ Error al eliminar: ${String(e)}`);
    } finally {
      setDeleting(null);
    }
  }, [accounts]);

  return (
    <div style={styles.page}>
      {/* Cabecera */}
      <div style={styles.header}>
        <div>
          <h2 style={styles.title}>🗝️ Mi Bóveda</h2>
          <p style={styles.subtitle}>
            Tus cuentas de Antigravity guardadas cifradas. Agrégalas una por una para usarlas en cualquier PC.
          </p>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <button style={styles.refreshBtn} onClick={accounts.refetch} disabled={accounts.loading}>
            {accounts.loading ? "⏳" : "↻"} Actualizar
          </button>
          <button style={styles.addBtn} onClick={openModal}>
            + Agregar cuenta
          </button>
        </div>
      </div>

      {/* Mensaje de acción */}
      {actionMsg && (
        <div style={{
          ...styles.actionBar,
          borderColor: actionMsg.startsWith("✅") ? "var(--color-success)" : actionMsg.startsWith("🗑️") ? "var(--color-border)" : "var(--color-danger)",
          color: actionMsg.startsWith("✅") ? "var(--color-success)" : actionMsg.startsWith("🗑️") ? "var(--color-text-secondary)" : "var(--color-danger)",
        }}>
          {actionMsg}
          <button style={styles.closeBtn} onClick={() => setActionMsg(null)}>✕</button>
        </div>
      )}

      {/* Error de carga */}
      {accounts.error && (
        <div style={styles.errorBox}>
          {accounts.error.includes("bloqueada") || accounts.error.includes("cargada")
            ? "⚠️ La bóveda está bloqueada. Desbloquea primero desde el inicio."
            : accounts.error}
        </div>
      )}

      {/* Sin cuentas */}
      {!accounts.loading && !accounts.error && accounts.data?.length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 56 }}>🗝️</span>
          <p style={{ fontWeight: 700, fontSize: 16 }}>Tu bóveda está vacía</p>
          <p style={{ fontSize: 13, color: "var(--color-text-disabled)", maxWidth: 340, textAlign: "center", lineHeight: 1.6 }}>
            Haz clic en <strong>"+ Agregar cuenta"</strong> para iniciar el login de Google.
            La app capturará tus credenciales automáticamente.
          </p>
          <button style={styles.addBtn} onClick={openModal}>
            + Agregar primera cuenta
          </button>
        </div>
      )}

      {/* Lista */}
      {accounts.data && accounts.data.length > 0 && (
        <div style={styles.list}>
          {accounts.data.map(account => (
            <div key={account.id} style={styles.card}>
              <div style={styles.cardLeft}>
                <div style={styles.cardProvider}>
                  {account.provider === "agy" ? "🤖 Antigravity" : "💻 Codex"}
                </div>
                <div style={styles.cardAlias}>{account.alias}</div>
                <div style={styles.cardMeta}>
                  Guardada: {new Date(account.created_at).toLocaleDateString("es-MX")}
                </div>
              </div>
              <div style={styles.cardActions}>
                <button
                  style={styles.deleteBtn}
                  onClick={() => handleDelete(account)}
                  disabled={deleting === account.id}
                >
                  {deleting === account.id ? "…" : "🗑️ Eliminar"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Modal OAuth */}
      {showModal && (
        <div style={styles.overlay} onClick={e => { if (!waiting && e.target === e.currentTarget) setShowModal(false); }}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <h3 style={styles.modalTitle}>Agregar cuenta</h3>
              {!waiting && <button style={styles.closeBtn} onClick={() => setShowModal(false)}>✕</button>}
            </div>

            {/* Estado: antes de iniciar */}
            {!launched && (
              <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
                <div style={styles.oauthInfo}>
                  <div style={styles.oauthStep}>
                    <span style={styles.oauthNum}>1</span>
                    <span>Escribe un alias para identificar la cuenta</span>
                  </div>
                  <div style={styles.oauthStep}>
                    <span style={styles.oauthNum}>2</span>
                    <span>Haz clic en <strong>"Iniciar login"</strong> → se abre el navegador con Google</span>
                  </div>
                  <div style={styles.oauthStep}>
                    <span style={styles.oauthNum}>3</span>
                    <span>Inicia sesión con la cuenta que quieres guardar</span>
                  </div>
                  <div style={styles.oauthStep}>
                    <span style={styles.oauthNum}>4</span>
                    <span>La app captura las credenciales automáticamente y las guarda cifradas</span>
                  </div>
                </div>

                <div style={styles.formRow}>
                  <label style={styles.label}>Alias para esta cuenta</label>
                  <input
                    style={styles.input}
                    type="text"
                    placeholder='Ej: "trabajo", "personal", "miguel@empresa"'
                    value={alias}
                    onChange={e => setAlias(e.target.value)}
                    onKeyDown={e => e.key === "Enter" && !waiting && handleStartOAuth()}
                    autoFocus
                  />
                  <span style={styles.fieldHint}>Solo para reconocerla en tu bóveda. No tiene que ser el email.</span>
                </div>

                {error && <div style={styles.errorBox}>{error}</div>}

                <div style={styles.modalFooter}>
                  <button style={styles.cancelBtn} onClick={() => setShowModal(false)}>Cancelar</button>
                  <button style={styles.saveBtn} onClick={handleStartOAuth}>
                    🔑 Iniciar login con Google
                  </button>
                </div>
              </div>
            )}

            {/* Estado: esperando que el usuario haga login */}
            {launched && waiting && (
              <div style={{ display: "flex", flexDirection: "column", gap: 16, alignItems: "center", padding: "20px 0" }}>
                <div style={styles.spinner}>⏳</div>
                <p style={{ fontWeight: 700, fontSize: 15, margin: 0 }}>
                  Esperando que completes el login…
                </p>
                <p style={{ fontSize: 13, color: "var(--color-text-secondary)", textAlign: "center", lineHeight: 1.6, margin: 0 }}>
                  El navegador debería haberse abierto con la página de Google.<br/>
                  Inicia sesión con la cuenta <strong>"{alias}"</strong> y la app capturará las credenciales automáticamente.
                </p>
                <div style={styles.waitHint}>
                  Tienes hasta <strong>2 minutos</strong> para completar el login.
                </div>
                {error && <div style={{ ...styles.errorBox, width: "100%" }}>{error}</div>}
              </div>
            )}

            {/* Estado: error después de lanzar */}
            {launched && !waiting && error && (
              <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
                <div style={styles.errorBox}>{error}</div>
                <div style={styles.modalFooter}>
                  <button style={styles.cancelBtn} onClick={() => setShowModal(false)}>Cerrar</button>
                  <button style={styles.saveBtn} onClick={() => { setLaunched(false); setError(null); }}>
                    🔄 Intentar de nuevo
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 16 },
  header: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16 },
  title: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  subtitle: { fontSize: 12, color: "var(--color-text-disabled)", margin: "4px 0 0 0", lineHeight: 1.5 },
  refreshBtn: { padding: "7px 12px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer", whiteSpace: "nowrap" as const },
  addBtn: { padding: "7px 16px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 13, cursor: "pointer", fontWeight: 700, whiteSpace: "nowrap" as const },
  actionBar: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "10px 14px", border: "1px solid", borderRadius: "var(--radius-md)", fontSize: 13 },
  errorBox: { padding: "10px 14px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 12, color: "#991B1B" },
  empty: { display: "flex", flexDirection: "column" as const, alignItems: "center", gap: 14, padding: "56px 24px", color: "var(--color-text-secondary)", textAlign: "center" },
  list: { display: "flex", flexDirection: "column" as const, gap: 8 },
  card: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "14px 18px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", gap: 12 },
  cardLeft: { display: "flex", flexDirection: "column" as const, gap: 3 },
  cardProvider: { fontSize: 10, fontWeight: 700, color: "var(--color-primary)", textTransform: "uppercase" as const, letterSpacing: "0.06em" },
  cardAlias: { fontSize: 15, fontWeight: 600, color: "var(--color-text)" },
  cardMeta: { fontSize: 11, color: "var(--color-text-disabled)" },
  cardActions: { display: "flex", gap: 8, flexShrink: 0 },
  deleteBtn: { padding: "7px 12px", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", background: "#FEF2F2", color: "#991B1B", fontSize: 12, cursor: "pointer" },
  // Modal
  overlay: { position: "fixed" as const, inset: 0, background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000 },
  modal: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: 28, width: "min(460px, 92vw)", display: "flex", flexDirection: "column" as const, gap: 16, maxHeight: "90vh", overflowY: "auto" as const },
  modalHeader: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  modalTitle: { fontSize: 17, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  closeBtn: { background: "none", border: "none", fontSize: 16, cursor: "pointer", color: "var(--color-text-secondary)", padding: "2px 6px" },
  oauthInfo: { display: "flex", flexDirection: "column" as const, gap: 10, padding: "14px 16px", background: "var(--color-bg-secondary)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)" },
  oauthStep: { display: "flex", alignItems: "flex-start", gap: 10, fontSize: 13, color: "var(--color-text-secondary)", lineHeight: 1.5 },
  oauthNum: { background: "var(--color-primary)", color: "#fff", borderRadius: "50%", width: 20, height: 20, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 11, fontWeight: 700, flexShrink: 0 } as React.CSSProperties,
  formRow: { display: "flex", flexDirection: "column" as const, gap: 5 },
  label: { fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)" },
  fieldHint: { fontSize: 10, color: "var(--color-text-disabled)", lineHeight: 1.4 },
  input: { padding: "10px 12px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 14, background: "var(--color-bg-secondary)", color: "var(--color-text)", outline: "none", fontFamily: "inherit" },
  modalFooter: { display: "flex", gap: 10, justifyContent: "flex-end" },
  cancelBtn: { padding: "8px 16px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer" },
  saveBtn: { padding: "9px 20px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 13, cursor: "pointer", fontWeight: 700 },
  spinner: { fontSize: 40, animation: "spin 2s linear infinite" },
  waitHint: { padding: "8px 14px", background: "#FFFBEB", border: "1px solid #FDE68A", borderRadius: "var(--radius-md)", fontSize: 12, color: "#92400E" },
};
