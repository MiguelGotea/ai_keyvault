//! Notificador de alertas.
//!
//! Envía notificaciones via:
//! 1. **Windows toast** — popup de sistema operativo (solo Windows).
//! 2. **ntfy.sh** — HTTP POST al tópico configurado (opcional).

use crate::models::AlertEvent;
use tracing::debug;
#[cfg(windows)]
use tracing::warn;

pub struct Notifier {
    ntfy_topic: Option<String>,
    ntfy_url: String,
    toast_enabled: bool,
}

impl Notifier {
    pub fn new(ntfy_topic: Option<String>, toast_enabled: bool) -> Self {
        Self {
            ntfy_topic,
            ntfy_url: std::env::var("AKYV_NTFY_URL")
                .unwrap_or_else(|_| "https://ntfy.sh".to_string()),
            toast_enabled,
        }
    }

    /// Envía todas las notificaciones configuradas para este evento.
    pub fn send(&self, event: &AlertEvent) -> Result<(), NotifyError> {
        // 1. Windows toast (solo compila en Windows)
        if self.toast_enabled {
            self.send_toast(event);
        }

        // 2. ntfy.sh
        if let Some(ref topic) = self.ntfy_topic {
            self.send_ntfy(event, topic)?;
        }

        Ok(())
    }

    /// Notificación toast de Windows via PowerShell (System.Windows.Forms).
    ///
    /// No depende de winrt-notification — usa solo APIs de .NET Framework
    /// disponibles en todas las versiones de Windows desde 7.
    #[cfg(windows)]
    fn send_toast(&self, event: &AlertEvent) {
        let title = event.title().replace('\'', "''");
        let body = event.body().replace('\'', "''");
        // Escapa las comillas simples para PowerShell
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $n = New-Object System.Windows.Forms.NotifyIcon; \
             $n.Icon = [System.Drawing.SystemIcons]::Warning; \
             $n.Visible = $true; \
             $n.ShowBalloonTip(8000, '{title}', '{body}', \
             [System.Windows.Forms.ToolTipIcon]::Warning); \
             Start-Sleep -Milliseconds 500; \
             $n.Dispose()",
            title = title,
            body = body
        );

        let result = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output();

        match result {
            Ok(out) if out.status.success() => debug!("🔔 Toast enviado"),
            Ok(out) => warn!(
                "Toast falló: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(e) => warn!("No se pudo lanzar PowerShell para toast: {e}"),
        }
    }

    /// No-op en plataformas que no son Windows.
    #[cfg(not(windows))]
    fn send_toast(&self, _event: &AlertEvent) {
        debug!("Toast no soportado en esta plataforma");
    }

    /// Envía a ntfy.sh via HTTP POST bloqueante.
    fn send_ntfy(&self, event: &AlertEvent, topic: &str) -> Result<(), NotifyError> {
        let url = format!("{}/{}", self.ntfy_url.trim_end_matches('/'), topic);

        // Priority: 4=high para WARN, 5=urgent para CRITICAL
        let priority = match event.level {
            crate::models::AlertLevel::Warn => "high",
            crate::models::AlertLevel::Critical => "urgent",
        };

        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(&url)
            .header("Title", event.title())
            .header("Priority", priority)
            .header("Tags", "warning,ai-keyvault")
            .body(event.body())
            .send()
            .map_err(|e| NotifyError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(NotifyError::Http(format!(
                "ntfy.sh respondió {}",
                resp.status()
            )));
        }

        debug!(topic = %topic, "📱 Notificación ntfy enviada");
        Ok(())
    }
}

#[derive(Debug)]
pub enum NotifyError {
    Http(String),
}

impl std::fmt::Display for NotifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotifyError::Http(e) => write!(f, "HTTP error: {e}"),
        }
    }
}
