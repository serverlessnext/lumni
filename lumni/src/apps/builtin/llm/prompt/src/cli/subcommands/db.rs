use std::sync::Arc;

use clap::{Arg, ArgAction, ArgMatches, Command};
use lumni::api::error::ApplicationError;

use super::ConversationDatabase;
use crate::external as lumni;

pub fn create_db_subcommand() -> Command {
    Command::new("db")
        .about("Manage and query the conversation database")
        .subcommand(create_path_subcommand())
        .subcommand(create_list_subcommand())
        .subcommand(create_show_subcommand())
        .subcommand(create_truncate_subcommand())
}

fn create_path_subcommand() -> Command {
    Command::new("path").about("Show the path to the SQLite database file")
}

fn create_list_subcommand() -> Command {
    Command::new("list").about("List recent conversations").arg(
        Arg::new("limit")
            .short('n')
            .long("limit")
            .help("Number of conversations to list")
            .value_name("LIMIT")
            .default_value("20"),
    )
}

fn create_show_subcommand() -> Command {
    Command::new("show")
        .about("Show details of a specific conversation")
        .arg(Arg::new("id").help("ID of the conversation to show"))
}

fn create_truncate_subcommand() -> Command {
    Command::new("truncate")
        .about("Truncate all tables and vacuum the database")
        .arg(
            Arg::new("confirm")
                .short('y')
                .long("yes")
                .help("Confirm the truncation without prompting")
                .action(ArgAction::SetTrue),
        )
}

pub async fn handle_db_subcommand(
    db_matches: &ArgMatches,
    db_conn: &Arc<ConversationDatabase>,
) -> Result<(), ApplicationError> {
    match db_matches.subcommand() {
        Some(("path", _)) => {
            let filepath = ConversationDatabase::get_filepath();
            println!("SQLite database filepath: {:?}", filepath);
            Ok(())
        }
        Some(("list", list_matches)) => {
            let limit: usize = list_matches
                .get_one::<String>("limit")
                .unwrap()
                .parse()
                .unwrap_or(20);
            db_conn.print_conversation_list(limit).await
        }
        Some(("show", show_matches)) => {
            if let Some(id) = show_matches.get_one::<String>("id") {
                db_conn.print_conversation_by_id(id).await
            } else {
                create_show_subcommand().print_help()?;
                Ok(())
            }
        }
        Some(("truncate", truncate_matches)) => {
            if truncate_matches.get_flag("confirm") {
                db_conn.truncate_and_vacuum().await?;
                println!(
                    "Database tables truncated and vacuumed successfully."
                );
            } else {
                println!(
                    "Are you sure you want to truncate all tables and vacuum \
                     the database? This action cannot be undone."
                );
                println!("Type 'yes' to confirm or any other input to cancel:");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim().eq_ignore_ascii_case("yes") {
                    db_conn.truncate_and_vacuum().await?;
                    println!(
                        "Database tables truncated and vacuumed successfully."
                    );
                } else {
                    println!("Operation cancelled.");
                }
            }
            Ok(())
        }
        _ => {
            create_db_subcommand().print_help()?;
            Ok(())
        }
    }
}
