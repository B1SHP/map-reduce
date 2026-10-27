use std::{collections::BTreeMap, io};

// Estado agregável de Pearson, compartilhado entre as três etapas MapReduce.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    // Quantidade de pares válidos.
    pub count: u64,
    // Média da temperatura.
    pub mean_x: f64,
    // Média da variável-alvo.
    pub mean_y: f64,
    // Soma estável dos desvios quadrados da temperatura.
    pub m2_x: f64,
    // Soma estável dos desvios quadrados da variável-alvo.
    pub m2_y: f64,
    // Soma estável dos desvios cruzados.
    pub co_moment: f64,
}

impl Stats {
    // Cria estado inicial com somente um par.
    pub fn single(x: f64, y: f64) -> Self {
        Self {
            count: 1,
            mean_x: x,
            mean_y: y,
            ..Self::default()
        }
    }

    // Une estados sem calcular correlação parcial.
    pub fn merge(self, other: Self) -> Self {
        if self.count == 0 {
            return other;
        }
        if other.count == 0 {
            return self;
        }

        let count = self.count + other.count;
        let left_count = self.count as f64;
        let right_count = other.count as f64;
        let total_count = count as f64;
        let delta_x = other.mean_x - self.mean_x;
        let delta_y = other.mean_y - self.mean_y;
        let adjustment = left_count * right_count / total_count;

        Self {
            count,
            mean_x: self.mean_x + delta_x * right_count / total_count,
            mean_y: self.mean_y + delta_y * right_count / total_count,
            m2_x: self.m2_x + other.m2_x + delta_x * delta_x * adjustment,
            m2_y: self.m2_y + other.m2_y + delta_y * delta_y * adjustment,
            co_moment: self.co_moment + other.co_moment + delta_x * delta_y * adjustment,
        }
    }

    // Calcula Pearson depois de união global de todos estados.
    pub fn correlation(self) -> Option<f64> {
        let variance_x = non_negative(self.m2_x)?;
        let variance_y = non_negative(self.m2_y)?;
        let denominator = (variance_x * variance_y).sqrt();
        if self.count < 2 || !denominator.is_finite() || denominator == 0.0 {
            return None;
        }

        let result = self.co_moment / denominator;
        (result.is_finite() && result.abs() <= 1.0 + 1e-12).then(|| result.clamp(-1.0, 1.0))
    }

    // Serializa estado para stdout do protocolo Hadoop.
    pub fn encode(self) -> String {
        format!(
            "{},{},{},{},{},{}",
            self.count, self.mean_x, self.mean_y, self.m2_x, self.m2_y, self.co_moment
        )
    }

    // Lê estado vindo do stdout da etapa anterior.
    pub fn decode(value: &str) -> Option<Self> {
        let mut fields = value.split(',');
        let count = fields.next()?.parse::<u64>().ok()?;
        let mean_x = parse_finite(fields.next()?)?;
        let mean_y = parse_finite(fields.next()?)?;
        let m2_x = parse_finite(fields.next()?)?;
        let m2_y = parse_finite(fields.next()?)?;
        let co_moment = parse_finite(fields.next()?)?;
        fields.next().is_none().then_some(Self {
            count,
            mean_x,
            mean_y,
            m2_x,
            m2_y,
            co_moment,
        })
    }
}

// Agrupa cada chave e une seus estados em um único total.
pub fn aggregate<I>(pairs: I) -> BTreeMap<String, Stats>
where
    I: IntoIterator<Item = (String, Stats)>,
{
    pairs
        .into_iter()
        .fold(BTreeMap::new(), |mut totals, (key, stats)| {
            totals
                .entry(key)
                .and_modify(|total| *total = total.merge(stats))
                .or_insert(stats);
            totals
        })
}

// Divide linha `chave<TAB>estado` e valida conteúdo.
pub fn parse_protocol(line: &str) -> Option<(String, Stats)> {
    let (key, value) = line.split_once('\t')?;
    let stats = Stats::decode(value)?;
    (!key.is_empty()).then_some((key.to_owned(), stats))
}

// Aceita somente ponto flutuante finito.
fn parse_finite(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

// Corrige negativo minúsculo causado por arredondamento de ponto flutuante.
fn non_negative(value: f64) -> Option<f64> {
    (value >= 0.0)
        .then_some(value)
        .or_else(|| (value.abs() <= f64::EPSILON * 64.0).then_some(0.0))
}

// Registra falha de leitura e remove linha inválida de um iterator.
pub fn log_read_error(result: io::Result<String>) -> Option<String> {
    result
        .inspect_err(|error| eprintln!("Ignoring unreadable input line: {error}"))
        .ok()
}

// Relata falha terminal de qualquer binário e encerra processo com erro.
pub fn report(program: &str, error: io::Error) -> ! {
    eprintln!("{program} failed: {error}");
    std::process::exit(1);
}
