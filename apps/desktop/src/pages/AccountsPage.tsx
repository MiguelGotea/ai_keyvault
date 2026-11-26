/**
 * AccountsPage — Bóveda de cuentas AI.
 *
 * FLUJO REAL DE "AGREGAR CUENTA":
 *
 * Para agy (Antigravity):
 *   1. Inicia sesión en Antigravity (agy) en esta PC con la cuenta que quieres guardar.
 *   2. Haz clic en "+ Agregar cuenta" → elige "Antigravity (agy)".
 *   3. Escribe un alias y confirma.
 *   4. Rust lee el token desde Credential Manager ("gemini:antigravity") y lo cifra en la bóveda.
 *
 * Para Codex:
 *   1. Inicia sesión en Codex en esta PC con la cuenta que quieres guardar.
 *   2. Haz clic en "+ Agregar cuenta" → elige "Codex".
 *   3. Escribe un alias y confirma.
 *   4. Rust lee el auth.json (~/.codex/auth.json) y lo cifra en la bóveda.
 *
 * No hay formulario manual. No hay campos de email/password/token.
 * El switch ("Usar en esta PC") está solo en el Dashboard.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import type { AccountInfo } from "../types/api";

type ProviderType = "agy" | "codex";

export default function AccountsPage() {
  const [showModal, setShowModal] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState<ProviderType>("agy");
  const [importAlias, setImportAlias] = useState("");
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  const accounts = useCommand<AccountInfo[]>("list_vault_accounts", {});

  const openModal = () => {
    setShowModal(true);
    setSelectedProvider("agy");
    setImportAlias("");
    setImportError(null);
  };

  // Importar la cuenta activa según el proveedor
  const handleImport = useCallback(async () => {
    if (!importAlias.trim()) {
      setImportError("Escribe un alias para identificar esta cuenta (ej: trabajo, personal).");
      return;
    }
    setImporting(true);
    setImportError(null);

    try {
      let result: { success: boolean; id: string | null; error: string | null };

      if (selectedProvider === "agy") {
        // Lee de "gemini:antigravity" en el Credential Manager de Windows
        result = await invoke("import_current_agy_credential", {
          alias: importAlias.trim(),
        });
      } else {
        // Lee de ~/.codex/auth.json
        result = await invoke("import_current_codex_credential", {
          alias: importAlias.trim(),
        });
      }

      if (result.success) {
        setShowModal(false);
        setImportAlias("");
        accounts.refetch();
        const pName = selectedProvider === "agy" ? "Antigravity" : "Codex";
        setActionMsg(`✅ Cuenta ${pName} '${importAlias.trim()}' guardada en la bóveda.`);
      } else {
        setImportError(result.error ?? "Error desconocido al importar.");
      }
    } catch (e) {
      setImportError(String(e));
    } finally {
      setImporting(false);
    }
  }, [importAlias, selectedProvider, accounts]);

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

  const providerInstructions = {
    agy: (
      <>
        <strong>Requisito:</strong> tener Antigravity (<code>agy</code>) instalado y logueado en esta PC
        con la cuenta que quieres guardar.
        <br />
        La app captura el token de <code>gemini:antigravity</code> en el Credential Manager.
      </>
    ),
    codex: (
      <>
        <strong>Requisito:</strong> tener Codex instalado y logueado en esta PC
        con la cuenta que quieres guardar.
        <br />
        La app captura el token de <code>~/.codex/auth.json</code>.
      </>
    ),
  };

  return (
    <div style={styles.page}>
      {/* Cabecera */}
      <div style={styles.header}>
        <div>
          <h2 style={styles.title}>🗝️ Mi Bóveda</h2>
          <p style={styles.subtitle}>
            Cuentas guardadas cifradas. Cada cuenta se importa desde una sesión activa.
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

      {/* Instrucción contextual */}
      <div style={styles.howto}>
        <strong>¿Cómo agregar una cuenta?</strong>
        <ol style={{ margin: "6px 0 0 0", paddingLeft: 18, fontSize: 12, lineHeight: 1.7 }}>
          <li>Inicia sesión en <strong>Antigravity (agy)</strong> o <strong>Codex</strong> en esta PC con la cuenta que quieres guardar.</li>
          <li>Haz clic en <strong>"+ Agregar cuenta"</strong>, elige el proveedor y escribe un alias.</li>
          <li>La app captura automáticamente el token activo y lo cifra en la bóveda.</li>
          <li>Repite para cada cuenta — puedes tener todas las que quieras.</li>
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
            Agrega tus cuentas de Antigravity y Codex para tenerlas disponibles en cualquier PC y cambiar rápidamente entre ellas.
          </p>
          <button style={styles.addBtn} onClick={openModal}>
            + Agregar primera cuenta
          </button>
        </div>
      )}

      {/* Lista de cuentas agrupadas por proveedor */}
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
      {showModal && (
        <div
          style={styles.overlay}
          onClick={e => { if (e.target === e.currentTarget) setShowModal(false); }}
        >
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <h3 style={styles.modalTitle}>Agregar cuenta a la bóveda</h3>
              <button style={styles.closeBtn} onClick={() => setShowModal(false)}>✕</button>
            </div>

            <div style={styles.modalBody}>
              {/* Selector de proveedor */}
              <div style={styles.formRow}>
                <label style={styles.label}>Proveedor</label>
                <div style={styles.providerTabs}>
                  <button
                    style={{
                      ...styles.providerTab,
                      ...(selectedProvider === "agy" ? styles.providerTabActive : {}),
                    }}
                    onClick={() => setSelectedProvider("agy")}
                  >
                    🤖 Antigravity (agy)
                  </button>
                  <button
                    style={{
                      ...styles.providerTab,
                      ...(selectedProvider === "codex" ? styles.providerTabActive : {}),
                    }}
                    onClick={() => setSelectedProvider("codex")}
                  >
                    💻 Codex
                  </button>
                </div>
              </div>

              {/* Instrucciones del proveedor seleccionado */}
              <div style={styles.infoBox}>
                <span style={{ fontSize: 11, color: "var(--color-text-secondary)", lineHeight: 1.6 }}>
                  {providerInstructions[selectedProvider]}
                </span>
              </div>

              {/* Alias */}
              <div style={styles.formRow}>
                <label style={styles.label}>Alias para identificar esta cuenta</label>
                <input
                  style={styles.input}
                  type="text"
                  placeholder='Ej: "trabajo", "personal", "miguel35"'
                  value={importAlias}
                  onChange={e => setImportAlias(e.target.value)}
                  onKeyDown={e => e.key === "Enter" && handleImport()}
                  autoFocus
                />
                <span style={styles.fieldHint}>
                  El alias es solo para identificarla. Puede ser cualquier nombre corto.
                </span>
              </div>

              {importError && <div style={styles.error}>{importError}</div>}
            </div>

            <div style={styles.modalFooter}>
              <button style={styles.cancelBtn} onClick={() => setShowModal(false)}>Cancelar</button>
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
  modal: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: 24, width: "min(480px, 90vw)", display: "flex", flexDirection: "column" as const, gap: 16, maxHeight: "90vh", overflowY: "auto" as const },
  modalHeader: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  modalTitle: { fontSize: 16, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  modalBody: { display: "flex", flexDirection: "column" as const, gap: 12 },
  closeBtn: { background: "none", border: "none", fontSize: 16, cursor: "pointer", color: "var(--color-text-secondary)", padding: "2px 6px" },
  formRow: { display: "flex", flexDirection: "column" as const, gap: 4 },
  label: { fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)" },
  fieldHint: { fontSize: 10, color: "var(--color-text-disabled)", lineHeight: 1.4 },
  input: { padding: "9px 12px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 14, background: "var(--color-bg-secondary)", color: "var(--color-text)", outline: "none", fontFamily: "inherit" },
  infoBox: { padding: "10px 12px", background: "var(--color-bg-secondary)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)" },
  providerTabs: { display: "flex", gap: 8 },
  providerTab: { flex: 1, padding: "8px 12px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer", fontWeight: 500, textAlign: "center" as const },
  providerTabActive: { borderColor: "var(--color-primary)", background: "rgba(6,78,59,0.08)", color: "var(--color-primary)", fontWeight: 700 },
  modalFooter: { display: "flex", gap: 10, justifyContent: "flex-end" },
  cancelBtn: { padding: "8px 16px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer" },
  saveBtn: { padding: "8px 18px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 13, cursor: "pointer", fontWeight: 600 },
};
