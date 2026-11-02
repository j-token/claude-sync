pub mod backup;
pub mod config_cmd;
pub mod doctor;
pub mod export;
pub mod import;
pub mod list;
pub mod p2p;
pub mod secret;
pub mod template;
pub mod ui;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "claude-sync",
    about = "Claude Code 설정(스킬·에이전트·설정·훅·MCP 등)을 골라서 ZIP 번들이나 LAN P2P로 내보내고, 가져오고, 백업합니다",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 로컬 Claude Code 항목 목록 (항목 id는 다른 명령의 인자로 쓴다)
    List {
        /// 종류로 거르기 (skill, agent, command, rule, setting, hook, hook_script, mcp, plugin, ...)
        #[arg(long)]
        kind: Option<String>,
    },

    /// 선택한 항목을 ZIP 번들로 내보내기
    Export {
        /// 항목 id (예: skill:review setting:permissions). 생략하고 --template도 없으면 대화형으로 고른다.
        items: Vec<String>,
        /// 템플릿으로 항목 고르기 (items와 합쳐진다)
        #[arg(short, long)]
        template: Option<String>,
        /// 출력 파일 경로 (기본: ./claude-sync-<기기>-<시각>.zip)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// 번들 설명
        #[arg(short, long)]
        label: Option<String>,
    },

    /// 번들 내용과 로컬 대비 상태 보기
    Inspect {
        file: PathBuf,
    },

    /// 번들에서 항목 가져오기 (가져오기 전 바뀔 항목을 자동 백업한다)
    Import {
        file: PathBuf,
        /// 가져올 항목 id. 생략하면 대화형으로 고르거나 --all/--template을 쓴다.
        items: Vec<String>,
        /// 동일하지 않은 모든 항목 가져오기
        #[arg(long)]
        all: bool,
        /// 번들 항목 중 템플릿에 맞는 것만
        #[arg(short, long)]
        template: Option<String>,
        /// merge(기본) | overwrite | skip
        #[arg(short, long, default_value = "merge")]
        mode: String,
        /// 명령을 실행하는 항목(훅·MCP·플러그인 등) 포함 시 확인 생략
        #[arg(short, long)]
        yes: bool,
        /// 실제로 쓰지 않고 무엇이 바뀔지만 표시
        #[arg(long)]
        dry_run: bool,
    },

    /// 백업 관리
    Backup {
        #[command(subcommand)]
        action: BackupAction,
    },

    /// 템플릿 관리 (항목 id glob 패턴으로 정의한 선택 집합)
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },

    /// 같은 네트워크의 다른 기기로 P2P 공유 (페어링 코드 사용)
    Share {
        items: Vec<String>,
        #[arg(short, long)]
        template: Option<String>,
        /// 대기 포트 (기본: 설정값 47821)
        #[arg(long)]
        port: Option<u16>,
    },

    /// 다른 기기가 공유 중인 번들 받기
    Receive {
        /// 상대 주소 ip:port. 생략하면 주변 기기를 찾는다.
        address: Option<String>,
        /// 상대 화면에 표시된 페어링 코드
        #[arg(short, long)]
        code: Option<String>,
        /// 받은 뒤 바로 가져오기 단계로 넘어가기
        #[arg(long)]
        import: bool,
        #[arg(short, long, default_value = "merge")]
        mode: String,
    },

    /// 같은 네트워크에서 공유 중인 기기 찾기
    Peers {
        #[arg(long, default_value_t = 3)]
        timeout: u64,
    },

    /// 시크릿 탐지 결과와 마스킹 패턴 관리
    Secret {
        #[command(subcommand)]
        action: SecretAction,
    },

    /// claude-sync 설정 관리
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// 환경 진단
    Doctor,
}

#[derive(Subcommand)]
pub enum BackupAction {
    /// 백업 만들기 (항목/템플릿 생략 시 기본 템플릿)
    Create {
        items: Vec<String>,
        #[arg(short, long)]
        template: Option<String>,
        #[arg(short, long)]
        label: Option<String>,
    },
    /// 백업 목록
    List,
    /// 백업 내용 보기
    Show { id: String },
    /// 백업에서 복원 (항목 생략 시 전체)
    Restore {
        id: String,
        items: Vec<String>,
        #[arg(short, long)]
        yes: bool,
    },
    /// 백업 삭제
    Delete { id: String },
}

#[derive(Subcommand)]
pub enum TemplateAction {
    /// 템플릿 목록
    List,
    /// 템플릿이 고르는 로컬 항목 보기
    Show { name: String },
    /// 템플릿 만들기/덮어쓰기
    Create {
        name: String,
        /// 포함 패턴 (여러 번 지정 가능). 예: -i 'skill:*' -i setting:permissions
        #[arg(short, long = "include", required = true)]
        include: Vec<String>,
        #[arg(short, long = "exclude")]
        exclude: Vec<String>,
        #[arg(short, long, default_value = "")]
        description: String,
    },
    /// 템플릿 삭제
    Delete { name: String },
}

#[derive(Subcommand)]
pub enum SecretAction {
    /// 현재 설정에서 탐지된 시크릿과 패턴 목록
    List,
    /// 마스킹 패턴 추가 (경로 끝부분과 비교. 예: "env.*_KEY")
    Add { name: String, json_path: String },
    /// 마스킹 패턴 제거
    Remove { name: String },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// 현재 설정 보기
    Show,
    /// 값 설정 (device.name, default_template, backup.max_auto_backups, backup.backup_before_import, p2p.port, p2p.discovery)
    Set { key: String, value: String },
    /// 편집기로 열기
    Edit,
    /// 사용하는 경로 보기
    Path,
}

pub fn execute(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::List { kind } => list::run(kind.as_deref()),
        Commands::Export { items, template, output, label } => export::run(&items, template.as_deref(), output, label),
        Commands::Inspect { file } => import::inspect(&file),
        Commands::Import { file, items, all, template, mode, yes, dry_run } => import::run(import::ImportArgs {
            file,
            items,
            all,
            template,
            mode,
            yes,
            dry_run,
        }),
        Commands::Backup { action } => backup::run(action),
        Commands::Template { action } => template::run(action),
        Commands::Share { items, template, port } => p2p::share(&items, template.as_deref(), port),
        Commands::Receive { address, code, import, mode } => p2p::receive(address, code, import, &mode),
        Commands::Peers { timeout } => p2p::peers(timeout),
        Commands::Secret { action } => secret::run(action),
        Commands::Config { action } => config_cmd::run(action),
        Commands::Doctor => doctor::run(),
    }
}
