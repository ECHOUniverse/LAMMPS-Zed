pub mod schema;

// Include the generated database
include!(concat!(env!("OUT_DIR"), "/command_database.rs"));
