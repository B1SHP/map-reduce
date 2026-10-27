use std::io::{self, BufRead, Write};

use map_reduce::{Stats, aggregate, log_read_error, parse_protocol, report};

fn main() {
    write_reduced(io::stdin().lock(), io::stdout().lock())
        .unwrap_or_else(|error| report("Redutor", error));
}

// Une estados globais por chave e produz uma correlação final por alvo.
fn write_reduced<R: BufRead, W: Write>(reader: R, mut writer: W) -> io::Result<()> {
    let totals = aggregate(
        reader
            .lines()
            .filter_map(log_read_error)
            .filter_map(|line| parse_protocol(&line)),
    );
    let temperature_control = totals.get("temp").copied();

    totals
        .into_iter()
        .filter(|(key, _)| key != "temp")
        .try_for_each(|(key, stats)| writeln!(writer, "{}", render(&key, stats)))?;
    writer.flush()?;

    if let Some(stats) = temperature_control {
        log_temperature_control(stats);
    }
    Ok(())
}

// Registra autocorrelação somente como controle, fora da saída final Hadoop.
fn log_temperature_control(stats: Stats) {
    match stats.correlation() {
        Some(correlation) => eprintln!(
            "Controle técnico — temp × temp: r={correlation:.6}; n={}. Esperado: r=1,0 quando a temperatura possui variação.",
            stats.count
        ),
        None => eprintln!(
            "Controle técnico — temp × temp: indefinida; n={}. A temperatura não possui variação.",
            stats.count
        ),
    }
}

// Formata uma linha final, incluindo valor e interpretação do coeficiente.
fn render(key: &str, stats: Stats) -> String {
    match stats.correlation() {
        Some(correlation) => format!(
            "{key}\tr={correlation:.6}\tn={}\t{}",
            stats.count,
            classify(correlation)
        ),
        None => format!("{key}\tr=indefinida\tn={}", stats.count),
    }
}

// Traduz módulo e sinal de Pearson em categoria legível para defesa.
fn classify(correlation: f64) -> &'static str {
    if correlation == 0.0 {
        return "sem relacao linear";
    }
    match (correlation.abs(), correlation.is_sign_positive()) {
        (value, true) if value < 0.20 => "muito fraca positiva",
        (value, false) if value < 0.20 => "muito fraca negativa",
        (value, true) if value < 0.40 => "fraca positiva",
        (value, false) if value < 0.40 => "fraca negativa",
        (value, true) if value < 0.60 => "moderada positiva",
        (value, false) if value < 0.60 => "moderada negativa",
        (value, true) if value < 0.80 => "forte positiva",
        (value, false) if value < 0.80 => "forte negativa",
        (_, true) => "muito forte positiva",
        (_, false) => "muito forte negativa",
    }
}
