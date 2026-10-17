pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// Ordered, append-only migration list. Never edit an applied migration;
/// add a new file + entry instead.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "001_init",
        sql: include_str!("m001_init.sql"),
    },
    Migration {
        version: 2,
        name: "002_lyrics_cache",
        sql: include_str!("m002_lyrics_cache.sql"),
    },
    Migration {
        version: 3,
        name: "003_download_pins",
        sql: include_str!("m003_download_pins.sql"),
    },
    Migration {
        version: 4,
        name: "004_genre",
        sql: include_str!("m004_genre.sql"),
    },
];
