/**
 * AccountsPage — Fase 13b: Gestión de cuentas en la bóveda local.
 * Permite agregar, ver y eliminar cuentas cifradas.
 * También permite activar (switch) una cuenta desde aquí.
 *
 * NOTA: El "alias" de cada cuenta debe coincidir con el email de la cuenta
 * para que el switch automático desde el Dashboard funcione.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCommand } from "../hooks/useCommand";
import type { AccountInfo } from "../types/api";

interface AddForm {
  provider: "agy" | "codex";
  alias: string;
  email: string;
  password: string;
  tokens_json: string;
  notes: string;
}

const EMPTY_FORM: AddForm = {
  provider: "agy",
  alias: "",
  email: "",
  password: "",
  tokens_json: "",
  notes: "",
};

export default function AccountsPage() {
  const [showModal, setShowModal] = useState(false);
  const [form, setForm] = useState<AddForm>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [switching, setSwitching] = useState<string | null>(null);
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  const accounts = useCommand<AccountInfo[]>("list_vault_accounts", {});

  const handleSave = useCallback(async () => {
    if (!form.alias.trim() || !form.email.trim()) {
      setSaveError("El alias y el email son obligatorios.");
      return;
    }
    setSaving(true);
    setSaveError(null);
    try {
      const result = await invoke<{ success: boolean; id: string | null; error: string | null }>(
        "add_vault_account",
        {
          provider: form.provider,
          alias: form.alias.trim(),
          email: form.email.trim(),
          password: form.password.trim() || null,
          tokensJson: form.tokens_json.trim() || null,
          notes: form.notes.trim() || null,
        }
      );
      if (result.success) {
        setShowModal(false);
        setForm(EMPTY_FORM);
        accounts.refetch();
        setActionMsg(`✅ Cuenta '${form.alias}' agregada correctamente.`);
      } else {
        setSaveError(result.error ?? "Error desconocido al agregar la cuenta.");
      }
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSaving(false);
    }
  }, [form, accounts]);

  const handleDelete = useCallback(async (account: AccountInfo) => {
    if (!confirm(`¿Eliminar la cuenta '${account.alias}' de la bóveda? Esta acción no se puede deshacer.`)) return;
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

  const handleSwitch = useCallback(async (account: AccountInfo) => {
    setSwitching(account.id);
    setActionMsg(null);
    try {
      const result = await invoke<{
        success: boolean;
        error: string | null;
        restart_notice: string | null;
      }>("switch_account", {
        accountId: account.id,
        provider: account.provider,
        deviceId: "desktop-dev",
      });
      if (result.success) {
        setActionMsg(`✅ Switch a '${account.alias}' realizado.${result.restart_notice ? " " + result.restart_notice : ""}`);
      } else {
        setActionMsg(`❌ ${result.error ?? "Error desconocido al hacer switch."}`);
      }
    } catch (e) {
      setActionMsg(`❌ Error: ${String(e)}`);
    } finally {
      setSwitching(null);
    }
  }, []);

  return (
    <div style={styles.page}>
      {/* Cabecera */}
      <div style={styles.header}>
        <h2 style={styles.title}>🗝️ Mis Cuentas (Bóveda)</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <button style={styles.refreshBtn} onClick={accounts.refetch} disabled={accounts.loading}>
            {accounts.loading ? "⏳" : "↻"} Actualizar
          </button>
          <button style={styles.addBtn} onClick={() => { setShowModal(true); setSaveError(null); setForm(EMPTY_FORM); }}>
            + Agregar cuenta
          </button>
        </div>
      </div>

      <p style={styles.hint}>
        Las cuentas aquí guardadas están cifradas con tu clave maestra.
        El <strong>alias</strong> debe ser el email de la cuenta para que el switch desde el Dashboard funcione.
      </p>

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
            ? "⚠️ La bóveda está bloqueada. Desbloquea primero desde la pantalla de inicio."
            : accounts.error}
        </div>
      )}

      {/* Sin cuentas */}
      {!accounts.loading && !accounts.error && accounts.data?.length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 40 }}>🗝️</span>
          <p>No hay cuentas en la bóveda.</p>
          <p style={{ fontSize: 12, color: "var(--color-text-disabled)" }}>
            Agrega una cuenta para poder hacer switch desde el Dashboard.
          </p>
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
                  Creada: {new Date(account.created_at).toLocaleDateString("es-MX")}
                </div>
              </div>
              <div style={styles.cardActions}>
                <button
                  style={styles.switchBtn}
                  onClick={() => handleSwitch(account)}
                  disabled={switching === account.id}
                  title="Activar esta cuenta en el sistema"
                >
                  {switching === account.id ? "Cambiando…" : "⚡ Activar"}
                </button>
                <button
                  style={styles.deleteBtn}
                  onClick={() => handleDelete(account)}
                  disabled={deleting === account.id}
                  title="Eliminar esta cuenta de la bóveda"
                >
                  {deleting === account.id ? "…" : "🗑️"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Modal de agregar cuenta */}
      {showModal && (
        <div style={styles.overlay} onClick={e => { if (e.target === e.currentTarget) setShowModal(false); }}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <h3 style={styles.modalTitle}>Agregar cuenta a la bóveda</h3>
              <button style={styles.closeBtn} onClick={() => setShowModal(false)}>✕</button>
            </div>

            <div style={styles.formRow}>
              <label style={styles.label}>Proveedor</label>
              <select
                style={styles.select}
                value={form.provider}
                onChange={e => setForm(f => ({ ...f, provider: e.target.value as "agy" | "codex" }))}
              >
                <option value="agy">🤖 Antigravity (agy)</option>
                <option value="codex">💻 Codex</option>
              </select>
            </div>

            <div style={styles.formRow}>
              <label style={styles.label}>Alias (= email de la cuenta)</label>
              <input
                style={styles.input}
                type="text"
                placeholder="usuario@gmail.com"
                value={form.alias}
                onChange={e => setForm(f => ({ ...f, alias: e.target.value }))}
              />
              <span style={styles.fieldHint}>El alias debe ser el email para que el switch automático funcione.</span>
            </div>

            <div style={styles.formRow}>
              <label style={styles.label}>Email (confirmación)</label>
              <input
                style={styles.input}
                type="email"
                placeholder="usuario@gmail.com"
                value={form.email}
                onChange={e => setForm(f => ({ ...f, email: e.target.value }))}
              />
            </div>

            <div style={styles.formRow}>
              <label style={styles.label}>Contraseña (opcional)</label>
              <input
                style={styles.input}
                type="password"
                placeholder="Solo si aplica"
                value={form.password}
                onChange={e => setForm(f => ({ ...f, password: e.target.value }))}
              />
            </div>

            <div style={styles.formRow}>
              <label style={styles.label}>Tokens JSON (opcional)</label>
              <textarea
                style={{ ...styles.input, minHeight: 60, resize: "vertical" as const }}
                placeholder='{"access_token": "...", ...}'
                value={form.tokens_json}
                onChange={e => setForm(f => ({ ...f, tokens_json: e.target.value }))}
              />
            </div>

            <div style={styles.formRow}>
              <label style={styles.label}>Notas (opcional)</label>
              <input
                style={styles.input}
                type="text"
                placeholder="Cuenta de trabajo, etc."
                value={form.notes}
                onChange={e => setForm(f => ({ ...f, notes: e.target.value }))}
              />
            </div>

            {saveError && <div style={styles.error}>{saveError}</div>}

            <div style={styles.modalFooter}>
              <button style={styles.cancelBtn} onClick={() => setShowModal(false)}>Cancelar</button>
              <button style={styles.saveBtn} onClick={handleSave} disabled={saving}>
                {saving ? "Guardando…" : "Guardar en bóveda"}
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
  header: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  title: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  hint: { fontSize: 12, color: "var(--color-text-disabled)", margin: 0, lineHeight: 1.5 },
  refreshBtn: { padding: "7px 12px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer" },
  addBtn: { padding: "7px 14px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 13, cursor: "pointer", fontWeight: 600 },
  actionBar: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "10px 14px", border: "1px solid", borderRadius: "var(--radius-md)", fontSize: 13 },
  error: { padding: "10px 14px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 12, color: "#991B1B" },
  empty: { display: "flex", flexDirection: "column", alignItems: "center", gap: 12, padding: 48, color: "var(--color-text-secondary)", textAlign: "center" },
  list: { display: "flex", flexDirection: "column", gap: 8 },
  card: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "14px 16px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", gap: 12 },
  cardLeft: { display: "flex", flexDirection: "column" as const, gap: 3 },
  cardProvider: { fontSize: 10, fontWeight: 700, color: "var(--color-primary)", textTransform: "uppercase" as const, letterSpacing: "0.06em" },
  cardAlias: { fontSize: 14, fontWeight: 600, color: "var(--color-text)", fontFamily: "var(--font-mono)" },
  cardMeta: { fontSize: 11, color: "var(--color-text-disabled)" },
  cardActions: { display: "flex", gap: 8, flexShrink: 0 },
  switchBtn: { padding: "7px 14px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 12, cursor: "pointer", fontWeight: 600 },
  deleteBtn: { padding: "7px 10px", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", background: "#FEF2F2", color: "#991B1B", fontSize: 14, cursor: "pointer" },
  // Modal
  overlay: { position: "fixed" as const, inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000 },
  modal: { background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", padding: 24, width: "min(480px, 90vw)", display: "flex", flexDirection: "column" as const, gap: 14, maxHeight: "90vh", overflowY: "auto" as const },
  modalHeader: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  modalTitle: { fontSize: 16, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  closeBtn: { background: "none", border: "none", fontSize: 16, cursor: "pointer", color: "var(--color-text-secondary)", padding: "2px 6px" },
  formRow: { display: "flex", flexDirection: "column" as const, gap: 4 },
  label: { fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)" },
  fieldHint: { fontSize: 10, color: "var(--color-text-disabled)", lineHeight: 1.4 },
  input: { padding: "8px 10px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 13, background: "var(--color-bg-secondary)", color: "var(--color-text)", outline: "none", fontFamily: "inherit" },
  select: { padding: "8px 10px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", fontSize: 13, background: "var(--color-bg-secondary)", color: "var(--color-text)" },
  modalFooter: { display: "flex", gap: 10, justifyContent: "flex-end", marginTop: 8 },
  cancelBtn: { padding: "8px 16px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", color: "var(--color-text-secondary)", fontSize: 13, cursor: "pointer" },
  saveBtn: { padding: "8px 16px", border: "1px solid var(--color-primary)", borderRadius: "var(--radius-md)", background: "var(--color-primary)", color: "#fff", fontSize: 13, cursor: "pointer", fontWeight: 600 },
};
