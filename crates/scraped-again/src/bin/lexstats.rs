//! G16 — print the **lexicon's statistical fingerprint** machine-readably (one `key=value` per
//! line), for tuning the generator against the natural-language bands the honesty tests assert:
//!
//!   cargo run -p scraped-again --bin lexstats [seed] [n_tokens]
//!
//! The same metrics the `lexicon::stats` unit tests check (Zipf slope, Heaps β, char conditional
//! entropy, mean word length, function-word share, adjacent-vs-distant edit distance), plus a
//! sample phrase / record / frame so the structured nonsense is eyeballable.

use scraped_again::lexicon::{self, stats};

fn main() {
    let seed: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1337);
    let n: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8000);
    let corpus = lexicon::corpus(seed, n);
    let fw = lexicon::function_words();
    let (adj, dist) = stats::adjacent_vs_distant_similarity(&corpus, 7);
    let (freq_len, rare_len) = stats::abbreviation(&corpus);
    println!("seed={seed}");
    println!("tokens={n}");
    println!("zipf_slope={:.3}", stats::zipf_slope(&corpus)); // ~ -1.0
    println!("heaps_beta={:.3}", stats::heaps_beta(&corpus)); // ~0.5–0.8
    println!(
        "char_cond_entropy_bits={:.3}",
        stats::char_conditional_entropy(&corpus)
    ); // ~3–4
    println!(
        "function_word_share={:.3}",
        stats::function_word_share(&corpus, fw)
    );
    println!("mean_word_len_frequent_half={freq_len:.2}");
    println!("mean_word_len_rare_half={rare_len:.2}");
    println!("adjacent_edit_dist={adj:.3}");
    println!("distant_edit_dist={dist:.3}");
    // A few eyeball samples of the structured nonsense (still no English/lore).
    println!("sample_fragment={}", lexicon::phrase(seed, (3, -2), 3));
    println!("sample_phrase={}", lexicon::phrase(seed, (3, -2), 6));
    println!("sample_record={}", lexicon::record(seed, (5, 5)));
    println!("sample_frame_a={}", lexicon::frame(seed, (1, 0)));
    println!("sample_frame_b={}", lexicon::frame(seed, (9, 9)));
    // G20: this world's true names (block display names + parameter words) — eyeballable, and a
    // quick way to confirm no candidate accidentally spells English.
    for (key, word) in lexicon::vocabulary(seed) {
        println!("vocab_{key}={word}");
    }
}
