#[cfg(any(feature = "diesel1", feature = "diesel2"))]
mod diesel;

#[cfg(feature = "postgres-types")]
mod postgres_types;

/// Represents the NUMERIC value in PostgreSQL, closely mirroring the PG wire protocol without NaN.
///
/// Reference: <https://github.com/postgres/postgres/blob/master/src/backend/utils/adt/numeric.c#L106>
pub(in crate::third_party::postgres) struct PgNumeric<D> {
    pub neg: bool,
    pub weight: i16,
    pub scale: u16,
    pub digits: D,
}

impl<D: ExactSizeIterator<Item = u16>> From<PgNumeric<D>> for crate::repr::Repr<10> {
    fn from(_: PgNumeric<D>) -> Self {
        todo!()
    }
}
