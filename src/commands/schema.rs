use crate::args::SchemaCommand;
use crate::commands::BucketCommand;
use crate::errors::BucketError;

const SCHEMA_SQL: &str = include_str!("../sql/schema.sql");

pub struct Schema {
    #[allow(dead_code)]
    args: SchemaCommand,
}

impl BucketCommand for Schema {
    type Args = SchemaCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        println!("{}", SCHEMA_SQL.trim_end());
        Ok(())
    }
}
