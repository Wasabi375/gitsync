use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::PathBuf,
};

use clap::{Args, Parser, Subcommand};

use gitsync::{Object, ObjectType, Repository};

// TODO error handling

#[derive(Debug, Parser)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// creates a new repository
    Init(InitArgs),

    /// Finds the repository for workdir and prints its path
    Find,

    /// Cats the specified file to std-out
    CatFile(CatFileArgs),

    /// Hashes the given object and prints the sha1-hash
    HashObject(HashObjectArgs),
}

#[derive(Debug, Args)]
struct InitArgs {
    path: PathBuf,
}

#[derive(Debug, Args)]
struct CatFileArgs {
    #[arg(value_enum, name = "type")]
    typ: ObjectType,
    object: String,
}

#[derive(Debug, Args)]
struct HashObjectArgs {
    #[arg(required_unless_present("stdin"))]
    file: Option<PathBuf>,

    #[arg(long)]
    stdin: bool,

    #[arg(value_enum, name = "type", long, short, default_value = "blob")]
    typ: ObjectType,

    #[arg(long, short)]
    write: bool,
}

fn main() {
    let args = Arguments::parse();

    match args.command {
        Command::Init(args) => init(args),
        Command::Find => find(),
        Command::CatFile(args) => cat_file(args),
        Command::HashObject(args) => hash_object(args),
    }
}

fn find_repo() -> Repository {
    Repository::find().unwrap()
}

fn init(args: InitArgs) {
    Repository::create_at(args.path).unwrap();
}

fn find() {
    let repo = find_repo();
    println!("Git repository at: {}", repo.worktree_root().display());
}

fn cat_file(args: CatFileArgs) {
    let repo = find_repo();
    let sha1 = repo.find_object_sha1(&args.object, None, true);
    let obj_path = Repository::sha1_to_object(&sha1);

    let mut open_opts = OpenOptions::new();
    open_opts.read(true);

    let file = repo.file(obj_path, &open_opts, false).unwrap();

    let obj = Object::deserialize_zlib_read(file).unwrap();
    obj.serialize(&mut std::io::stdout()).unwrap();
}

fn hash_object(args: HashObjectArgs) {
    let mut input: Box<dyn Read> = if args.stdin {
        Box::new(std::io::stdin())
    } else {
        Box::new(File::open(args.file.unwrap()).unwrap())
    };

    let obj = Object::deserialize_read(args.typ, &mut input).unwrap();

    if args.write {
        let repo = Repository::find().unwrap();
        let sha1 = obj.save(&repo).unwrap();
        println!("{}", sha1);
    } else {
        println!("{}", obj.sha1());
    }
}
