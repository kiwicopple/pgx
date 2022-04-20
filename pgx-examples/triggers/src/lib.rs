/*
Portions Copyright 2019-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the MIT license that can be found in the LICENSE file.
*/
use pgx::*;

pg_module_magic!();

/// ```sql
/// CREATE OR REPLACE FUNCTION trigger_example()
///            RETURNS TRIGGER
///            LANGUAGE c
///            AS 'MODULE_PATHNAME', 'trigger_example_wrapper';
/// ```
#[pg_extern]
unsafe fn trigger_example(fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    // we can only be called as a trigger
    if !called_as_trigger(fcinfo) {
        panic!("not called by trigger manager");
    }

    let trigdata = (fcinfo.as_ref().expect("fcinfo is NULL").context as *mut pg_sys::TriggerData)
        .as_ref()
        .unwrap();

    // and for this example, we're only going to operate as an ON BEFORE INSERT FOR EACH ROW trigger
    if trigger_fired_before(trigdata.tg_event)
        && trigger_fired_by_insert(trigdata.tg_event)
        && trigger_fired_for_row(trigdata.tg_event)
    {
        let tuple =
            PgHeapTuple::from_trigger_data(trigdata, TriggerTuple::Current).expect("tuple is NULL");
        let id = tuple
            .get_by_index::<i64>(1.try_into().unwrap())
            .expect("could not get id");
        let title = tuple
            .get_by_index::<String>(2.try_into().unwrap())
            .expect("could not get title");
        let description = tuple
            .get_by_index::<String>(3.try_into().unwrap())
            .expect("could not get description");
        let payload = tuple
            .get_by_index::<JsonB>(4.try_into().unwrap())
            .expect("could not get payload");

        warning!(
            "id={:?}, title={:?}, description={:?}, payload={:?}",
            id,
            title,
            description,
            payload
        );

        // change the title
        let mut tuple = tuple.into_owned();
        tuple
            .set_by_name("title", "a new title")
            .expect("failed to change the title");
        assert_eq!(
            tuple.get_by_name::<&str>("title").unwrap().unwrap(),
            "a new title"
        );

        // return the inserting tuple, which includes the changed title
        match tuple.into_datum() {
            Some(datum) => datum,
            None => return pg_return_null(fcinfo),
        }
    } else {
        panic!("not fired in the ON BEFORE INSERT context");
    }
}

extension_sql!(
    r#"
CREATE TABLE test (
    id serial8 NOT NULL PRIMARY KEY,
    title varchar(50),
    description text,
    payload jsonb
);

CREATE TRIGGER test_trigger BEFORE INSERT ON test FOR EACH ROW EXECUTE PROCEDURE trigger_example();
INSERT INTO test (title, description, payload) VALUES ('the title', 'a description', '{"key": "value"}');

"#,
    name = "create_trigger",
);

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgx::*;

    #[pg_test]
    fn test_insert() {
        Spi::run(
            r#"INSERT INTO test (title, description, payload) VALUES ('a different title', 'a different description', '{"key": "value"}')"#,
        );
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
