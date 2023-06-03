use actionkv::{ActionKV, Cli, Commands, Parser};

fn main() {
    let cli = Cli::parse();
    let path = std::path::Path::new(&cli.file);
    let mut store = ActionKV::open(path).expect("unable to open file");
    store.load().expect("unable to load data");

    match &cli.command {
        Commands::Get(args) => match store.get(args.key.as_bytes()).unwrap() {
            None => eprintln!("{:?} not found", args.key),
            Some(value) => println!("{:?}", value),
        },
        Commands::Delete(args) => store.delete(args.key.as_bytes()).unwrap(),
        Commands::Insert(args) => store
            .insert(args.key.as_bytes(), args.value.as_bytes())
            .unwrap(),
        Commands::Update(args) => store
            .update(args.key.as_bytes(), args.value.as_bytes())
            .unwrap(),
    }
}
