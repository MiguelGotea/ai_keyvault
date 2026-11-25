//! Cálculo de estadísticas de cuota: burn rate, proyección, mapa de calor,
//! comparativa entre cuentas, informe semanal y exportación CSV/JSON.
//!
//! Módulo puro sin dependencias de red para facilitar tests unitarios.

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/// Lectura cruda de BD usada para cálculos estadísticos.
#[derive(Debug, Clone, Serialize)]
pub struct ReadingSample {
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub tokens_used: i64,
    pub tokens_limit: i64,
    pub percent_used: f64,
    pub reset_time: Option<String>,
    pub source: String,
    pub read_at: String,
}

/// Resumen y proyección por pool para una cuenta.
#[derive(Debug, Clone, Serialize)]
pub struct PoolSummary {
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub current_percent_used: f64,
    pub tokens_used: i64,
    pub tokens_limit: i64,
    pub reset_time: Option<String>,
    pub last_read_at: String,
    /// Porcentaje consumido por hora calculado sobre deltas positivos recientes.
    pub burn_rate_pct_per_hour: f64,
    /// Horas estimadas para llegar al 100% al ritmo actual (None si burn_rate == 0).
    pub estimated_hours_to_100: Option<f64>,
    /// Timestamp RFC 3339 estimado de agotamiento (None si burn_rate == 0).
    pub estimated_exhaustion_at: Option<String>,
    /// Cantidad de muestras consideradas en el cálculo.
    pub sample_count: usize,
}

/// Celda del mapa de calor por día de semana y hora UTC.
#[derive(Debug, Clone, Serialize)]
pub struct HeatmapCell {
    /// Día de la semana (0 = Lun .. 6 = Dom).
    pub weekday: u32,
    /// Nombre corto del día ("Lun", "Mar", ...).
    pub weekday_name: &'static str,
    /// Hora UTC (0..=23).
    pub hour: u32,
    /// Suma de consumo porcentual positivo en esa franja.
    pub delta_percent: f64,
    /// Número de lecturas registradas en esa franja.
    pub samples: usize,
}

/// Resumen diario para el informe semanal.
#[derive(Debug, Clone, Serialize)]
pub struct DailyReportRow {
    pub date: String,
    pub pool: String,
    pub min_percent: f64,
    pub max_percent: f64,
    pub delta_consumed: f64,
    pub readings_count: usize,
}

/// Informe semanal completo de una cuenta.
#[derive(Debug, Clone, Serialize)]
pub struct WeeklyReport {
    pub account_id: String,
    pub generated_at: String,
    pub total_readings: usize,
    pub pools: Vec<PoolSummary>,
    pub daily: Vec<DailyReportRow>,
}

fn parse_ts(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn weekday_short(idx: u32) -> &'static str {
    match idx {
        0 => "Lun",
        1 => "Mar",
        2 => "Mié",
        3 => "Jue",
        4 => "Vie",
        5 => "Sáb",
        _ => "Dom",
    }
}

/// Calcula el burn rate (% / hora) a partir de muestras ordenadas cronológicamente (ascendente).
///
/// Ignora caídas de porcentaje (resets de cuota) sumando únicamente los deltas
/// positivos consecutivos y dividiendo entre el tiempo total transcurrido en esos
/// tramos válidos.
pub fn compute_burn_rate(ordered_asc: &[&ReadingSample]) -> f64 {
    if ordered_asc.len() < 2 {
        return 0.0;
    }

    let mut total_delta_pct = 0.0_f64;
    let mut total_hours = 0.0_f64;

    for pair in ordered_asc.windows(2) {
        let prev = pair[0];
        let curr = pair[1];

        let (Some(t1), Some(t2)) = (parse_ts(&prev.read_at), parse_ts(&curr.read_at)) else {
            continue;
        };

        let secs = (t2 - t1).num_seconds();
        if secs <= 0 {
            continue;
        }

        let delta = curr.percent_used - prev.percent_used;
        // Si delta < 0 hubo un reset de cuota; no lo contamos como consumo negativo.
        if delta >= 0.0 {
            total_delta_pct += delta;
            total_hours += secs as f64 / 3600.0;
        }
    }

    if total_hours <= 0.0 {
        0.0
    } else {
        (total_delta_pct / total_hours * 100.0).round() / 100.0
    }
}

/// Calcula el resumen por `(account_id, provider, pool)` a partir de una lista de lecturas.
pub fn summarize_pools(samples: &[ReadingSample]) -> Vec<PoolSummary> {
    let mut groups: BTreeMap<(String, String, String), Vec<&ReadingSample>> = BTreeMap::new();
    for s in samples {
        groups
            .entry((s.account_id.clone(), s.provider.clone(), s.pool.clone()))
            .or_default()
            .push(s);
    }

    let mut summaries = Vec::with_capacity(groups.len());

    for ((account_id, provider, pool), mut group) in groups {
        group.sort_by(|a, b| a.read_at.cmp(&b.read_at));

        let latest = match group.last() {
            Some(l) => l,
            None => continue,
        };

        let burn_rate = compute_burn_rate(&group);
        let remaining_pct = (100.0 - latest.current_or_percent()).max(0.0);

        let (hours_to_100, exhaustion_at) = if burn_rate > 0.0 && remaining_pct > 0.0 {
            let hrs = (remaining_pct / burn_rate * 100.0).round() / 100.0;
            let ex = parse_ts(&latest.read_at)
                .map(|t| (t + chrono::Duration::seconds((hrs * 3600.0) as i64)).to_rfc3339());
            (Some(hrs), ex)
        } else if remaining_pct == 0.0 {
            (Some(0.0), Some(latest.read_at.clone()))
        } else {
            (None, None)
        };

        summaries.push(PoolSummary {
            account_id,
            provider,
            pool,
            current_percent_used: latest.percent_used,
            tokens_used: latest.tokens_used,
            tokens_limit: latest.tokens_limit,
            reset_time: latest.reset_time.clone(),
            last_read_at: latest.read_at.clone(),
            burn_rate_pct_per_hour: burn_rate,
            estimated_hours_to_100: hours_to_100,
            estimated_exhaustion_at: exhaustion_at,
            sample_count: group.len(),
        });
    }

    summaries.sort_by(|a, b| {
        b.current_percent_used
            .partial_cmp(&a.current_percent_used)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    summaries
}

trait PercentExt {
    fn current_or_percent(&self) -> f64;
}

impl PercentExt for ReadingSample {
    fn current_or_percent(&self) -> f64 {
        self.percent_used
    }
}

/// Construye el mapa de calor por `(weekday, hour)` UTC agrupando por pool
/// para calcular deltas positivos de consumo en cada hora.
pub fn build_heatmap(samples: &[ReadingSample]) -> Vec<HeatmapCell> {
    let mut by_pool: HashMap<&str, Vec<&ReadingSample>> = HashMap::new();
    for s in samples {
        by_pool.entry(s.pool.as_str()).or_default().push(s);
    }

    let mut grid: BTreeMap<(u32, u32), (f64, usize)> = BTreeMap::new();

    for (_pool, mut group) in by_pool {
        group.sort_by(|a, b| a.read_at.cmp(&b.read_at));

        for (idx, curr) in group.iter().enumerate() {
            let Some(dt) = parse_ts(&curr.read_at) else {
                continue;
            };
            let wd = dt.weekday().num_days_from_monday();
            let hr = dt.hour();

            let delta = if idx > 0 {
                let d = curr.percent_used - group[idx - 1].percent_used;
                if d > 0.0 {
                    d
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let entry = grid.entry((wd, hr)).or_insert((0.0, 0));
            entry.0 += delta;
            entry.1 += 1;
        }
    }

    grid.into_iter()
        .map(|((wd, hr), (delta, count))| HeatmapCell {
            weekday: wd,
            weekday_name: weekday_short(wd),
            hour: hr,
            delta_percent: (delta * 100.0).round() / 100.0,
            samples: count,
        })
        .collect()
}

/// Genera el informe semanal agrupado por `(date, pool)`.
pub fn build_weekly_report(account_id: &str, samples: &[ReadingSample]) -> WeeklyReport {
    let pools = summarize_pools(samples);

    let mut by_day_pool: BTreeMap<(NaiveDate, String), Vec<&ReadingSample>> = BTreeMap::new();
    for s in samples {
        if let Some(dt) = parse_ts(&s.read_at) {
            by_day_pool
                .entry((dt.date_naive(), s.pool.clone()))
                .or_default()
                .push(s);
        }
    }

    let mut daily = Vec::with_capacity(by_day_pool.len());
    for ((date, pool), mut group) in by_day_pool {
        group.sort_by(|a, b| a.read_at.cmp(&b.read_at));

        let min_p = group
            .iter()
            .map(|s| s.percent_used)
            .fold(f64::INFINITY, f64::min);
        let max_p = group
            .iter()
            .map(|s| s.percent_used)
            .fold(f64::NEG_INFINITY, f64::max);

        let mut delta_consumed = 0.0_f64;
        for pair in group.windows(2) {
            let d = pair[1].percent_used - pair[0].percent_used;
            if d > 0.0 {
                delta_consumed += d;
            }
        }

        daily.push(DailyReportRow {
            date: date.to_string(),
            pool,
            min_percent: (min_p * 100.0).round() / 100.0,
            max_percent: (max_p * 100.0).round() / 100.0,
            delta_consumed: (delta_consumed * 100.0).round() / 100.0,
            readings_count: group.len(),
        });
    }

    WeeklyReport {
        account_id: account_id.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        total_readings: samples.len(),
        pools,
        daily,
    }
}

/// Serializa una lista de lecturas a CSV estándar con cabecera.
pub fn to_csv(samples: &[ReadingSample]) -> String {
    let mut out = String::from(
        "account_id,provider,pool,tokens_used,tokens_limit,percent_used,reset_time,source,read_at\n",
    );

    for s in samples {
        let reset = s.reset_time.as_deref().unwrap_or("");
        out.push_str(&format!(
            "{},{},{},{},{},{:.2},{},{},{}\n",
            escape_csv(&s.account_id),
            escape_csv(&s.provider),
            escape_csv(&s.pool),
            s.tokens_used,
            s.tokens_limit,
            s.percent_used,
            escape_csv(reset),
            escape_csv(&s.source),
            escape_csv(&s.read_at),
        ));
    }

    out
}

fn escape_csv(val: &str) -> String {
    if val.contains(',') || val.contains('"') || val.contains('\n') {
        format!("\"{}\"", val.replace('"', "\"\""))
    } else {
        val.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(pool: &str, pct: f64, ts: &str) -> ReadingSample {
        ReadingSample {
            account_id: "test@example.com".to_string(),
            provider: "agy".to_string(),
            pool: pool.to_string(),
            tokens_used: (pct * 10000.0) as i64,
            tokens_limit: 1_000_000,
            percent_used: pct,
            reset_time: None,
            source: "PC-TEST".to_string(),
            read_at: ts.to_string(),
        }
    }

    #[test]
    fn test_burn_rate_simple_y_proyeccion() {
        let samples = vec![
            sample("claude-gpt-weekly", 20.0, "2026-11-25T00:00:00Z"),
            sample("claude-gpt-weekly", 30.0, "2026-11-25T02:00:00Z"),
            sample("claude-gpt-weekly", 40.0, "2026-11-25T04:00:00Z"),
        ];

        let summaries = summarize_pools(&samples);
        assert_eq!(summaries.len(), 1);
        let s = &summaries[0];
        // 20% en 4h = 5.0% por hora
        assert_eq!(s.burn_rate_pct_per_hour, 5.0);
        // Faltan 60% a 5%/h = 12.0h
        assert_eq!(s.estimated_hours_to_100, Some(12.0));
        assert!(s.estimated_exhaustion_at.is_some());
    }

    #[test]
    fn test_burn_rate_ignora_reset_de_cuota() {
        let samples = vec![
            sample("claude-gpt-5h", 80.0, "2026-11-25T00:00:00Z"),
            sample("claude-gpt-5h", 90.0, "2026-11-25T01:00:00Z"),
            // Reset aquí (baja de 90 a 5)
            sample("claude-gpt-5h", 5.0, "2026-11-25T02:00:00Z"),
            sample("claude-gpt-5h", 15.0, "2026-11-25T03:00:00Z"),
        ];

        let summaries = summarize_pools(&samples);
        // Dos tramos válidos: +10% en 1h y +10% en 1h => 10%/h
        assert_eq!(summaries[0].burn_rate_pct_per_hour, 10.0);
    }

    #[test]
    fn test_heatmap_agrupa_por_dia_y_hora() {
        let samples = vec![
            sample("gemini-weekly", 10.0, "2026-11-25T10:00:00Z"),
            sample("gemini-weekly", 18.0, "2026-11-25T10:30:00Z"),
        ];

        let cells = build_heatmap(&samples);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].hour, 10);
        assert_eq!(cells[0].delta_percent, 8.0);
        assert_eq!(cells[0].samples, 2);
    }

    #[test]
    fn test_weekly_report_y_csv() {
        let samples = vec![
            sample("gemini-weekly", 10.0, "2026-11-24T10:00:00Z"),
            sample("gemini-weekly", 25.0, "2026-11-24T12:00:00Z"),
        ];

        let report = build_weekly_report("test@example.com", &samples);
        assert_eq!(report.total_readings, 2);
        assert_eq!(report.daily.len(), 1);
        assert_eq!(report.daily[0].delta_consumed, 15.0);

        let csv = to_csv(&samples);
        assert!(csv.starts_with("account_id,provider,pool,"));
        assert!(csv.contains("test@example.com,agy,gemini-weekly,"));
    }
}
