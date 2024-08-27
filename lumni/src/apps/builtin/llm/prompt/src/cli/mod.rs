mod select_profile;
mod subcommands;

use clap::{Arg, Command};
use lumni::api::spec::ApplicationSpec;
pub use select_profile::handle_profile_selection;
use subcommands::db::create_db_subcommand;
pub use subcommands::db::handle_db_subcommand;
use subcommands::profile::create_profile_subcommand;
pub use subcommands::profile::handle_profile_subcommand;

use super::chat::db::{
    ConversationDatabase, EncryptionHandler, MaskMode, ModelSpec, UserProfile,
    UserProfileDbHandler,
};
use super::server::{ModelServer, ServerTrait, SUPPORTED_MODEL_ENDPOINTS};
use crate::external as lumni;

pub fn parse_cli_arguments(spec: ApplicationSpec) -> Command {
    let name = Box::leak(spec.name().into_boxed_str()) as &'static str;
    let version = Box::leak(spec.version().into_boxed_str()) as &'static str;
    Command::new(name)
        .version(version)
        .about("CLI for prompt interaction")
        .arg_required_else_help(false)
        .subcommand(create_db_subcommand())
        .subcommand(create_profile_subcommand())
        .arg(
            Arg::new("profile")
                .long("profile")
                .short('p')
                .help("Select a profile (format: name, name::id, or ::id)"),
        )
        .arg(
            Arg::new("system")
                .long("system")
                .short('s')
                .help("System prompt"),
        )
        .arg(
            Arg::new("assistant")
                .long("assistant")
                .short('a')
                .help("Specify an assistant to use"),
        )
        .arg(Arg::new("options").long("options").short('o').help(
            "Comma-separated list of model options e.g., \
             temperature=1,max_tokens=100",
        ))
}
