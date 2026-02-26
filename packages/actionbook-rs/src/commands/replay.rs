use colored::Colorize;

use super::record::RecordedScenario;
use crate::browser::BrowserDriver;
use crate::cli::Cli;
use crate::error::{ActionbookError, Result};

pub async fn run(cli: &Cli, file: &str, dry_run: bool) -> Result<()> {
    // Read and parse scenario file
    let content = std::fs::read_to_string(file)
        .map_err(|e| ActionbookError::Other(format!("Failed to read '{}': {}", file, e)))?;

    let scenario: RecordedScenario = serde_json::from_str(&content)?;

    if !cli.json {
        println!(
            "{} {} ({} steps)",
            if dry_run {
                "Dry run:".yellow().bold()
            } else {
                "Replaying:".cyan().bold()
            },
            scenario.url,
            scenario.steps.len()
        );
        println!("{}", "-".repeat(50).dimmed());
    }

    let mut driver = if !dry_run {
        let mut d = BrowserDriver::from_cli(cli).await?;
        d.goto(&scenario.url).await?;
        // Wait for page to settle
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        Some(d)
    } else {
        None
    };

    let mut results = Vec::new();
    let mut success_count = 0u32;

    for (i, step) in scenario.steps.iter().enumerate() {
        let step_num = i + 1;

        if dry_run {
            if !cli.json {
                let value_str = step
                    .value
                    .as_deref()
                    .map(|v| format!(" \"{}\"", v))
                    .unwrap_or_default();
                println!(
                    "  {}. {} {} {}",
                    step_num,
                    step.action.green(),
                    step.selector.yellow(),
                    value_str.dimmed()
                );
            }
            results.push(serde_json::json!({
                "step": step_num,
                "action": step.action,
                "selector": step.selector,
                "value": step.value,
                "status": "skipped",
            }));
            success_count += 1;
            continue;
        }

        let driver = driver.as_mut().unwrap();

        let exec_result = match step.action.as_str() {
            "click" | "submit" => driver.click(&step.selector).await,
            "fill" => {
                if let Some(ref val) = step.value {
                    driver.fill(&step.selector, val).await
                } else {
                    Err(ActionbookError::ElementActionFailed(
                        step.selector.clone(),
                        "fill".to_string(),
                        "No value provided".to_string(),
                    ))
                }
            }
            "type" => {
                if let Some(ref val) = step.value {
                    driver.type_text(&step.selector, val).await
                } else {
                    Err(ActionbookError::ElementActionFailed(
                        step.selector.clone(),
                        "type".to_string(),
                        "No value provided".to_string(),
                    ))
                }
            }
            "select" => {
                if let Some(ref val) = step.value {
                    driver.select(&step.selector, val).await
                } else {
                    Err(ActionbookError::ElementActionFailed(
                        step.selector.clone(),
                        "select".to_string(),
                        "No value provided".to_string(),
                    ))
                }
            }
            other => Err(ActionbookError::ElementActionFailed(
                step.selector.clone(),
                other.to_string(),
                "Unknown action".to_string(),
            )),
        };

        let (status, error_msg) = match &exec_result {
            Ok(()) => {
                success_count += 1;
                ("success".to_string(), None)
            }
            Err(e) => ("failed".to_string(), Some(e.to_string())),
        };

        if !cli.json {
            let icon = if exec_result.is_ok() {
                "[ok]".green()
            } else {
                "[fail]".red()
            };
            let value_str = step
                .value
                .as_deref()
                .map(|v| format!(" \"{}\"", v))
                .unwrap_or_default();
            println!(
                "{} {}. {} {} {}",
                icon,
                step_num,
                step.action,
                step.selector,
                value_str.dimmed()
            );
            if let Some(ref msg) = error_msg {
                println!("     {}", msg.red());
            }
        }

        results.push(serde_json::json!({
            "step": step_num,
            "action": step.action,
            "selector": step.selector,
            "status": status,
            "error": error_msg,
        }));

        // Small delay between steps
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let total = scenario.steps.len() as u32;

    if cli.json {
        println!(
            "{}",
            serde_json::json!({
                "success": success_count == total,
                "url": scenario.url,
                "total_steps": total,
                "successful": success_count,
                "failed": total - success_count,
                "dry_run": dry_run,
                "steps": results,
            })
        );
    } else {
        println!();
        let summary = format!("Result: {}/{} steps succeeded", success_count, total);
        if success_count == total {
            println!("{}", summary.green().bold());
        } else {
            println!("{}", summary.yellow().bold());
        }
    }

    Ok(())
}
