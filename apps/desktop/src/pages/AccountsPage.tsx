/**
 * AccountsPage — Bóveda de cuentas AI.
 *
 * FLUJO CORRECTO:
 * 1. "Agregar cuenta" → captura la sesión activa de agy desde Credential Manager
 *    y la guarda cifrada en la bóveda (NO hay formulario manual).
 * 2. Cada cuenta puede tener alias y notas editables.
 * 3. Eliminar borra de la bóveda.
 * 4. No hay botón "Activar" — el switch se hace desde el Dashboard.
 *
 * Para agregar una cuenta:
 *   1. Inicia sesión en Antigravity (agy) en la PC.
 *   2. Haz clic en "Importar cuenta actual".
 *   3. La app lee el token desde el Credential Manager y lo cifra en la bóveda.
 *   4. Repite para cada cuenta que quieras guardar.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import type { AccountInfo } from "../types/api";

export default function AccountsPage() {
  const [importing, setImporting] = useState(false);
  const [importAlias, setImportAlias] = useState("");
  const [showImportModal, setShowImportModal] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  const accounts = useCommand<AccountInfo[]>("list_vault_accounts", {});

  // Importar la cuenta agy actualmente activa en el Credential Manager
  const handleImport = useCallback(async () => {
    if (!importAlias.trim()) {
      setImportError("Escribe un alias para identificar esta cuenta (ej: trabajo, personal).");
      return;
    }
    setImporting(true);
    setImportError(null);
    try {
      const result = await invoke<{ success: boolean; id: string | null; error: string | null }>(
        "import_current_agy_credential",
        { alias: importAlias.trim() }
      );
      if (result.success) {
        setShowImportModal(false);
        setImportAlias("");
        accounts.refetch();
        setActionMsg(`✅ Cuenta '${importAlias.trim()}' importada y guardada en la bóveda.`);
      } else {
        setImportError(result.error ?? "Error desconocido al importar.");
      }
    } catch (e) {
      setImportError(String(e));
    } finally {
      setImporting(false);
    }
  }, [importAlias, accounts]);

  const handleDelete = useCallback(async (account: AccountInfo) => {
    if (!confirm(`¿Eliminar '${account.alias}' de la bóveda? No se puede deshacer.`)) return;
    setDeleting(account.id);
    try {
      await invoke("delete_vault_account", { accountUuid: account.id });
      accounts.refetch();
      setActionMsg(`🗑️ Cuenta '${account.alias}' eliminada de la bóveda.`);
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
            Cuentas guardadas cifradas. Cada cuenta se importa desde una sesión activa de Antigravity.
          </p>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <button style={styles.refreshBtn} onClick={accounts.refetch} disabled={accounts.loading}>
            {accounts.loading ? "⏳" : "↻"} Actualizar
          </button>
          <button
            style={styles.addBtn}
            onClick={() => { setShowImportModal(true); setImportError(null); setImportAlias(""); }}
          >
            + Agregar cuenta
          </button>
        </div>
      </div>

      {/* Instrucción contextual */}
      <div style={styles.howto}>
        <strong>¿Cómo agregar una cuenta?</strong>
        <ol style={{ margin: "6px 0 0 0", paddingLeft: 18, fontSize: 12, lineHeight: 1.7 }}>
          <li>Inicia sesión en <strong>Antigravity (agy)</strong> en esta PC con la cuenta que quieres guardar.</li>
          <li>Haz clic en <strong>"+ Agregar cuenta"</strong>.</li>
          <li>Escribe un alias para identificarla (ej: "trabajo") y confirma.</li>
          <li>Repite para cada cuenta.</li>
        </ol>
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
        <div style={styles.error}>
          {accounts.error.includes("bloqueada") || accounts.error.includes("cargada")
            ? "⚠️ La bóveda está bloqueada. Desbloquea primero desde el inicio."
            : accounts.error}
        </div>
      )}

      {/* Sin cuentas */}
      {!accounts.loading && !accounts.error && accounts.data?.length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 48 }}>🗝️</span>
          <p style={{ fontWeight: 600 }}>Tu bóveda está vacía</p>
          <p style={{ fontSize: 12, color: "var(--color-text-disabled)", maxWidth: 300, textAlign: "center" }}>
            Agrega tus cuentas de Antigravity para tenerlas siempre disponibles y poder cambiar de cuenta rápidamente.
          </p>
          <button
            style={styles.addBtn}
            onClick={() => { setShowImportModal(true); setImportError(null); setImportAlias(""); }}
          >
            + Agregar primera cuenta
          </button>
        </div>
      )}

      {/* Lista de cuentas */}
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
                  title="Eliminar esta cuenta de la bóveda"
                >
                  {deleting === account.id ? "…" : "🗑️ Eliminar"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Modal: Importar cuenta */}
      {showImportModal && (
        <div style={styles.overlay} onClick={e => { if (e.target === e.currentTarget) setShowImportModal(false); }}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <h3 style={styles.modalTitle}>Importar cuenta activa</h3>
              <button style={styles.closeBtn} onClick={() => setShowImportModal(false)}>✕</button>
            </div>

            <div style={styles.modalBody}>
              <p style={{ fontSize: 13, color: "var(--color-text-secondary)", margin: 0, lineHeight: 1.6 }}>
                Se capturará la sesión de <strong>Antigravity</strong> activa en esta PC 
                y se guardará cifrada en tu bóveda.
              </p>
              <p style={{ fontSize: 12, color: "var(--color-warning)", margin: "8px 0 0 0" }}>
                ⚠️ Asegúrate de estar logueado con la cuenta correcta en agy antes de continuar.
              </p>

              <div style={styles.formRow}>
                <label style={styles.label}>Alias para esta cuenta</label>
                <input
                  style={styles.input}
                  type="text"
                  placeholder='Ej: "trabajo", "personal", "miguel@empresa"'
                  value={importAlias}
                  onChange={e => setImportAlias(e.target.value)}
                  onKeyDown={e => e.key === "Enter" && handleImport()}
                  autoFocus
                />
                <span style={styles.fieldHint}>
                  El alias es solo para identificarla en tu bóveda. Puede ser cualquier nombre.
                </span>
              </div>

              {importError && <div style={styles.error}>{importError}</div>}
            </div>

            <div style={styles.modalFooter}>
              <button style={styles.cancelBtn} onClick={() => setShowImportModal(false)}>Cancelar</button>
              <button style={styles.saveBtn} onClick={handleImport} disabled={importing}>
                {importing ? "Importando…" : "✅ Importar y guardar"}
              </button>
            </div>
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
  subtitle: { fontSize: 12, color: "var(--color-text-disabled)", margin: "4px 0 0 0" },
  howto: { padding: "14px 16px", background: "var(--color-bg-secondary)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 12, color: "var(--color-text-secondary)", lineHeight: 1.5 },
  refreshBtn: { padding: "7px 12px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer" },
  addBtn: { padding: "7px 14px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 13, cursor: "pointer", fontWeight: 600, whiteSpace: "nowrap" as const },
  actionBar: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "10px 14px", border: "1px solid", borderRadius: "var(--radius-md)", fontSize: 13 },
  error: { padding: "10px 14px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 12, color: "#991B1B" },
  empty: { display: "flex", flexDirection: "column" as const, alignItems: "center", gap: 12, padding: 48, color: "var(--color-text-secondary)", textAlign: "center" },
  list: { display: "flex", flexDirection: "column" as const, gap: 8 },
  card: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "14px 16px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", gap: 12 },
  cardLeft: { display: "flex", flexDirection: "column" as const, gap: 3 },
  cardProvider: { fontSize: 10, fontWeight: 700, color: "var(--color-primary)", textTransform: "uppercase" as const, letterSpacing: "0.06em" },
  cardAlias: { fontSize: 15, fontWeight: 600, color: "var(--color-text)" },
  cardMeta: { fontSize: 11, color: "var(--color-text-disabled)" },
  cardActions: { display: "flex", gap: 8, flexShrink: 0 },
  deleteBtn: { padding: "7px 12px", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", background: "#FEF2F2", color: "#991B1B", fontSize: 12, cursor: "pointer" },
  // Modal
  overlay: { position: "fixed" as const, inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000 },
  modal: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: 24, width: "min(440px, 90vw)", display: "flex", flexDirection: "column" as const, gap: 16, maxHeight: "90vh", overflowY: "auto" as const },
  modalHeader: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  modalTitle: { fontSize: 16, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  modalBody: { display: "flex", flexDirection: "column" as const, gap: 12 },
  closeBtn: { background: "none", border: "none", fontSize: 16, cursor: "pointer", color: "var(--color-text-secondary)", padding: "2px 6px" },
  formRow: { display: "flex", flexDirection: "column" as const, gap: 4 },
  label: { fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)" },
  fieldHint: { fontSize: 10, color: "var(--color-text-disabled)", lineHeight: 1.4 },
  input: { padding: "9px 12px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 14, background: "var(--color-bg-secondary)", color: "var(--color-text)", outline: "none", fontFamily: "inherit" },
  modalFooter: { display: "flex", gap: 10, justifyContent: "flex-end" },
  cancelBtn: { padding: "8px 16px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer" },
  saveBtn: { padding: "8px 18px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 13, cursor: "pointer", fontWeight: 600 },
};
