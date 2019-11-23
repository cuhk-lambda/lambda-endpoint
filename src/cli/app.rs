use clap::*;

fn get_matches<'a>() -> ArgMatches<'a> {
    App::new("lambda-endpoint")
        .subcommand(SubCommand::with_name("endpoint")
            .arg(Arg::with_name("config").short("c").long("config")
                .help("path to the configuration").default_value("/home/schrodinger/CLionProject/lambda_endpoint/target/debug/test.toml")))
        .get_matches()
}

lazy_static! {
    static ref MATCHES : ArgMatches<'static> = get_matches();
    pub static ref SUB_COMMAND : (&'static str, &'static ArgMatches<'static>) = {
        let (a, b) = MATCHES.subcommand();
        if b.is_none() {
            eprintln!("you must enter a subcommand, use --help to see details");
            std::process::exit(1);
        }
        (a, b.unwrap())
    };
}

fn init_config() -> &'static str {
    SUB_COMMAND.1.value_of("config").unwrap()
}

lazy_static! {
    static ref CONFIG : &'static str = init_config();
}

pub fn config() -> &'static str {
    *CONFIG
}