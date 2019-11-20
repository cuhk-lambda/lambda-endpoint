use clap::*;

fn get_matches<'a>() -> ArgMatches<'a> {
    App::new("lambda-endpoint")
        .arg(Arg::with_name("config").short("c").long("config")
            .help("path to the configuration").default_value("/home/schrodinger/CLionProject/lambda_endpoint/target/debug/test.toml"))
        .get_matches()
}

lazy_static!{
    static ref MATCHES : ArgMatches<'static> = get_matches();
}

fn init_config() -> &'static str {
    MATCHES.value_of("config").unwrap()
}

lazy_static!{
    static ref CONFIG : &'static str = init_config();
}

pub fn config() -> &'static str {
    *CONFIG
}