/** Lee si el modo "Sin agente en esta PC" está activo en localStorage. */
export function isNoAgentOnPc(): boolean {
  try {
    const raw = localStorage.getItem("akv_settings");
    if (!raw) return false;
    return Boolean(JSON.parse(raw).noAgentOnPc);
  } catch {
    return false;
  }
}
