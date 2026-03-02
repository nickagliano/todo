mod formatter;
mod hooks;
mod reading;
mod serve;
mod storage;
mod task;

use anyhow::Result;
use clap::{Parser, Subcommand};

use formatter::{Formatter, PlainText};
use storage::{JsonStorage, Storage};

#[derive(Parser)]
#[command(name = "simple_todo", about = "A minimal todo list harness")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new task
    Add {
        /// Task description
        text: String,
    },
    /// List all tasks
    List,
    /// Mark a task as done
    Done {
        /// Task ID
        id: u32,
    },
    /// Delete a task
    Delete {
        /// Task ID
        id: u32,
    },
    /// Start an HTTP API server (used by epc deploy)
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8765")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Commands::Serve { port } = cli.command {
        return serve::run(port).await;
    }

    // PORT: swap JsonStorage for any type that implements Storage
    let storage = JsonStorage::default();

    // PORT: swap PlainText for any type that implements Formatter
    let formatter = PlainText;

    let mut tasks = storage.load()?;

    match cli.command {
        Commands::Add { text } => {
            let id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            tasks.push(task::Task::new(id, text.clone()));
            storage.save(&tasks)?;
            println!("Added #{id}: {text}");
            hooks::run("on_add", &[&id.to_string(), &text])?;
        }

        Commands::List => {
            println!("{}", formatter.format(&tasks));
        }

        Commands::Done { id } => match tasks.iter_mut().find(|t| t.id == id) {
            Some(t) => {
                t.done = true;
                let text = t.text.clone();
                storage.save(&tasks)?;
                println!("Done #{id}: {text}");
                hooks::run("on_complete", &[&id.to_string(), &text])?;
            }
            None => eprintln!("No task #{id}"),
        },

        Commands::Delete { id } => match tasks.iter().position(|t| t.id == id) {
            Some(pos) => {
                let text = tasks[pos].text.clone();
                tasks.remove(pos);
                storage.save(&tasks)?;
                println!("Deleted #{id}: {text}");
                hooks::run("on_delete", &[&id.to_string(), &text])?;
            }
            None => eprintln!("No task #{id}"),
        },

        Commands::Serve { .. } => unreachable!(),
    }

    Ok(())
}
