use std::io::{self, BufRead, Write};

use map_reduce::{Stats, log_read_error, report};

#[derive(Clone, Copy)]
// Campo do CSV Forest Fires; variante guarda posição e nome sem números/strings soltos.
enum Column {
    // Campo 1: coordenada X.
    X,
    // Campo 2: coordenada Y.
    Y,
    // Campo 3: mês categórico.
    Month,
    // Campo 4: dia categórico.
    Day,
    // Campo 5: Fine Fuel Moisture Code.
    Ffmc,
    // Campo 6: Duff Moisture Code.
    Dmc,
    // Campo 7: Drought Code.
    Dc,
    // Campo 8: Initial Spread Index.
    Isi,
    // Campo 9: temperatura, variável principal.
    Temp,
    // Campo 10: umidade relativa.
    Rh,
    // Campo 11: vento.
    Wind,
    // Campo 12: chuva.
    Rain,
    // Campo 13: área queimada.
    Area,
}

impl Column {
    // Campos numéricos correlacionados diretamente com temperatura.
    const NUMERIC: [Self; 11] = [
        Self::X,
        Self::Y,
        Self::Ffmc,
        Self::Dmc,
        Self::Dc,
        Self::Isi,
        Self::Temp,
        Self::Rh,
        Self::Wind,
        Self::Rain,
        Self::Area,
    ];

    // Retorna índice zero-based de cada campo no vetor após separar CSV.
    const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Month => 2,
            Self::Day => 3,
            Self::Ffmc => 4,
            Self::Dmc => 5,
            Self::Dc => 6,
            Self::Isi => 7,
            Self::Temp => 8,
            Self::Rh => 9,
            Self::Wind => 10,
            Self::Rain => 11,
            Self::Area => 12,
        }
    }

    // Retorna chave textual estável emitida pelo mapper.
    const fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Month => "month",
            Self::Day => "day",
            Self::Ffmc => "ffmc",
            Self::Dmc => "dmc",
            Self::Dc => "dc",
            Self::Isi => "isi",
            Self::Temp => "temp",
            Self::Rh => "rh",
            Self::Wind => "wind",
            Self::Rain => "rain",
            Self::Area => "area",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
// Valores permitidos no campo categórico de mês.
enum Month {
    // Janeiro.
    Jan,
    // Fevereiro.
    Feb,
    // Março.
    Mar,
    // Abril.
    Apr,
    // Maio.
    May,
    // Junho.
    Jun,
    // Julho.
    Jul,
    // Agosto.
    Aug,
    // Setembro.
    Sep,
    // Outubro.
    Oct,
    // Novembro.
    Nov,
    // Dezembro.
    Dec,
}

impl Month {
    // Todos meses necessários para emitir 12 indicadores binários.
    const ALL: [Self; 12] = [
        Self::Jan,
        Self::Feb,
        Self::Mar,
        Self::Apr,
        Self::May,
        Self::Jun,
        Self::Jul,
        Self::Aug,
        Self::Sep,
        Self::Oct,
        Self::Nov,
        Self::Dec,
    ];

    // Converte valor textual de entrada para variante tipada.
    fn parse(value: &str) -> Option<Self> {
        match value {
            "jan" => Some(Self::Jan),
            "feb" => Some(Self::Feb),
            "mar" => Some(Self::Mar),
            "apr" => Some(Self::Apr),
            "may" => Some(Self::May),
            "jun" => Some(Self::Jun),
            "jul" => Some(Self::Jul),
            "aug" => Some(Self::Aug),
            "sep" => Some(Self::Sep),
            "oct" => Some(Self::Oct),
            "nov" => Some(Self::Nov),
            "dec" => Some(Self::Dec),
            _ => None,
        }
    }

    // Retorna sufixo de chave, como `month_jan`.
    const fn name(self) -> &'static str {
        match self {
            Self::Jan => "jan",
            Self::Feb => "feb",
            Self::Mar => "mar",
            Self::Apr => "apr",
            Self::May => "may",
            Self::Jun => "jun",
            Self::Jul => "jul",
            Self::Aug => "aug",
            Self::Sep => "sep",
            Self::Oct => "oct",
            Self::Nov => "nov",
            Self::Dec => "dec",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
// Valores permitidos no campo categórico de dia da semana.
enum Day {
    // Segunda-feira.
    Mon,
    // Terça-feira.
    Tue,
    // Quarta-feira.
    Wed,
    // Quinta-feira.
    Thu,
    // Sexta-feira.
    Fri,
    // Sábado.
    Sat,
    // Domingo.
    Sun,
}

impl Day {
    // Todos dias necessários para emitir 7 indicadores binários.
    const ALL: [Self; 7] = [
        Self::Mon,
        Self::Tue,
        Self::Wed,
        Self::Thu,
        Self::Fri,
        Self::Sat,
        Self::Sun,
    ];

    // Converte valor textual de entrada para variante tipada.
    fn parse(value: &str) -> Option<Self> {
        match value {
            "mon" => Some(Self::Mon),
            "tue" => Some(Self::Tue),
            "wed" => Some(Self::Wed),
            "thu" => Some(Self::Thu),
            "fri" => Some(Self::Fri),
            "sat" => Some(Self::Sat),
            "sun" => Some(Self::Sun),
            _ => None,
        }
    }

    // Retorna sufixo de chave, como `day_mon`.
    const fn name(self) -> &'static str {
        match self {
            Self::Mon => "mon",
            Self::Tue => "tue",
            Self::Wed => "wed",
            Self::Thu => "thu",
            Self::Fri => "fri",
            Self::Sat => "sat",
            Self::Sun => "sun",
        }
    }
}

fn main() {
    write_mapped(io::stdin().lock(), io::stdout().lock())
        .unwrap_or_else(|error| report("Mapeador", error));
}

// Lê CSV, cria pares para cada alvo e escreve protocolo MapReduce.
fn write_mapped<R: BufRead, W: Write>(reader: R, mut writer: W) -> io::Result<()> {
    reader
        .lines()
        .filter_map(log_read_error)
        .filter_map(|line| {
            map_record(&line).or_else(|| {
                eprintln!("Ignorando registro CSV inválido");
                None
            })
        })
        .flat_map(Vec::into_iter)
        .try_for_each(|(key, stats)| writeln!(writer, "{key}\t{}", stats.encode()))
}

// Converte um registro CSV em pares `(chave, estado)` para cada correlação.
fn map_record(line: &str) -> Option<Vec<(String, Stats)>> {
    let fields: Vec<&str> = line.trim().split(',').collect();
    let temperature = parse_finite(*fields.get(Column::Temp.index())?)?;
    let month = Month::parse(*fields.get(Column::Month.index())?)?;
    let day = Day::parse(*fields.get(Column::Day.index())?)?;
    let numeric = Column::NUMERIC.into_iter().filter_map(|column| {
        parse_finite(*fields.get(column.index())?)
            .map(|value| (column.name().to_owned(), Stats::single(temperature, value)))
    });
    let months = Month::ALL.into_iter().map(|candidate| {
        (
            format!("{}_{}", Column::Month.name(), candidate.name()),
            Stats::single(temperature, f64::from(candidate == month)),
        )
    });
    let days = Day::ALL.into_iter().map(|candidate| {
        (
            format!("{}_{}", Column::Day.name(), candidate.name()),
            Stats::single(temperature, f64::from(candidate == day)),
        )
    });
    Some(numeric.chain(months).chain(days).collect())
}

// Converte texto em número aceitando apenas valores finitos.
fn parse_finite(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}
