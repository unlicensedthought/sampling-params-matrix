use sampling_params_matrix::{temperature_plan, Target};

const USAGE: &str = "usage: sampling-params-matrix <model> --builtin [--thinking on|off] [--temp T]\n       sampling-params-matrix <model> --endpoint <base_url> [--temp T]\n\nPrints the temperature plan as one JSON line.";

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}\n{USAGE}");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(model) = args.first().filter(|a| !a.starts_with("--")) else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let temp: f64 = match flag("--temp").map(|t| t.parse()) {
        None => 0.2,
        Some(Ok(t)) => t,
        Some(Err(_)) => die("--temp is not a number"),
    };
    let thinking = match flag("--thinking").as_deref() {
        None | Some("on") => true,
        Some("off") => false,
        Some(other) => die(&format!("--thinking takes on or off, got {other}")),
    };
    let base = flag("--endpoint");
    let target = match (&base, args.iter().any(|a| a == "--builtin")) {
        (Some(b), false) => Target::Endpoint { base_url: b },
        (None, true) => Target::Builtin { thinking },
        _ => die("give exactly one of --builtin or --endpoint <base_url>"),
    };
    println!("{}", temperature_plan(model, target).to_json(temp));
}
