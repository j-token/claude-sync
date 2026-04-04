pub mod config_cmd;
pub mod diff;
pub mod doctor;
pub mod init;
pub mod pull;
pub mod push;
pub mod restore;
pub mod secret;
pub mod skill;
pub mod status;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "claude-sync",
    about = "Sync Claude Code configuration across devices via GitHub",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Setup wizard: configure repo, auth, and sync options
    Init,

    /// Push local configuration to remote repository
    Push {
        /// Show what would be pushed without actually pushing
        #[arg(long)]
        dry_run: bool,
    },

    /// Pull remote configuration to local
    Pull {
        /// Overwrite local changes with remote
        #[arg(long)]
        force: bool,

        /// Show what would be pulled without actually pulling
        #[arg(long)]
        dry_run: bool,
    },

    /// Show current sync status
    Status,

    /// Show diff between local and remote configuration
    Diff {
        /// Show diff for a specific file only
        #[arg(long)]
        file: Option<String>,
    },

    /// Manage sync configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Manage skills sync
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },

    /// Manage secret patterns
    Secret {
        #[command(subcommand)]
        action: SecretAction,
    },

    /// Restore from a previous snapshot
    Restore {
        /// Restore the most recent snapshot
        #[arg(long)]
        latest: bool,

        /// List available snapshots
        #[arg(long)]
        list: bool,
    },

    /// Diagnose configuration and connectivity issues
    Doctor,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        key: String,
        value: String,
    },
    /// Open configuration in editor
    Edit,
}

#[derive(Subcommand)]
pub enum SkillAction {
    /// List all skills with sync status
    List,
    /// Push specific skills to remote (all if no names given)
    Push {
        /// Skill names to push
        names: Vec<String>,
    },
    /// Pull specific skills from remote (all if no names given)
    Pull {
        /// Skill names to pull
        names: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum SecretAction {
    /// List detected secrets
    List,
    /// Add a custom secret pattern
    Add {
        /// Name for the pattern
        name: String,
        /// JSONPath pattern (e.g., "mcpServers.*.env.*_KEY")
        json_path: String,
    },
    /// Remove a secret pattern
    Remove {
        /// Name of the pattern to remove
        name: String,
    },
}

pub async fn execute(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init => init::run().await,
        Commands::Push { dry_run } => push::run(dry_run).await,
        Commands::Pull { force, dry_run } => pull::run(force, dry_run).await,
        Commands::Status => status::run().await,
        Commands::Diff { file } => diff::run(file).await,
        Commands::Config { action } => config_cmd::run(action).await,
        Commands::Skill { action } => skill::run(action).await,
        Commands::Secret { action } => secret::run(action).await,
        Commands::Restore { latest, list } => restore::run(latest, list).await,
        Commands::Doctor => doctor::run().await,
    }
}
