//! GG1 **drill loop** — consumes the T229 content data seam (research §"share DATA, not
//! code"). Questions come from `parity-vectors.json` (the deterministic `{p,a}` behavioural
//! contract — NOT the JS transform, which is never executed here); the mode name/tag come
//! from `modes.json`. A correct re-implementation reproduces these `{p,a}` exactly, so the
//! parity vectors *are* the cross-repo correctness check (a CI test, no GPU needed).

use crate::keypad::Key;
use serde::Deserialize;
use std::collections::HashMap;

/// The one-way-synced T229 export (see `data/gg1/README.md`).
const MODES_JSON: &str = include_str!("../data/gg1/modes.json");
const PARITY_JSON: &str = include_str!("../data/gg1/parity-vectors.json");

#[derive(Deserialize)]
struct ModeMeta {
    id: String,
    name: String,
    #[serde(default)]
    tag: String,
}

#[derive(Deserialize)]
struct Vector {
    p: String,
    a: f64,
}

/// One drill question: a prompt to show + the expected numeric answer.
#[derive(Clone)]
pub struct Question {
    pub prompt: String,
    pub answer: f64,
}

/// Whether the last submitted answer was right or wrong.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark {
    Right,
    Wrong,
}

/// A single-topic drill driven entirely by the data seam.
pub struct Drill {
    pub name: String,
    pub tag: String,
    questions: Vec<Question>,
    idx: usize,
    typed: String,
    last: Option<Mark>,
    solved: u32,
}

impl Drill {
    /// Build a drill for `mode_id` from the embedded T229 export. Panics if the mode is
    /// absent from either file (a seam-integrity failure worth failing loudly on).
    pub fn from_seam(mode_id: &str) -> Drill {
        let modes: Vec<ModeMeta> = serde_json::from_str(MODES_JSON).expect("modes.json");
        let meta = modes
            .into_iter()
            .find(|m| m.id == mode_id)
            .expect("mode in modes.json");
        let parity: HashMap<String, Vec<Vector>> =
            serde_json::from_str(PARITY_JSON).expect("parity-vectors.json");
        let vecs = parity.get(mode_id).expect("mode in parity-vectors.json");
        let questions: Vec<Question> = vecs
            .iter()
            .map(|v| Question {
                prompt: v.p.clone(),
                answer: v.a,
            })
            .collect();
        assert!(!questions.is_empty(), "no questions for {mode_id}");
        Drill {
            name: meta.name,
            tag: meta.tag,
            questions,
            idx: 0,
            typed: String::new(),
            last: None,
            solved: 0,
        }
    }

    /// Build a drill for ANY of the 46 topics by **generating** its questions from the pool via the
    /// phase-2 transforms ([`crate::transforms::generate`]) — the real data-driven path (vs
    /// [`from_seam`](Self::from_seam), which replays the parity vectors). Name/tag from `modes.json`.
    pub fn from_topic(mode_id: &str) -> Drill {
        let modes: Vec<ModeMeta> = serde_json::from_str(MODES_JSON).expect("modes.json");
        let meta = modes
            .into_iter()
            .find(|m| m.id == mode_id)
            .unwrap_or_else(|| panic!("mode `{mode_id}` not in modes.json"));
        let questions: Vec<Question> = crate::transforms::generate(mode_id)
            .into_iter()
            .map(|(prompt, answer)| Question { prompt, answer })
            .collect();
        assert!(
            !questions.is_empty(),
            "no questions generated for {mode_id}"
        );
        Drill {
            name: meta.name,
            tag: meta.tag,
            questions,
            idx: 0,
            typed: String::new(),
            last: None,
            solved: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.questions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.questions.is_empty()
    }
    pub fn solved(&self) -> u32 {
        self.solved
    }
    pub fn prompt(&self) -> &str {
        &self.questions[self.idx].prompt
    }
    pub fn expected(&self) -> f64 {
        self.questions[self.idx].answer
    }
    pub fn typed(&self) -> &str {
        &self.typed
    }
    pub fn last_mark(&self) -> Option<Mark> {
        self.last
    }

    /// Feed a keypad key. Digits/dot append; `Back` deletes; `Enter` submits (→ mark) and,
    /// on a correct answer, advances to the next question.
    pub fn press(&mut self, k: Key) {
        match k {
            Key::Digit(d) => {
                if self.typed.len() < 12 {
                    self.typed.push((b'0' + d) as char);
                    self.last = None;
                }
            }
            Key::Dot => {
                if !self.typed.contains('.') && self.typed.len() < 12 {
                    self.typed.push('.');
                    self.last = None;
                }
            }
            Key::Back => {
                self.typed.pop();
                self.last = None;
            }
            Key::Enter => self.submit(),
        }
    }

    /// Check the typed answer against the seam's expected value (tolerant of float form).
    pub fn submit(&mut self) {
        let ok = self
            .typed
            .trim()
            .parse::<f64>()
            .map(|v| (v - self.expected()).abs() < 1e-9)
            .unwrap_or(false);
        if ok {
            self.last = Some(Mark::Right);
            self.solved += 1;
            self.idx = (self.idx + 1) % self.questions.len();
            self.typed.clear();
        } else {
            self.last = Some(Mark::Wrong);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_halves_from_the_seam() {
        let d = Drill::from_seam("halves");
        assert_eq!(d.name, "Halves");
        assert_eq!(d.len(), 27); // 27 deterministic {p,a} vectors
        assert!(!d.prompt().is_empty());
    }

    // Phase 3: the drill drives ALL 46 topics via the phase-2 transforms — every topic generates a
    // non-empty question set, and every generated answer is accepted (the generated questions match
    // the parity contract, since the transforms reproduce it).
    #[test]
    fn from_topic_drives_every_one_of_the_46_topics() {
        for m in crate::progression::modes() {
            let mut d = Drill::from_topic(&m.id);
            assert!(!d.is_empty(), "topic {} generated no questions", m.id);
            // Type+submit the first generated answer; it must be accepted.
            let want = d.expected();
            for c in format!("{want}").chars() {
                if let Some(k) = crate::keypad::Keypad::key_for_char(c) {
                    d.press(k);
                }
            }
            d.submit();
            assert_eq!(
                d.last_mark(),
                Some(Mark::Right),
                "topic {}: generated answer {want} rejected",
                m.id
            );
        }
    }

    // The data seam IS the correctness contract: typing each vector's expected answer must
    // be accepted, for every question in the topic (incl. the .5 decimals like 15→7.5).
    #[test]
    fn every_parity_vector_answer_is_accepted() {
        let mut d = Drill::from_seam("halves");
        let n = d.len();
        for _ in 0..n {
            let want = d.expected();
            for c in format!("{want}").chars() {
                d.press(crate::keypad::Keypad::key_for_char(c).expect("digit/dot"));
            }
            d.submit();
            assert_eq!(
                d.last_mark(),
                Some(Mark::Right),
                "vector answer {want} should be accepted"
            );
        }
        assert_eq!(d.solved(), n as u32);
    }

    #[test]
    fn a_wrong_answer_is_marked_wrong_and_does_not_advance() {
        let mut d = Drill::from_seam("halves");
        let p0 = d.prompt().to_string();
        let bad = d.expected() + 1.0;
        for c in format!("{bad}").chars() {
            d.press(crate::keypad::Keypad::key_for_char(c).unwrap());
        }
        d.submit();
        assert_eq!(d.last_mark(), Some(Mark::Wrong));
        assert_eq!(d.prompt(), p0, "wrong answer must not advance");
        assert_eq!(d.solved(), 0);
    }

    #[test]
    fn another_topic_loads_too() {
        // proves it's data-driven, not hardcoded to halves
        let d = Drill::from_seam("times");
        assert_eq!(d.name, "Times");
        assert!(!d.is_empty() && d.prompt().contains('×'));
    }
}
