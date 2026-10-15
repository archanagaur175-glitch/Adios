pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// Ordered, append-only migration list. Never edit an applied migration;
/// add a new file + entry instead.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "001_init",
    sql: include_str!("m001_init.sql"),
}];
