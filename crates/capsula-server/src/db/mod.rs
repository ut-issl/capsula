//! Per-resource database queries.
//!
//! Each module owns the queries of one table (or a tight group of tables) and
//! the Rust types for its rows. Functions take any [`sqlx::PgExecutor`], so
//! they work with a pool as well as inside a transaction. Every org-scoped
//! query takes the `org_id` explicitly; only `users`, `tokens` and `ssh_keys`
//! are not scoped to an organization.
//!
//! Errors are returned as [`sqlx::Error`]; use [`is_unique_violation`] to
//! recognize duplicate keys (for example a run pushed twice).

pub mod audit;
pub mod captured_files;
pub mod invitations;
pub mod memberships;
pub mod orgs;
pub mod run_outputs;
pub mod runs;
pub mod ssh_keys;
pub mod tokens;
pub mod users;
pub mod vaults;

#[cfg(test)]
pub(crate) mod test_support;

/// Whether `error` is a unique-constraint violation (`SQLSTATE 23505`).
#[must_use]
pub fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(e) if e.is_unique_violation())
}

/// Whether `error` is a foreign-key violation (`SQLSTATE 23503`).
#[must_use]
pub fn is_foreign_key_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(e) if e.is_foreign_key_violation())
}

/// Turn a value stored as `TEXT` into its enum, reporting unknown values as a
/// decode error. The `CHECK` constraints of the schema make this unreachable
/// for rows written by this server.
fn decode_enum<T>(
    column: &'static str,
    value: &str,
    parse: fn(&str) -> Option<T>,
) -> sqlx::Result<T> {
    parse(value).ok_or_else(|| sqlx::Error::ColumnDecode {
        index: column.to_owned(),
        source: format!("unknown value `{value}`").into(),
    })
}
