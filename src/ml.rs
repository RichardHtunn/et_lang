use crate::parser::ModelSpec;
use std::{collections::HashMap, fs, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    pub threshold: u16,
    pub one_when_high: bool,
    pub correct: usize,
    pub total: usize,
}

pub fn load_models(
    specs: &[ModelSpec],
    source_dir: &Path,
) -> Result<HashMap<String, Model>, String> {
    let mut models = HashMap::new();
    for spec in specs {
        if models.contains_key(&spec.name) {
            return Err(format!("line {}: duplicate model {}", spec.line, spec.name));
        }
        let path = source_dir.join(&spec.path);
        let csv = fs::read_to_string(&path)
            .map_err(|e| format!("line {}: {}: {e}", spec.line, path.display()))?;
        let model =
            train(&csv).map_err(|e| format!("line {}: {}: {e}", spec.line, path.display()))?;
        models.insert(spec.name.clone(), model);
    }
    Ok(models)
}

pub fn train(csv: &str) -> Result<Model, String> {
    let mut samples = Vec::new();
    let mut header_seen = false;
    for (index, raw) in csv.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let columns: Vec<_> = line.split(',').map(str::trim).collect();
        if !header_seen {
            if columns != ["value", "label"] {
                return Err("first CSV row must be value,label".into());
            }
            header_seen = true;
            continue;
        }
        if columns.len() != 2 {
            return Err(format!("CSV line {}: expected value,label", index + 1));
        }
        let value: u16 = columns[0]
            .parse()
            .map_err(|_| format!("CSV line {}: value must be 0 through 1023", index + 1))?;
        if value > 1023 {
            return Err(format!(
                "CSV line {}: value must be 0 through 1023",
                index + 1
            ));
        }
        let label = match columns[1] {
            "0" => 0,
            "1" => 1,
            _ => return Err(format!("CSV line {}: label must be 0 or 1", index + 1)),
        };
        samples.push((value, label));
    }
    if !header_seen {
        return Err("CSV is empty; expected value,label".into());
    }
    let ones = samples.iter().filter(|(_, label)| *label == 1).count();
    let zeros = samples.len() - ones;
    if zeros < 2 || ones < 2 {
        return Err("need at least two examples for each label (0 and 1)".into());
    }

    let mut candidates = vec![0];
    for (value, _) in &samples {
        candidates.push(value + 1);
    }
    candidates.sort_unstable();
    candidates.dedup();

    let mut best = Model {
        threshold: 0,
        one_when_high: true,
        correct: 0,
        total: samples.len(),
    };
    for threshold in candidates {
        for one_when_high in [true, false] {
            let correct = samples
                .iter()
                .filter(|(value, label)| ((*value >= threshold) == one_when_high) == (*label == 1))
                .count();
            if correct > best.correct {
                best = Model {
                    threshold,
                    one_when_high,
                    correct,
                    total: samples.len(),
                };
            }
        }
    }
    if best.correct <= zeros.max(ones) {
        return Err("readings do not distinguish labels better than a constant guess".into());
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learns_high_values_as_one() {
        let model = train("value,label\n100,0\n200,0\n800,1\n900,1\n").unwrap();
        assert!(model.one_when_high);
        assert_eq!(model.correct, 4);
        assert_eq!(model.threshold, 201);
    }

    #[test]
    fn learns_low_values_as_one() {
        let model = train("value,label\n100,1\n200,1\n800,0\n900,0\n").unwrap();
        assert!(!model.one_when_high);
        assert_eq!(model.correct, 4);
        assert_eq!(model.threshold, 201);
    }

    #[test]
    fn rejects_uninformative_readings() {
        let error = train("value,label\n500,0\n500,1\n500,0\n500,1\n").unwrap_err();
        assert!(error.contains("do not distinguish"));
    }
}
