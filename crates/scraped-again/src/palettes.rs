//! Scraped Again's curated colour palettes (E10/E8 biomes) — art-direction/content.
//! The engine (`bm_render::palette`) maps the frame to *whatever* ramp it's handed via
//! `PalettePass::set_colors`; this is the game's set of named ramps (one per biome) and
//! the index→ramp resolution the app + headless tool feed it. Moved out of the engine in
//! M9 so bm-render carries no specific look.

/// A named, ordered (dark → light) colour ramp. Kept small + restrained on purpose.
pub struct Palette {
    pub name: &'static str,
    pub colors: &'static [[f32; 3]],
}

/// Curated palettes, none of them the stock voxel/Minecraft hues. Index 0 is a neutral
/// mono ramp; the rest lean into a single restrained mood. `count` (below) can use fewer
/// than a palette's full length for an even harder look.
pub const PALETTES: &[Palette] = &[
    Palette {
        name: "mono",
        colors: &[
            [0.05, 0.06, 0.08],
            [0.30, 0.33, 0.38],
            [0.58, 0.62, 0.68],
            [0.86, 0.89, 0.94],
        ],
    },
    Palette {
        name: "verdant",
        colors: &[
            [0.06, 0.09, 0.08],
            [0.13, 0.24, 0.20],
            [0.27, 0.42, 0.31],
            [0.52, 0.66, 0.44],
            [0.83, 0.86, 0.66],
        ],
    },
    Palette {
        name: "ash",
        colors: &[
            [0.07, 0.08, 0.10],
            [0.22, 0.26, 0.31],
            [0.40, 0.46, 0.52],
            [0.62, 0.68, 0.72],
            [0.88, 0.90, 0.92],
        ],
    },
    Palette {
        name: "ember",
        colors: &[
            [0.05, 0.04, 0.06],
            [0.24, 0.12, 0.14],
            [0.55, 0.22, 0.16],
            [0.82, 0.45, 0.22],
            [0.95, 0.80, 0.55],
        ],
    },
    Palette {
        name: "dusk",
        colors: &[
            [0.06, 0.06, 0.11],
            [0.20, 0.18, 0.33],
            [0.40, 0.34, 0.52],
            [0.63, 0.52, 0.66],
            [0.88, 0.80, 0.82],
        ],
    },
    Palette {
        name: "mist",
        colors: &[
            [0.09, 0.12, 0.14],
            [0.24, 0.34, 0.36],
            [0.45, 0.58, 0.57],
            [0.70, 0.80, 0.76],
            [0.92, 0.95, 0.92],
        ],
    },
    // --- Two-hue palettes: a dark, grimy base ramp with a contrasting accent at the bright
    // end. Because the look maps luminance onto the ramp, that accent lands on highlights —
    // i.e. the point lights — so they "pop" in a clashing hue against the base. Best with
    // the sun off.
    Palette {
        // Red base with pops of green (the requested look): a blood/rust ramp, acid-green
        // glints where the light hits.
        name: "rust",
        colors: &[
            [0.06, 0.04, 0.05],
            [0.24, 0.07, 0.07],
            [0.48, 0.13, 0.10],
            [0.74, 0.28, 0.16],
            [0.46, 0.92, 0.38],
        ],
    },
    Palette {
        // Deep purple → hot magenta, with a cyan pop. Synthwave-in-a-cave.
        name: "neon",
        colors: &[
            [0.04, 0.03, 0.07],
            [0.15, 0.07, 0.22],
            [0.36, 0.10, 0.40],
            [0.82, 0.18, 0.60],
            [0.40, 0.95, 0.96],
        ],
    },
    Palette {
        // Cold slate base with warm sodium-amber pops — streetlights through fog.
        name: "sodium",
        colors: &[
            [0.04, 0.05, 0.07],
            [0.10, 0.15, 0.19],
            [0.20, 0.28, 0.30],
            [0.62, 0.40, 0.14],
            [1.00, 0.78, 0.34],
        ],
    },
    Palette {
        // Dark soil/olive base with an acid-lime pop — toxic bog.
        name: "bog",
        colors: &[
            [0.05, 0.05, 0.04],
            [0.15, 0.13, 0.09],
            [0.28, 0.24, 0.12],
            [0.40, 0.46, 0.16],
            [0.78, 0.98, 0.34],
        ],
    },
    // --- Batch 2 (10 more): a wider spread of one- and two-hue ramps, all dark-leaning to
    // suit the grimy mood. Two-hue ones (oxide, bruise, cobalt, slime) put a clashing accent
    // at the bright end so the point lights pop in a complementary colour.
    Palette {
        // Verdigris teal base with a rust-orange pop — weathered copper/patina.
        name: "oxide",
        colors: &[
            [0.05, 0.07, 0.07],
            [0.10, 0.20, 0.19],
            [0.21, 0.38, 0.34],
            [0.55, 0.34, 0.16],
            [0.93, 0.62, 0.26],
        ],
    },
    Palette {
        // Deep indigo base, sickly acid yellow-green pop — a bruise.
        name: "bruise",
        colors: &[
            [0.05, 0.04, 0.08],
            [0.16, 0.10, 0.24],
            [0.31, 0.16, 0.34],
            [0.48, 0.30, 0.30],
            [0.82, 0.88, 0.32],
        ],
    },
    Palette {
        // Cold deep-sea blue ramp, black → cyan-white.
        name: "abyss",
        colors: &[
            [0.02, 0.03, 0.06],
            [0.06, 0.12, 0.24],
            [0.12, 0.28, 0.45],
            [0.31, 0.53, 0.67],
            [0.80, 0.93, 0.97],
        ],
    },
    Palette {
        // Toxic green ramp, near-black → acid lime-white.
        name: "venom",
        colors: &[
            [0.03, 0.05, 0.03],
            [0.08, 0.18, 0.08],
            [0.18, 0.36, 0.14],
            [0.42, 0.66, 0.22],
            [0.82, 0.98, 0.60],
        ],
    },
    Palette {
        // Lava ramp: black → blood → orange → yellow-white. Brighter/hotter than ember.
        name: "magma",
        colors: &[
            [0.04, 0.02, 0.02],
            [0.24, 0.05, 0.03],
            [0.55, 0.14, 0.05],
            [0.86, 0.42, 0.10],
            [1.00, 0.88, 0.48],
        ],
    },
    Palette {
        // Oily black → brown → amber — tar, sump, crude.
        name: "tar",
        colors: &[
            [0.03, 0.03, 0.03],
            [0.14, 0.10, 0.07],
            [0.28, 0.20, 0.10],
            [0.50, 0.36, 0.16],
            [0.86, 0.68, 0.32],
        ],
    },
    Palette {
        // Navy base with a hot-orange pop — classic complementary blue/orange.
        name: "cobalt",
        colors: &[
            [0.04, 0.05, 0.10],
            [0.09, 0.13, 0.28],
            [0.16, 0.25, 0.46],
            [0.62, 0.40, 0.18],
            [1.00, 0.66, 0.26],
        ],
    },
    Palette {
        // Dark teal base with a magenta pop — toxic slime.
        name: "slime",
        colors: &[
            [0.04, 0.06, 0.07],
            [0.10, 0.20, 0.20],
            [0.18, 0.36, 0.34],
            [0.52, 0.18, 0.42],
            [0.94, 0.36, 0.72],
        ],
    },
    Palette {
        // Muted sepia → cream — old parchment, candlelight. Soft and warm.
        name: "parchment",
        colors: &[
            [0.06, 0.05, 0.04],
            [0.20, 0.16, 0.11],
            [0.40, 0.33, 0.22],
            [0.64, 0.56, 0.40],
            [0.92, 0.86, 0.70],
        ],
    },
    Palette {
        // Cold blue → ice white — frost, distinct from mist (bluer, colder).
        name: "frost",
        colors: &[
            [0.04, 0.05, 0.09],
            [0.14, 0.20, 0.31],
            [0.30, 0.43, 0.55],
            [0.58, 0.73, 0.83],
            [0.90, 0.97, 1.00],
        ],
    },
];
