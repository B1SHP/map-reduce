use std::io::{self, BufRead, Write};

use map_reduce::{aggregate, log_read_error, parse_protocol, report};

fn main() {
    write_combined(io::stdin().lock(), io::stdout().lock())
        .unwrap_or_else(|error| report("Combinador", error));
}

// Agrupa estados locais, reduzindo volume transferido para reducer.
fn write_combined<R: BufRead, W: Write>(reader: R, mut writer: W) -> io::Result<()> {
    aggregate(
        reader
            .lines()
            .filter_map(log_read_error)
            .filter_map(|line| parse_protocol(&line)),
    )
    .into_iter()
    .try_for_each(|(key, stats)| writeln!(writer, "{key}\t{}", stats.encode()))
}
