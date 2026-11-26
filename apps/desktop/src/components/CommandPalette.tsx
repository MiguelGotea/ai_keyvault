/**
 * CommandPalette — Paleta de comandos global Ctrl+K (Fase 23).
 * Permite navegar entre pantallas y ejecutar acciones rápidas sobre cuentas.
 */
import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type { AccountInfo, SwitchResult } from "../types/api";
import es from "../i18n/es.json";

interface Props {
  open: boolean;
  onClose: () => void;
}

interface PaletteCommand {
  id: string;
  label: string;
  category: string;
  run: () => Promise<void> | void;
}

export default function CommandPalette({ open, onClose }: Props) {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [feedback, setFeedback] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setSelectedIdx(0);
    setFeedback(null);
    invoke<AccountInfo[]>("list_vault_accounts")
      .then(setAccounts)
      .catch(e => setFeedback(`Error cargando cuentas: ${String(e)}`));
  }, [open]);

  const commands = useMemo<PaletteCommand[]>(() => {
    const navItems: PaletteCommand[] = [
      { id: "nav-dashboard", label: es.nav.dashboard, category: "Ir a", run: () => { navigate("/dashboard"); onClose(); } },
      { id: "nav-alerts", label: es.nav.alerts, category: "Ir a", run: () => { navigate("/alerts"); onClose(); } },
      { id: "nav-ping", label: es.nav.ping, category: "Ir a", run: () => { navigate("/ping"); onClose(); } },
      { id: "nav-chats", label: es.nav.chats, category: "Ir a", run: () => { navigate("/chats"); onClose(); } },
      { id: "nav-accounts", label: es.nav.accounts, category: "Ir a", run: () => { navigate("/accounts"); onClose(); } },
      { id: "nav-stats", label: es.nav.stats, category: "Ir a", run: () => { navigate("/stats"); onClose(); } },
      { id: "nav-settings", label: es.nav.settings, category: "Ir a", run: () => { navigate("/settings"); onClose(); } },
    ];

    const accountActions: PaletteCommand[] = accounts.flatMap(acc => [
      {
        id: `switch-${acc.id}`,
        label: `⚡ Usar en esta PC: ${acc.alias} (${acc.provider})`,
        category: "Cambiar cuenta",
        run: async () => {
          const res = await invoke<SwitchResult>("switch_account", {
            accountId: acc.id,
            provider: acc.provider,
            deviceId: "desktop-win",
          });
          if (!res.success) {
            setFeedback(`❌ ${res.error ?? "Error al cambiar de cuenta"}`);
            return;
          }
          onClose();
        },
      },
      {
        id: `copy-email-${acc.id}`,
        label: `📋 Copiar correo de ${acc.alias}`,
        category: "Portapapeles",
        run: async () => {
          const email = acc.email ?? await invoke<string>("get_account_email", { accountUuid: acc.id });
          await navigator.clipboard.writeText(email);
          await invoke("schedule_clipboard_clear", { seconds: 30 });
          onClose();
        },
      },
    ]);

    const globalActions: PaletteCommand[] = [
      {
        id: "lock-vault",
        label: `🔒 ${es.vault.bloquear}`,
        category: "Seguridad",
        run: async () => {
          await invoke("lock_vault");
          onClose();
          navigate("/unlock", { replace: true });
        },
      },
      {
        id: "pause-poller",
        label: "⏸ Pausar poller",
        category: "Poller",
        run: async () => {
          await invoke("pause_poller");
          onClose();
        },
      },
      {
        id: "resume-poller",
        label: "▶️ Reanudar poller",
        category: "Poller",
        run: async () => {
          await invoke("resume_poller");
          onClose();
        },
      },
    ];

    return [...accountActions, ...navItems, ...globalActions];
  }, [accounts, navigate, onClose]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands;
    return commands.filter(
      c => c.label.toLowerCase().includes(q) || c.category.toLowerCase().includes(q)
    );
  }, [commands, query]);

  useEffect(() => {
    setSelectedIdx(0);
  }, [query]);

  if (!open) return null;

  async function execute(cmd: PaletteCommand) {
    setFeedback(null);
    try {
      await cmd.run();
    } catch (e) {
      setFeedback(`❌ ${String(e)}`);
    }
  }

  return (
    <div
      style={styles.overlay}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div style={styles.modal}>
        <input
          style={styles.input}
          placeholder="Buscar comando, pantalla o cuenta… (Esc para cerrar)"
          value={query}
          onChange={e => setQuery(e.target.value)}
          onKeyDown={e => {
            if (e.key === "Escape") {
              e.preventDefault();
              onClose();
            } else if (e.key === "ArrowDown") {
              e.preventDefault();
              setSelectedIdx(i => Math.min(i + 1, Math.max(0, filtered.length - 1)));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSelectedIdx(i => Math.max(0, i - 1));
            } else if (e.key === "Enter" && filtered[selectedIdx]) {
              e.preventDefault();
              execute(filtered[selectedIdx]);
            }
          }}
          autoFocus
        />
        {feedback && <div style={styles.feedback}>{feedback}</div>}
        <div style={styles.list}>
          {filtered.length === 0 ? (
            <div style={styles.empty}>Sin coincidencias</div>
          ) : (
            filtered.map((cmd, idx) => (
              <button
                key={cmd.id}
                onClick={() => execute(cmd)}
                style={{
                  ...styles.item,
                  ...(idx === selectedIdx ? styles.itemActive : {}),
                }}
              >
                <span>{cmd.label}</span>
                <span style={styles.category}>{cmd.category}</span>
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  overlay: { position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "flex-start", justifyContent: "center", paddingTop: 90, zIndex: 1200 },
  modal: { width: "min(540px, 92vw)", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-lg)", overflow: "hidden", boxShadow: "0 20px 50px rgba(0,0,0,0.35)" },
  input: { width: "100%", padding: "14px 16px", border: "none", borderBottom: "1px solid var(--color-border)", background: "var(--color-bg)", color: "var(--color-text)", fontSize: 14, outline: "none", boxSizing: "border-box" },
  feedback: { padding: "8px 16px", fontSize: 12, color: "var(--color-danger)", borderBottom: "1px solid var(--color-border)" },
  list: { maxHeight: 340, overflowY: "auto", display: "flex", flexDirection: "column" },
  empty: { padding: 20, textAlign: "center", fontSize: 12, color: "var(--color-text-disabled)" },
  item: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "10px 16px", border: "none", background: "transparent", color: "var(--color-text)", fontSize: 13, cursor: "pointer", textAlign: "left" },
  itemActive: { background: "var(--color-bg-secondary)", fontWeight: 600 },
  category: { fontSize: 11, color: "var(--color-text-disabled)" },
};
