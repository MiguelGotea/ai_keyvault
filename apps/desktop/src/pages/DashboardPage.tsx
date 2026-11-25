/** DashboardPage — Fase 10: placeholder mínimo.
 *  El dashboard real con tarjetas de cuota se implementa en Fase 13.
 */
import es from "../i18n/es.json";

export default function DashboardPage() {
  return (
    <div style={styles.container}>
      <h1 style={styles.title}>{es.nav.dashboard}</h1>
      <p style={styles.subtitle}>
        ⚙️ Fase 10 — Scaffold. Las tarjetas de cuentas y cuota se implementan en Fase 13.
      </p>
      <div style={styles.card}>
        <span style={styles.icon}>🔐</span>
        <p>El backend está operativo en <code>198.211.97.243:3010</code></p>
        <p style={{ color: "var(--color-text-secondary)", fontSize: 13 }}>
          Próximos pasos: Fase 11 (capa HTTP Rust) → Fase 12 (desbloqueo bóveda)
          → Fase 13 (dashboard real)
        </p>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: 32,
    display: "flex",
    flexDirection: "column",
    gap: 24,
  },
  title: {
    fontSize: 28,
    fontWeight: 700,
    color: "var(--color-primary)",
  },
  subtitle: {
    fontSize: 14,
    color: "var(--color-text-secondary)",
  },
  card: {
    background: "var(--color-bg-secondary)",
    border: "1px solid var(--color-border)",
    borderRadius: "var(--radius-lg)",
    padding: 24,
    display: "flex",
    flexDirection: "column",
    gap: 8,
    maxWidth: 500,
  },
  icon: { fontSize: 32 },
};
