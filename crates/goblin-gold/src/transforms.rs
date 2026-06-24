//! GG1 **question transforms**, re-implemented in Rust (full-port phase 2). Each of the ~46
//! topics maps a raw `pool` datum → a question `(prompt, answer)`; this is the game LOGIC,
//! re-authored from the behaviour (not the JS — "share DATA not code"), and **proven against
//! `parity-vectors.json`**: for every mode, `{ transform(d) : d ∈ pool }` must equal the committed
//! `{p,a}` set exactly (see the test). The parity vectors ARE the spec.
//!
//! Datums are heterogeneous (a bare number, or an array whose first element is often a kind tag),
//! so they're parsed as `serde_json::Value` and matched structurally — exactly how the data seam
//! is shaped. Number→string formatting mirrors JavaScript's `String(n)` (integers print plain;
//! terminating decimals print shortest) so the prompt strings match byte-for-byte.

use serde::Deserialize;
use serde_json::Value;

/// The one-way-synced T229 modes export (id + raw pool per topic).
const MODES_JSON: &str = include_str!("../data/gg1/modes.json");

#[derive(Deserialize)]
struct Mode {
    id: String,
    #[serde(default)]
    pool: Vec<Value>,
}

/// A numeric datum as `f64` (answers + arithmetic). Non-numbers → 0.0 (never used for those).
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

/// A datum element as a string for prompt-building: a JSON string verbatim, else JS `String(n)`.
fn el(v: &Value) -> String {
    v.as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| jsnum(v))
}

/// JavaScript-faithful `String(Number)`: integers plain; integral floats drop the `.0`; other
/// terminating decimals shortest (Rust's `f64` Display matches JS for the clean values in the data).
fn jsnum(v: &Value) -> String {
    if let Some(i) = v.as_i64() {
        return i.to_string();
    }
    if let Some(u) = v.as_u64() {
        return u.to_string();
    }
    if let Some(x) = v.as_f64() {
        return if x.fract() == 0.0 {
            format!("{}", x as i64)
        } else {
            format!("{x}")
        };
    }
    String::new()
}

/// Join an array datum's elements with `sep` (each via [`el`]).
fn join(v: &Value, sep: &str) -> String {
    v.as_array()
        .map(|a| a.iter().map(el).collect::<Vec<_>>().join(sep))
        .unwrap_or_default()
}

/// JS `Number(v).toFixed(2)` — the £-money prompt format.
fn fixed2(v: &Value) -> String {
    format!("{:.2}", f(v))
}

/// JS two-digit zero-pad (`HH`/`MM`).
fn z2(v: &Value) -> String {
    format!("{:02}", f(v) as i64)
}

/// Transform one pool `datum` for `mode` into its `(prompt, answer)` question.
pub fn transform(mode: &str, d: &Value) -> (String, f64) {
    match mode {
        // ── computed answers (the datum is the operand) ──────────────────────────────────────
        "halves" => (el(d), f(d) / 2.0),
        "doubles" => (el(d), f(d) * 2.0),
        "squares" => {
            let n = f(d);
            (format!("{}²", el(d)), n * n)
        }
        "times" => (
            format!("{} × {}", el(&d[0]), el(&d[1])),
            f(&d[0]) * f(&d[1]),
        ),
        "bonds" => (format!("{} + ? = 100", el(d)), 100.0 - f(d)),
        "cubes" => {
            if d[0] == "c" {
                let n = f(&d[1]);
                (format!("{}³", el(&d[1])), n * n * n)
            } else {
                (format!("{}{}", el(&d[0]), el(&d[1])), f(&d[2]))
            }
        }

        // ── fraction→decimal: the prompt IS the fraction string, answer the decimal ──────────
        "fractions" | "fractions2" => (el(&d[0]), f(&d[1])),

        // ── add/subtract (kind flag = subtract) ──────────────────────────────────────────────
        "addsub" | "addsub2" => {
            let (a, b) = (f(&d[0]), f(&d[1]));
            if f(&d[2]) != 0.0 {
                (format!("{} − {}", el(&d[0]), el(&d[1])), a - b)
            } else {
                (format!("{} + {}", el(&d[0]), el(&d[1])), a + b)
            }
        }

        // ── pre-computed answer in the datum; transform just formats the prompt ──────────────
        "bonds2" => (format!("{} + ? = {}", el(&d[0]), el(&d[1])), f(&d[2])),
        "placevalue" | "placevalue2" => (
            format!("{} {} {}", el(&d[0]), el(&d[1]), el(&d[2])),
            f(&d[3]),
        ),
        "fractionsof" | "fractionsof2" => (
            format!("{}/{} of {}", el(&d[0]), el(&d[1]), el(&d[2])),
            f(&d[3]),
        ),
        "percentages" | "percentages2" => (format!("{}% of {}", el(&d[0]), el(&d[1])), f(&d[2])),
        "rounding" => (format!("{} to nearest {}", el(&d[0]), el(&d[1])), f(&d[2])),
        "largermd" => (
            format!("{} {} {}", el(&d[0]), el(&d[1]), el(&d[2])),
            f(&d[3]),
        ),
        "metric" => (
            format!("{} {} in {}", el(&d[0]), el(&d[1]), el(&d[2])),
            f(&d[3]),
        ),
        "scaling" => (
            format!("{}→{}, {}→?", el(&d[0]), el(&d[1]), el(&d[2])),
            f(&d[3]),
        ),
        "percentoff" => (format!("{}% off {}", el(&d[0]), el(&d[1])), f(&d[2])),
        "balance" => (
            format!(
                "{} {} {} = {} {} ?",
                el(&d[0]),
                el(&d[1]),
                el(&d[2]),
                el(&d[3]),
                el(&d[4])
            ),
            f(&d[5]),
        ),
        "lcmhcf" => (
            format!("{} {},{}", el(&d[0]), el(&d[1]), el(&d[2])),
            f(&d[3]),
        ),
        "timegap" => (
            format!("{}:{} → {}:{}", z2(&d[0]), z2(&d[1]), z2(&d[2]), z2(&d[3])),
            f(&d[4]),
        ),
        "roman" => (el(&d[0]), f(&d[1])),
        "primes" => (format!("next prime > {}", el(&d[0])), f(&d[1])),
        "bodmas" | "algebra" | "negatives" => (el(&d[0]), f(&d[1])),
        "xtricks" => (format!("{} × {}", el(&d[0]), el(&d[1])), f(&d[2])),
        "pctup" => (format!("{} + {}%", el(&d[1]), el(&d[0])), f(&d[2])),
        "volume" => (
            format!("vol {}×{}×{}", el(&d[0]), el(&d[1]), el(&d[2])),
            f(&d[3]),
        ),

        // ── kind-tagged variants ─────────────────────────────────────────────────────────────
        "partwhole" => {
            if d[0] == "f" {
                (
                    format!("{}/{} of ? = {}", el(&d[1]), el(&d[2]), el(&d[3])),
                    f(&d[4]),
                )
            } else {
                (format!("{}% of ? = {}", el(&d[1]), el(&d[2])), f(&d[3]))
            }
        }
        "mean" => {
            if d[0] == "f" {
                (format!("mean of {}", join(&d[1], ",")), f(&d[2]))
            } else {
                (
                    format!("mean of {},? is {}", join(&d[1], ","), el(&d[2])),
                    f(&d[3]),
                )
            }
        }
        "ratioshare" => {
            if d[0] == "2" {
                let bs = if d[4] == "big" { "bigger" } else { "smaller" };
                (
                    format!("{} in {}:{} → {}", el(&d[1]), el(&d[2]), el(&d[3]), bs),
                    f(&d[5]),
                )
            } else {
                (
                    format!(
                        "{} in {}:{}:{} → biggest",
                        el(&d[1]),
                        el(&d[2]),
                        el(&d[3]),
                        el(&d[4])
                    ),
                    f(&d[6]),
                )
            }
        }
        "money" => {
            if d[0] == "m" {
                (format!("{} × £{}", el(&d[1]), fixed2(&d[2])), f(&d[3]))
            } else {
                (
                    format!("change from £{} of £{}", el(&d[1]), fixed2(&d[2])),
                    f(&d[3]),
                )
            }
        }
        "digitsum" => {
            if d[0] == "s" {
                (format!("digit sum of {}", el(&d[1])), f(&d[2]))
            } else {
                (format!("remainder {} ÷ 9", el(&d[1])), f(&d[2]))
            }
        }
        "fdp" => {
            if d[0] == "d" {
                (format!("{}% as a decimal", el(&d[1])), f(&d[2]))
            } else if d[0] == "p" {
                (format!("{} as a %", el(&d[1])), f(&d[2]))
            } else {
                (format!("{}/{} as a %", el(&d[1]), el(&d[2])), f(&d[3]))
            }
        }
        "sequences" | "sequences2" => {
            if d[0] == "next" {
                (format!("next: {}", join(&d[1], ", ")), f(&d[2]))
            } else {
                let add = f(&d[2]) as i64;
                let suffix = if add < 0 {
                    format!(" − {}", -add)
                } else if add > 0 {
                    format!(" + {add}")
                } else {
                    String::new()
                };
                (
                    format!("{}n{}, term {}", el(&d[1]), suffix, el(&d[3])),
                    f(&d[4]),
                )
            }
        }
        "area" => match d[0].as_str() {
            Some("ar") => (format!("area {}×{}", el(&d[1]), el(&d[2])), f(&d[3])),
            Some("pr") => (format!("perim {}×{}", el(&d[1]), el(&d[2])), f(&d[3])),
            _ => (format!("△ {}×{}", el(&d[1]), el(&d[2])), f(&d[3])),
        },
        "angles" => match d[0].as_str() {
            Some("L") => (format!("line {} + ?", el(&d[1])), f(&d[2])),
            Some("P") => (format!("point {} + ?", el(&d[1])), f(&d[2])),
            _ => (format!("△ {}, {} → ?", el(&d[1]), el(&d[2])), f(&d[3])),
        },
        "mmr" => {
            let label = match d[0].as_str() {
                Some("med") => "median",
                Some("mod") => "mode",
                _ => "range",
            };
            (format!("{} of {}", label, join(&d[1], ",")), f(&d[2]))
        }
        "sdt" => match d[0].as_str() {
            Some("d") => (
                format!("dist: {}km/h × {}h", el(&d[1]), el(&d[2])),
                f(&d[3]),
            ),
            Some("s") => (
                format!("speed: {}km in {}h", el(&d[1]), el(&d[2])),
                f(&d[3]),
            ),
            _ => (
                format!("time: {}km at {}km/h", el(&d[1]), el(&d[2])),
                f(&d[3]),
            ),
        },
        "factors" => match d[0].as_str() {
            Some("nf") => (format!("# factors of {}", el(&d[1])), f(&d[2])),
            Some("nm") => (format!("next ×{} > {}", el(&d[1]), el(&d[2])), f(&d[3])),
            _ => (format!("biggest prime of {}", el(&d[1])), f(&d[2])),
        },
        other => panic!("unknown mode `{other}` — no transform"),
    }
}

/// Every question for `mode_id`, generated by applying [`transform`] to each pool datum. Panics if
/// the mode is absent from `modes.json`.
pub fn generate(mode_id: &str) -> Vec<(String, f64)> {
    let modes: Vec<Mode> = serde_json::from_str(MODES_JSON).expect("modes.json");
    let m = modes
        .into_iter()
        .find(|m| m.id == mode_id)
        .unwrap_or_else(|| panic!("mode `{mode_id}` not in modes.json"));
    m.pool.iter().map(|d| transform(mode_id, d)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const PARITY_JSON: &str = include_str!("../data/gg1/parity-vectors.json");

    #[derive(Deserialize)]
    struct Pa {
        p: String,
        a: f64,
    }

    /// THE phase-2 gate: for EVERY mode, the questions our Rust transforms generate from the pool
    /// reproduce the committed `{p,a}` parity set EXACTLY — same count, every vector matched (prompt
    /// byte-identical, answer within float tolerance), and no extras. The parity vectors are the
    /// behavioural contract for the re-implemented logic (the JS is never run).
    #[test]
    fn every_mode_reproduces_its_parity_vectors() {
        let modes: Vec<Mode> = serde_json::from_str(MODES_JSON).expect("modes.json");
        let parity: HashMap<String, Vec<Pa>> =
            serde_json::from_str(PARITY_JSON).expect("parity-vectors.json");
        assert_eq!(modes.len(), 46, "expected all 46 GG1 topics");

        for m in &modes {
            let expected = parity
                .get(&m.id)
                .unwrap_or_else(|| panic!("mode `{}` missing from parity vectors", m.id));
            let mut produced = generate(&m.id);
            assert_eq!(
                produced.len(),
                expected.len(),
                "mode `{}`: produced {} questions, parity has {}",
                m.id,
                produced.len(),
                expected.len()
            );
            // Multiset equality (order-independent — the export sorts the set).
            for e in expected {
                let pos = produced
                    .iter()
                    .position(|(p, a)| *p == e.p && (a - e.a).abs() < 1e-9)
                    .unwrap_or_else(|| {
                        panic!(
                            "mode `{}`: no produced question matches parity {{p:{:?}, a:{}}}",
                            m.id, e.p, e.a
                        )
                    });
                produced.swap_remove(pos);
            }
            assert!(
                produced.is_empty(),
                "mode `{}`: {} generated questions have NO matching parity vector: {:?}",
                m.id,
                produced.len(),
                produced
            );
        }
    }

    #[test]
    fn halves_spot_check() {
        // A concrete sanity anchor for the computed-answer path.
        let q = transform("halves", &serde_json::json!(15));
        assert_eq!(q.0, "15");
        assert!((q.1 - 7.5).abs() < 1e-9);
    }
}
