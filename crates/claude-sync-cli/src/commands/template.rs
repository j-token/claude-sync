use anyhow::Result;
use console::style;

use claude_sync_core::app::App;
use claude_sync_core::template::{self, Template};

use super::{TemplateAction, ui};

pub fn run(action: TemplateAction) -> Result<()> {
    match action {
        TemplateAction::List => {
            ui::title("템플릿");
            for t in template::list()? {
                let badge = if t.builtin { style(" [기본]").dim().to_string() } else { String::new() };
                println!("  {}{}  {}", style(&t.name).bold(), badge, style(&t.description).dim());
                println!("      포함: {}", t.include.join(", "));
                if !t.exclude.is_empty() {
                    println!("      제외: {}", t.exclude.join(", "));
                }
            }
        }
        TemplateAction::Show { name } => {
            let t = template::find(&name)?;
            let app = App::load()?;
            let inventory = app.inventory()?;
            let selected = t.select(inventory.items.iter().map(|i| &i.id));
            ui::title(&format!("템플릿 '{}' — 로컬 항목 {}개", t.name, selected.len()));
            for id in selected {
                println!("  {id}");
            }
        }
        TemplateAction::Create { name, include, exclude, description } => {
            let saved = template::save(&Template {
                name,
                description,
                include,
                exclude,
                builtin: false,
            })?;
            ui::ok(format!("템플릿 '{}' 저장", saved.name));
        }
        TemplateAction::Delete { name } => {
            template::delete(&name)?;
            ui::ok(format!("템플릿 '{name}' 삭제"));
        }
    }
    Ok(())
}
