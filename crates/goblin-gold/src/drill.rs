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

/// The outcome flash for the current question. GG1 has **no wrong state** — the answer is
/// *auto-accepted* the instant the typed value equals it (checked after every keypress), so the
/// only marks are a correct flash and a skip (the action bar reveals the answer + moves on).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark {
    /// The typed value matched — auto-accepted, advancing to the next question.
    Right,
    /// The player skipped (the action bar): the answer was revealed and the round moved on.
    Skipped,
}

/// The most a player can type, in **digits** (the decimal point doesn't count) — GG1's input
/// length guard (`main.js:2293`).
const MAX_DIGITS: usize = 5;

/// A single-topic drill driven entirely by the data seam.
pub struct Drill {
    pub name: String,
    pub tag: String,
    questions: Vec<Question>,
    idx: usize,
    typed: String,
    last: Option<Mark>,
    revealed: Option<String>,
    solved: u32,
    skipped: u32,
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
            revealed: None,
            solved: 0,
            skipped: 0,
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
            revealed: None,
            solved: 0,
            skipped: 0,
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
    /// How many questions the player skipped (the action bar).
    pub fn skipped(&self) -> u32 {
        self.skipped
    }
    /// Questions resolved so far (solved + skipped) — the round ends once this reaches [`len`].
    pub fn consumed(&self) -> usize {
        (self.solved + self.skipped) as usize
    }
    /// The whole round has been worked through (every question solved or skipped).
    pub fn is_complete(&self) -> bool {
        self.consumed() >= self.questions.len()
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
    /// After a skip, the answer that was revealed (for the UI to show); cleared on the next input.
    pub fn revealed(&self) -> Option<&str> {
        self.revealed.as_deref()
    }

    /// Feed a keypad key — GG1's input model (`press()` → `checkAuto()`), which has **no
    /// Enter-to-submit**:
    /// - a **digit** appends (capped at [`MAX_DIGITS`], the decimal excluded) then **auto-checks**;
    /// - the **dot** appends one decimal point (a leading dot becomes `0.`) then auto-checks;
    /// - **Back** deletes the last char;
    /// - the action bar (**`Enter`**) is **SKIP**: it reveals the answer and moves on, counting as a
    ///   skip.
    ///
    /// "Auto-check" accepts the question the instant the typed value equals the answer — so a
    /// correct answer advances on its own, no confirm key.
    pub fn press(&mut self, k: Key) {
        match k {
            Key::Digit(d) => {
                if self.digit_count() < MAX_DIGITS {
                    self.typed.push((b'0' + d) as char);
                    self.last = None;
                    self.revealed = None;
                    self.check_auto();
                }
            }
            Key::Dot => {
                if !self.typed.contains('.') && self.digit_count() < MAX_DIGITS {
                    // A leading dot becomes "0." (GG1 `main.js:2289`).
                    if self.typed.is_empty() {
                        self.typed.push('0');
                    }
                    self.typed.push('.');
                    self.last = None;
                    self.revealed = None;
                    self.check_auto();
                }
            }
            Key::Back => {
                self.typed.pop();
                self.last = None;
                self.revealed = None;
            }
            Key::Enter => self.skip(),
        }
    }

    /// Digits typed so far (the decimal point isn't a digit — it doesn't count toward the cap).
    fn digit_count(&self) -> usize {
        self.typed.chars().filter(|c| c.is_ascii_digit()).count()
    }

    /// Parse the typed value the way GG1's `parseFloat` does (a trailing dot is just the integer),
    /// and if it equals the answer, **auto-accept**: flash correct, count it solved, advance.
    fn check_auto(&mut self) {
        let parsed = self.typed.trim().trim_end_matches('.').parse::<f64>();
        if let Ok(v) = parsed {
            if (v - self.expected()).abs() < 1e-9 {
                self.last = Some(Mark::Right);
                self.solved += 1;
                self.idx = (self.idx + 1) % self.questions.len();
                self.typed.clear();
            }
        }
    }

    /// Skip the current question (the action bar): reveal its answer, count a skip, and advance.
    pub fn skip(&mut self) {
        self.revealed = Some(fmt_answer(self.expected()));
        self.last = Some(Mark::Skipped);
        self.skipped += 1;
        self.idx = (self.idx + 1) % self.questions.len();
        self.typed.clear();
    }
}

/// Format an answer for display (no trailing-zero noise: `7.5`, `12`, `0.5`).
fn fmt_answer(a: f64) -> String {
    format!("{a}")
}

#[cfg(test)]
impl Drill {
    /// Test-only: a drill over a fixed question list, so input-mechanics tests (the length guard,
    /// the leading dot) don't race the auto-accept on whatever the real data's answer happens to be.
    fn from_questions(questions: Vec<Question>) -> Drill {
        Drill {
            name: "Test".into(),
            tag: String::new(),
            questions,
            idx: 0,
            typed: String::new(),
            last: None,
            revealed: None,
            solved: 0,
            skipped: 0,
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

    /// Type a number on the keypad (digits + dot only).
    fn type_num(d: &mut Drill, n: f64) {
        for c in format!("{n}").chars() {
            if let Some(k) = crate::keypad::Keypad::key_for_char(c) {
                d.press(k);
            }
        }
    }

    // Phase 3: the drill drives ALL 46 topics via the phase-2 transforms — every topic generates a
    // non-empty question set, and every generated answer is **auto-accepted** (no submit key): the
    // generated questions match the parity contract, since the transforms reproduce it.
    #[test]
    fn from_topic_drives_every_one_of_the_46_topics() {
        for m in crate::progression::modes() {
            let mut d = Drill::from_topic(&m.id);
            assert!(!d.is_empty(), "topic {} generated no questions", m.id);
            // Typing the first generated answer auto-accepts — without any Enter/submit key.
            let want = d.expected();
            type_num(&mut d, want);
            assert_eq!(
                d.last_mark(),
                Some(Mark::Right),
                "topic {}: generated answer {want} not auto-accepted",
                m.id
            );
        }
    }

    // The data seam IS the correctness contract: typing each vector's expected answer must
    // auto-accept, for every question in the topic (incl. the .5 decimals like 15→7.5).
    #[test]
    fn every_parity_vector_answer_is_accepted() {
        let mut d = Drill::from_seam("halves");
        let n = d.len();
        for _ in 0..n {
            let want = d.expected();
            type_num(&mut d, want);
            assert_eq!(
                d.last_mark(),
                Some(Mark::Right),
                "vector answer {want} should auto-accept"
            );
        }
        assert_eq!(d.solved(), n as u32);
        assert!(d.is_complete(), "every question solved → round complete");
    }

    // GG1 has no wrong state: a value that doesn't equal the answer simply isn't accepted (no
    // mark, no advance) — the player keeps typing or skips.
    #[test]
    fn a_non_matching_value_is_not_accepted_and_does_not_advance() {
        let mut d = Drill::from_seam("halves");
        let p0 = d.prompt().to_string();
        // A clearly-too-big value can't be a prefix of the (smaller) answer, so it never matches.
        let bad = d.expected() + 100.0;
        type_num(&mut d, bad);
        assert_eq!(d.last_mark(), None, "no wrong state — just no acceptance");
        assert_eq!(d.prompt(), p0, "a non-match must not advance");
        assert_eq!(d.solved(), 0);
    }

    // The action bar (Key::Enter) is SKIP: it reveals the answer, counts a skip, and advances —
    // it does NOT submit.
    #[test]
    fn the_action_bar_skips_revealing_the_answer() {
        let mut d = Drill::from_seam("halves");
        let p0 = d.prompt().to_string();
        let want = d.expected();
        d.press(Key::Enter);
        assert_eq!(d.last_mark(), Some(Mark::Skipped));
        assert_eq!(
            d.revealed(),
            Some(format!("{want}").as_str()),
            "answer shown"
        );
        assert_eq!(d.skipped(), 1);
        assert_eq!(d.solved(), 0);
        assert_ne!(d.prompt(), p0, "skip advances to the next question");
        assert_eq!(d.consumed(), 1, "a skip consumes the question");
    }

    // A whole round of skips still completes (initiation will then be false, but the round ends).
    #[test]
    fn a_round_of_skips_completes() {
        let mut d = Drill::from_seam("halves");
        let n = d.len();
        for _ in 0..n {
            d.press(Key::Enter);
        }
        assert_eq!(d.skipped(), n as u32);
        assert_eq!(d.solved(), 0);
        assert!(d.is_complete());
    }

    // The 5-digit length guard (decimal excluded): a sixth digit is ignored. Answer 123450 can't be
    // reached by any ≤5-digit prefix, so auto-accept never fires to confuse the count.
    #[test]
    fn input_is_capped_at_five_digits_excluding_the_decimal() {
        let mut d = Drill::from_questions(vec![Question {
            prompt: "x".into(),
            answer: 123_450.0,
        }]);
        for _ in 0..8 {
            d.press(Key::Digit(9)); // 99999999… but capped
        }
        assert_eq!(d.typed(), "99999", "capped at 5 digits");
        assert_eq!(d.solved(), 0, "never auto-accepted");
        // Once at the digit cap, the decimal point doesn't fit either (the guard counts digits).
        d.press(Key::Dot);
        assert_eq!(d.typed(), "99999", "no dot once at the digit cap");
        // But under the cap, the dot does NOT count toward it: 4 digits + dot + 1 digit = "1234.5".
        let mut d = Drill::from_questions(vec![Question {
            prompt: "x".into(),
            answer: 123_450.0,
        }]);
        for _ in 0..4 {
            d.press(Key::Digit(1));
        }
        d.press(Key::Dot);
        d.press(Key::Digit(5));
        assert_eq!(d.typed(), "1111.5", "decimal excluded from the 5-digit cap");
    }

    // A leading dot becomes "0." (GG1 main.js:2289) so a fractional answer is typeable.
    #[test]
    fn a_leading_dot_becomes_zero_dot() {
        // answer 999 → no "0.x" can match, so the box keeps what we type.
        let mut d = Drill::from_questions(vec![Question {
            prompt: "x".into(),
            answer: 999.0,
        }]);
        d.press(Key::Dot);
        assert_eq!(d.typed(), "0.", "a leading dot becomes 0.");
        d.press(Key::Digit(5));
        assert_eq!(d.typed(), "0.5");
    }

    // Auto-accept fires the instant the value matches — no Enter/submit key involved.
    #[test]
    fn auto_accepts_the_instant_the_value_matches() {
        let mut d = Drill::from_questions(vec![
            Question {
                prompt: "a".into(),
                answer: 12.0,
            },
            Question {
                prompt: "b".into(),
                answer: 3.0,
            },
        ]);
        d.press(Key::Digit(1)); // "1" ≠ 12
        assert_eq!(d.last_mark(), None);
        d.press(Key::Digit(2)); // "12" == 12 → auto-accept, advance
        assert_eq!(d.last_mark(), Some(Mark::Right));
        assert_eq!(d.solved(), 1);
        assert_eq!(d.prompt(), "b", "advanced to the next question");
        assert_eq!(d.typed(), "", "box cleared on accept");
    }

    #[test]
    fn another_topic_loads_too() {
        // proves it's data-driven, not hardcoded to halves
        let d = Drill::from_seam("times");
        assert_eq!(d.name, "Times");
        assert!(!d.is_empty() && d.prompt().contains('×'));
    }
}
