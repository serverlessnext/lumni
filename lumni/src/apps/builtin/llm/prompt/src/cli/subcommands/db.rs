use std::sync::Arc;

use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use lumni::api::error::ApplicationError;

use super::ConversationDatabase;
use crate::external as lumni;

pub fn create_db_subcommand() -> Command {
    Command::new("db")
        .about("Query the conversation database")
        .arg(
            Arg::new("path")
                .long("path")
                .short('p')
                .help("Path to the SQLite database file")
                .action(ArgAction::SetTrue)
                .value_name("FILE"),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .help("List recent conversations")
                .num_args(0..=1)
                .value_name("LIMIT"),
        )
        .arg(
            Arg::new("last")
                .long("last")
                .short('L')
                .help("Print the last conversation")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("id")
                .long("id")
                .short('i')
                .help("Fetch a specific conversation by ID")
                .num_args(1),
        )
        .arg(
            Arg::new("truncate")
                .long("truncate")
                .help("Truncate all tables and vacuum the database")
                .action(ArgAction::SetTrue),
        )
        .group(
            ArgGroup::new("db_group")
                .args(["path", "list", "last", "id", "truncate"])
                .required(false)
                .multiple(false),
        )
}

pub async fn handle_db_subcommand(
    db_matches: &ArgMatches,
    db_conn: &Arc<ConversationDatabase>,
) -> Result<(), ApplicationError> {
    if db_matches.get_flag("truncate") {
        let result = db_conn.truncate_and_vacuum().await;
        if result.is_ok() {
            println!("Database tables truncated");
        }
        result
    } else if db_matches.get_flag("path") {
        let filepath = ConversationDatabase::get_filepath();
        println!("Sqlite filepath: {:?}", filepath);
        Ok(())
    } else if db_matches.contains_id("list") {
        let limit = match db_matches.get_one::<String>("list") {
            Some(value) => value.parse().unwrap_or(20),
            None => 20,
        };
        db_conn.print_conversation_list(limit).await
    } else if let Some(id_value) = db_matches.get_one::<String>("id") {
        db_conn.print_conversation_by_id(id_value).await
    } else if db_matches.get_flag("last") {
        // If any arguments are present but not handled above, print the last conversation
        db_conn.print_last_conversation().await
    } else {
        // If no arguments are provided, print the help message
        let mut db_command = create_db_subcommand();
        db_command.print_help()?;
        Ok(())
    }
}
