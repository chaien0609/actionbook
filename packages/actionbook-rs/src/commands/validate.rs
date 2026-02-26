use std::time::{SystemTime, UNIX_EPOCH};

use colored::Colorize;
use serde::Serialize;

use crate::api::ApiClient;
use crate::browser::BrowserDriver;
use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub area_id: String,
    pub url: String,
    pub timestamp: String,
    pub overall_health: f64,
    pub elements: Vec<ElementValidation>,
}

#[derive(Debug, Serialize)]
pub struct ElementValidation {
    pub element_id: String,
    pub css_selector: Option<String>,
    pub xpath_selector: Option<String>,
    pub css_found: bool,
    pub xpath_found: bool,
    pub visible: bool,
    pub interactive: bool,
    pub status: String,
}

pub async fn run(cli: &Cli, area_id: &str, report: bool) -> Result<()> {
    // 1. Fetch structured area data
    let mut config = Config::load()?;
    if let Some(ref key) = cli.api_key {
        config.api.api_key = Some(key.clone());
    }
    let client = ApiClient::from_config(&config)?;

    let detail = client.get_action_by_area_id_json(area_id).await?;

    // 2. Get browser driver and navigate to page
    let mut driver = BrowserDriver::from_cli(cli).await?;

    let url = detail.url.as_deref().unwrap_or("");
    if !url.is_empty() {
        driver.goto(url).await?;
        // Wait a moment for page to settle
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    // 3. Validate each element
    if !cli.json {
        println!("{} {}", "Validating:".cyan().bold(), area_id);
        println!("{}", "-".repeat(50).dimmed());
    }

    let mut element_validations = Vec::new();
    let mut healthy_count = 0u32;
    let total = detail.elements.len() as u32;

    let mut sorted_elements: Vec<_> = detail.elements.iter().collect();
    sorted_elements.sort_by_key(|(id, _)| id.to_string());

    for (element_id, element) in &sorted_elements {
        let mut css_found = false;
        let mut xpath_found = false;
        let mut visible = false;
        let mut interactive = false;

        // Check CSS selector
        if let Some(ref css) = element.css_selector {
            let js = format!(
                r#"(() => {{
                    const el = document.querySelector({});
                    if (!el) return JSON.stringify({{ found: false }});
                    const rect = el.getBoundingClientRect();
                    const style = window.getComputedStyle(el);
                    const isVisible = rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
                    const isInteractive = !el.disabled && el.getAttribute('aria-hidden') !== 'true';
                    return JSON.stringify({{ found: true, visible: isVisible, interactive: isInteractive }});
                }})()"#,
                serde_json::to_string(css).unwrap_or_else(|_| format!("\"{}\"", css))
            );

            if let Ok(result) = driver.execute_js(&js).await {
                if let Ok(info) = serde_json::from_str::<serde_json::Value>(&result) {
                    css_found = info.get("found").and_then(|v| v.as_bool()).unwrap_or(false);
                    if css_found {
                        visible = info.get("visible").and_then(|v| v.as_bool()).unwrap_or(false);
                        interactive = info
                            .get("interactive")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                    }
                }
            }
        }

        // Check XPath selector if CSS not found
        if !css_found {
            if let Some(ref xpath) = element.xpath_selector {
                let js = format!(
                    r#"(() => {{
                        const result = document.evaluate({}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                        const el = result.singleNodeValue;
                        if (!el) return JSON.stringify({{ found: false }});
                        const rect = el.getBoundingClientRect();
                        const style = window.getComputedStyle(el);
                        const isVisible = rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
                        const isInteractive = !el.disabled && el.getAttribute('aria-hidden') !== 'true';
                        return JSON.stringify({{ found: true, visible: isVisible, interactive: isInteractive }});
                    }})()"#,
                    serde_json::to_string(xpath).unwrap_or_else(|_| format!("\"{}\"", xpath))
                );

                if let Ok(result) = driver.execute_js(&js).await {
                    if let Ok(info) = serde_json::from_str::<serde_json::Value>(&result) {
                        xpath_found =
                            info.get("found").and_then(|v| v.as_bool()).unwrap_or(false);
                        if xpath_found {
                            visible = info
                                .get("visible")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            interactive = info
                                .get("interactive")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                        }
                    }
                }
            }
        }

        let found = css_found || xpath_found;
        let status = if found && visible && interactive {
            "healthy".to_string()
        } else if found {
            "degraded".to_string()
        } else {
            "broken".to_string()
        };

        if found && visible && interactive {
            healthy_count += 1;
        }

        let selector_display = element
            .css_selector
            .as_deref()
            .or(element.xpath_selector.as_deref())
            .unwrap_or("(none)");

        if !cli.json {
            let icon = if found { "+" } else { "x" };
            let status_text = if found && visible && interactive {
                "found, visible, interactive".green().to_string()
            } else if found && visible {
                "found, visible, not interactive".yellow().to_string()
            } else if found {
                "found, hidden".yellow().to_string()
            } else {
                "NOT FOUND".red().to_string()
            };

            println!(
                "[{}] {:<16} {:<30} -> {}",
                icon,
                element_id.yellow(),
                selector_display.dimmed(),
                status_text
            );
        }

        element_validations.push(ElementValidation {
            element_id: element_id.to_string(),
            css_selector: element.css_selector.clone(),
            xpath_selector: element.xpath_selector.clone(),
            css_found,
            xpath_found,
            visible,
            interactive,
            status,
        });
    }

    let overall_health = if total > 0 {
        healthy_count as f64 / total as f64
    } else {
        0.0
    };

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());

    let result = ValidationResult {
        area_id: area_id.to_string(),
        url: url.to_string(),
        timestamp,
        overall_health,
        elements: element_validations,
    };

    if cli.json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "validation": result,
            })
        );
    } else {
        println!();
        let health_pct = (overall_health * 100.0) as u32;
        let health_str = format!("Health: {}/{} ({}%)", healthy_count, total, health_pct);
        if health_pct == 100 {
            println!("{}", health_str.green().bold());
        } else if health_pct >= 50 {
            println!("{}", health_str.yellow().bold());
        } else {
            println!("{}", health_str.red().bold());
        }
    }

    // Optional: POST report to backend
    if report {
        let report_json = serde_json::to_value(&result).unwrap_or_default();
        match client.post_validation_report(&report_json).await {
            Ok(()) => {
                if !cli.json {
                    println!("\n{}", "Report submitted to backend.".green());
                }
            }
            Err(e) => {
                if cli.json {
                    eprintln!(
                        "{}",
                        serde_json::json!({
                            "success": false,
                            "error": {
                                "code": "report_submission_failed",
                                "message": e.to_string(),
                            }
                        })
                    );
                } else {
                    eprintln!("{} {}", "Warning: failed to submit report:".yellow(), e);
                }
            }
        }
    }

    Ok(())
}
