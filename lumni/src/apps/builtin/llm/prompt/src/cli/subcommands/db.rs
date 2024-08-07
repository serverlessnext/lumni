use std::sync::Arc;

use clap::{Arg, ArgMatches, Command};
use lumni::api::error::ApplicationError;

use super::ConversationDatabase;
use crate::external as lumni;

pub fn create_db_subcommand() -> Command {
    Command::new("db")
        .about("Query the conversation database")
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .help("List recent conversations")
                .num_args(0..=1)
                .value_name("LIMIT"),
        )
        .arg(
            Arg::new("id")
                .long("id")
                .short('i')
                .help("Fetch a specific conversation by ID")
                .num_args(1),
        )
}

pub async fn handle_db_subcommand(
    db_matches: &ArgMatches,
    db_conn: &Arc<ConversationDatabase>,
) -> Result<(), ApplicationError> {
    if db_matches.contains_id("list") {
        let limit = match db_matches.get_one::<String>("list") {
            Some(value) => value.parse().unwrap_or(20),
            None => 20,
        };
        db_conn.print_conversation_list(limit).await
    } else if let Some(id_value) = db_matches.get_one::<String>("id") {
        db_conn.print_conversation_by_id(id_value).await
    } else {
        db_conn.print_last_conversation().await
    }
}
