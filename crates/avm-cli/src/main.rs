mod cli;
mod config;
mod lockfile;
mod resolver;
mod runtime;
mod shims;
mod trust;
mod ui;
mod update;
mod version_files;

fn main() {
    cli::main()
}
