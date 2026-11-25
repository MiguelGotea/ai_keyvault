/** ChatsPage — Fase 15: Lista de chats activos del VPS. */
import { useState } from "react";
import { useCommand } from "../hooks/useCommand";
import type { ChatSession } from "../types/api";
import es from "../i18n/es.json";

const ACCOUNTS = ["agy:test@agy", "codex:test@codex"]; // placeholder — Fase 15 usa vault

export default function ChatsPage() {
  const [selectedAccount, setSelectedAccount] = useState(ACCOUNTS[0]);
  const [provider, accountId] = selectedAccount.split(":");

  const chats = useCommand<ChatSession[]>("get_chats", { accountId }, { enabled: !!accountId });

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <h2 style={styles.title}>{es.chats.titulo}</h2>
        <select
          style={styles.select}
          value={selectedAccount}
          onChange={e => setSelectedAccount(e.target.value)}
        >
          {ACCOUNTS.map(a => <option key={a} value={a}>{a}</option>)}
        </select>
      </div>
      <p style={styles.subtitle}>{es.chats.subtitulo}</p>

      {chats.loading && <div style={styles.hint}>Cargando chats…</div>}
      {chats.error && <div style={styles.error}>{chats.error}</div>}

      {chats.data?.length === 0 && (
        <div style={styles.empty}>
          <span style={{ fontSize: 36 }}>💬</span>
          <p>{es.chats.sin_chats}</p>
        </div>
      )}

      {chats.data && chats.data.length > 0 && (
        <div style={styles.list}>
          {chats.data.map(chat => (
            <div key={chat.id} style={styles.chatRow}>
              <div>
                <div style={styles.chatTitle}>{chat.title ?? "(Sin título)"}</div>
                <div style={styles.chatMeta}>
                  {es.chats.ultima_actividad}: {new Date(chat.updated_at).toLocaleString("es-MX")}
                  {" · "}{provider}
                  {chat.archived && <span style={styles.archived}> · Archivado</span>}
                </div>
              </div>
              <span style={styles.chatId}>{chat.id.slice(0, 8)}…</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: { padding: 24, display: "flex", flexDirection: "column", gap: 16 },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  title: { fontSize: 20, fontWeight: 700, color: "var(--color-text)", margin: 0 },
  subtitle: { fontSize: 13, color: "var(--color-text-secondary)", margin: 0 },
  select: { padding: "6px 10px", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)", background: "var(--color-bg)", fontSize: 13, color: "var(--color-text)" },
  hint: { color: "var(--color-text-secondary)", fontSize: 13 },
  error: { padding: "10px 14px", background: "#FEF2F2", border: "1px solid #FECACA", borderRadius: "var(--radius-md)", fontSize: 12, color: "#991B1B" },
  empty: { display: "flex", flexDirection: "column", alignItems: "center", gap: 12, padding: 48, color: "var(--color-text-secondary)", textAlign: "center" },
  list: { display: "flex", flexDirection: "column", gap: 8 },
  chatRow: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "12px 16px", background: "var(--color-bg)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)" },
  chatTitle: { fontSize: 13, color: "var(--color-text)", fontWeight: 500 },
  chatMeta: { fontSize: 11, color: "var(--color-text-disabled)", marginTop: 3 },
  chatId: { fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--color-text-disabled)" },
  archived: { color: "var(--color-text-disabled)" },
};
