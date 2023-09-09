// These tests should be disabled by default, only tested when there
// is a PostgreSQL server in the testing machine.
//
// To set up the test, make sure the password is removed from the testing database.
// Instructions: <https://dba.stackexchange.com/a/126176>

use dashu_float::DBig;

mod helper_macros;

/// Gets the URL for connecting to PostgreSQL for testing. Set the POSTGRES_URL
/// environment variable to change from the default of "postgres://postgres@localhost".
fn get_postgres_url() -> String {
    std::env::var("POSTGRES_URL").unwrap_or("postgres://postgres@localhost".to_string())
}

/// Test cases in format (precision, scale, string repr sent to server, decimal repr)
fn get_test_cases() -> [(u32, i32, &'static str, DBig); 21] {
    [
        // integers has a precision at least to the decimal point
        (1, 0, "1", dbig!(1)),
        (10, 0, "1", dbig!(1)),
        (2, 1, "1", dbig!(1.0)),
        (5, 4, "1", dbig!(1.0000)),
        (5, 1, "1e3", dbig!(1000.0)),
        (1, -4, "1e4", dbig!(10000)),
        (31, 0, "1e30", dbig!(1e30).with_precision(31).unwrap()),
        (32, 1, "1e30", dbig!(1e30).with_precision(32).unwrap()),
        (41, 10, "1e30", dbig!(1e30).with_precision(41).unwrap()),
        (9, 0, "123456789", dbig!(123456789)),
        // fractionals
        (1, 1, "0.1", dbig!(1e-1)),
        (1, 4, "0.0001", dbig!(1e-4)),
        (8, 30, "12345678e-30", dbig!(12345678e-30)),
        // mixed
        (8, 1, "1234567.8", dbig!(1234567.8)),
        (8, 2, "123456.78", dbig!(123456.78)),
        (8, 3, "12345.678", dbig!(12345.678)),
        (8, 4, "1234.5678", dbig!(1234.5678)),
        (10, 5, "1234.5678", dbig!(1234.56780)),
        (20, 10, "123456789.0123456789", dbig!(123456789.0123456789)),
        // rounded
        (1, 1, "0.1234", dbig!(1e-1)),
        (1, 1, "0.5678", dbig!(6e-1)),
    ]
}

// test the `postgres` crate
mod postgres {
    use super::*;
    use ::postgres::{Client, NoTls};

    // execute query and get the result as a single decimal number
    fn query_value(client: &mut Client, query: &str) -> DBig {
        let result: Option<DBig> = client.query(query, &[]).unwrap()[0].get(0);
        result.unwrap()
    }

    // execute query with param `value`, and get the result as a single decimal number
    fn write_value(client: &mut Client, query: &str, value: &DBig) -> DBig {
        let result: Option<DBig> = client.query(query, &[value]).unwrap()[0].get(0);
        result.unwrap()
    }

    #[test]
    #[ignore]
    fn test_special_values() {
        let mut client = Client::connect(&get_postgres_url(), NoTls).unwrap();

        let result: Option<DBig> = client.query("SELECT NULL::NUMERIC", &[]).unwrap()[0].get(0);
        assert_eq!(result, None);

        assert_eq!(query_value(&mut client, "SELECT 0::NUMERIC"), DBig::ZERO);
        assert_eq!(query_value(&mut client, "SELECT 1::NUMERIC"), DBig::ONE);
        assert_eq!(query_value(&mut client, "SELECT -1::NUMERIC"), DBig::NEG_ONE);
        assert_eq!(query_value(&mut client, "SELECT 'INF'::NUMERIC"), DBig::INFINITY);
        assert_eq!(query_value(&mut client, "SELECT '-INF'::NUMERIC"), DBig::NEG_INFINITY);

        assert_eq!(write_value(&mut client, "SELECT $1::NUMERIC", &DBig::ZERO), DBig::ZERO);
        assert_eq!(write_value(&mut client, "SELECT $1::NUMERIC", &DBig::ONE), DBig::ONE);
        assert_eq!(write_value(&mut client, "SELECT $1::NUMERIC", &DBig::NEG_ONE), DBig::NEG_ONE);
        assert_eq!(write_value(&mut client, "SELECT $1::NUMERIC", &DBig::INFINITY), DBig::INFINITY);
        assert_eq!(
            write_value(&mut client, "SELECT $1::NUMERIC", &DBig::NEG_INFINITY),
            DBig::NEG_INFINITY
        );
    }

    #[test]
    #[ignore]
    fn test_normal_values() {
        let mut client = Client::connect(&get_postgres_url(), NoTls).unwrap();

        for (precision, scale, string, decimal) in get_test_cases() {
            let result = query_value(
                &mut client,
                &format!("SELECT {}::NUMERIC({}, {})", string, precision, scale),
            );
            assert_eq!(result, decimal);
            assert_eq!(result.precision(), decimal.precision());

            let result = write_value(
                &mut client,
                &format!("SELECT $1::NUMERIC({}, {})", precision, scale),
                &decimal,
            );
            assert_eq!(result, decimal);
            assert_eq!(result.precision(), decimal.precision());
        }
    }
}

mod diesel_v1 {
    use super::*;
    use ::diesel_v1::{
        self as diesel,
        deserialize::QueryableByName,
        pg::{Pg, PgConnection},
        query_dsl::RunQueryDsl,
        row::NamedRow,
        Connection,
    };

    #[derive(Debug, PartialEq)]
    struct TestRow(pub DBig);

    impl QueryableByName<Pg> for TestRow {
        fn build<R: NamedRow<Pg>>(row: &R) -> diesel::deserialize::Result<Self> {
            let number: DBig = row.get("numeric")?;
            Ok(Self(number))
        }
    }

    // execute query and get the result as a single decimal number
    fn query_value(client: &PgConnection, query: &str) -> DBig {
        let result: Option<TestRow> = diesel::sql_query(query).get_result(client).unwrap();
        result.unwrap().0
    }

    // execute query with param `value`, and get the result as a single decimal number
    fn write_value(client: &PgConnection, query: &str, value: &DBig) -> DBig {
        let result: Option<TestRow> = diesel::sql_query(query)
            .bind(value)
            .get_result(client)
            .unwrap();
        result.unwrap().0
    }

    #[test]
    #[ignore]
    fn test_special_values() {
        let client = PgConnection::establish(&get_postgres_url()).unwrap();

        let result: Option<TestRow> = diesel::sql_query("SELECT NULL::NUMERIC")
            .get_result(&client)
            .unwrap();
        assert_eq!(result, None);

        assert_eq!(query_value(&client, "SELECT 0::NUMERIC"), DBig::ZERO);
        assert_eq!(query_value(&client, "SELECT 1::NUMERIC"), DBig::ONE);
        assert_eq!(query_value(&client, "SELECT '-1'::NUMERIC"), DBig::NEG_ONE);

        assert_eq!(write_value(&client, "SELECT $1::NUMERIC", &DBig::ZERO), DBig::ZERO);
        assert_eq!(write_value(&client, "SELECT $1::NUMERIC", &DBig::ONE), DBig::ONE);
        assert_eq!(write_value(&client, "SELECT $1::NUMERIC", &DBig::NEG_ONE), DBig::NEG_ONE);
    }

    #[test]
    #[ignore]
    fn test_normal_values() {
        let client = PgConnection::establish(&get_postgres_url()).unwrap();

        for (precision, scale, string, decimal) in get_test_cases() {
            let result = query_value(
                &client,
                &format!("SELECT {}::NUMERIC({}, {})", string, precision, scale),
            );
            assert_eq!(result, decimal);
            assert_eq!(result.precision(), decimal.precision());

            let result = write_value(
                &client,
                &format!("SELECT $1::NUMERIC({}, {})", precision, scale),
                &decimal,
            );
            assert_eq!(result, decimal);
            assert_eq!(result.precision(), decimal.precision());
        }
    }
}

mod diesel_v2 {
    use super::*;
    use ::diesel_v2::{
        self as diesel,
        deserialize::QueryableByName,
        pg::{Pg, PgConnection},
        query_dsl::RunQueryDsl,
        row::NamedRow,
        Connection,
    };

    #[derive(Debug, PartialEq)]
    struct TestRow(pub DBig);

    impl QueryableByName<Pg> for TestRow {
        fn build<'a>(row: &impl NamedRow<'a, Pg>) -> diesel::deserialize::Result<Self> {
            let number: DBig = NamedRow::get(row, "numeric")?;
            Ok(Self(number))
        }
    }

    // execute query and get the result as a single decimal number
    fn query_value(client: &mut PgConnection, query: &str) -> DBig {
        let result: Option<TestRow> = diesel::sql_query(query).get_result(client).unwrap();
        result.unwrap().0
    }

    // execute query with param `value`, and get the result as a single decimal number
    fn write_value(client: &mut PgConnection, query: &str, value: &DBig) -> DBig {
        let result: Option<TestRow> = diesel::sql_query(query)
            .bind(value)
            .get_result(client)
            .unwrap();
        result.unwrap().0
    }

    #[test]
    #[ignore]
    fn test_special_values() {
        let mut client = PgConnection::establish(&get_postgres_url()).unwrap();

        let result: Option<TestRow> = diesel::sql_query("SELECT NULL::NUMERIC")
            .get_result(&mut client)
            .unwrap();
        assert_eq!(result, None);

        assert_eq!(query_value(&mut client, "SELECT 0::NUMERIC"), DBig::ZERO);
        assert_eq!(query_value(&mut client, "SELECT 1::NUMERIC"), DBig::ONE);
        assert_eq!(query_value(&mut client, "SELECT '-1'::NUMERIC"), DBig::NEG_ONE);

        assert_eq!(write_value(&mut client, "SELECT $1::NUMERIC", &DBig::ZERO), DBig::ZERO);
        assert_eq!(write_value(&mut client, "SELECT $1::NUMERIC", &DBig::ONE), DBig::ONE);
        assert_eq!(write_value(&mut client, "SELECT $1::NUMERIC", &DBig::NEG_ONE), DBig::NEG_ONE);
    }

    #[test]
    #[ignore]
    fn test_normal_values() {
        let mut client = PgConnection::establish(&get_postgres_url()).unwrap();

        for (precision, scale, string, decimal) in get_test_cases() {
            let result = query_value(
                &mut client,
                &format!("SELECT {}::NUMERIC({}, {})", string, precision, scale),
            );
            assert_eq!(result, decimal);
            assert_eq!(result.precision(), decimal.precision());

            let result = write_value(
                &mut client,
                &format!("SELECT $1::NUMERIC({}, {})", precision, scale),
                &decimal,
            );
            assert_eq!(result, decimal);
            assert_eq!(result.precision(), decimal.precision());
        }
    }
}
