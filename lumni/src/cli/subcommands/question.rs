use clap::{Arg, Command};

pub fn question_subcommand() -> Command {
    Command::new("-q")
        .long_flag("question")
        .about("Ask a question or use the prompt app")
        .trailing_var_arg(true)
        .allow_hyphen_values(true)
        .arg(
            Arg::new("question")
                .required(false)
                .num_args(1..)
                .help("The question or prompt"),
        )
}
