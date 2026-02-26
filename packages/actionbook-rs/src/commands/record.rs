use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::browser::BrowserDriver;
use crate::cli::Cli;
use crate::error::{ActionbookError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordedScenario {
    pub url: String,
    pub recorded_at: String,
    pub steps: Vec<RecordedStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedStep {
    pub action: String,
    pub selector: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

pub async fn run(cli: &Cli, url: &str, output: Option<&str>) -> Result<()> {
    let mut driver = BrowserDriver::from_cli(cli).await?;

    // Navigate to URL
    driver.goto(url).await?;

    // Inject recording script
    let inject_js = r#"
(() => {
    if (window.__actionbook_recording) return;
    window.__actionbook_recording = true;
    window.__actionbook_steps = [];

    function getSelector(el) {
        if (el.getAttribute('data-testid')) return `[data-testid="${el.getAttribute('data-testid')}"]`;
        if (el.id) return `#${el.id}`;
        if (el.getAttribute('aria-label')) return `[aria-label="${el.getAttribute('aria-label')}"]`;
        if (el.name) return `${el.tagName.toLowerCase()}[name="${el.name}"]`;
        // Build CSS path
        let path = [];
        let current = el;
        while (current && current !== document.body) {
            let tag = current.tagName.toLowerCase();
            if (current.id) { path.unshift(`#${current.id}`); break; }
            let parent = current.parentElement;
            if (parent) {
                let siblings = Array.from(parent.children).filter(c => c.tagName === current.tagName);
                if (siblings.length > 1) {
                    let idx = siblings.indexOf(current) + 1;
                    tag += `:nth-of-type(${idx})`;
                }
            }
            path.unshift(tag);
            current = current.parentElement;
        }
        return path.join(' > ');
    }

    function recordStep(action, el, value) {
        const step = { action, selector: getSelector(el) };
        if (value !== undefined && value !== null) step.value = value;
        window.__actionbook_steps.push(step);
        // Report via binding if available
        if (window.__actionbook_record) {
            window.__actionbook_record(JSON.stringify(step));
        }
    }

    document.addEventListener('click', (e) => {
        const el = e.target;
        if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') return; // Skip, will capture on change
        recordStep('click', el);
    }, true);

    document.addEventListener('change', (e) => {
        const el = e.target;
        if (el.tagName === 'SELECT') {
            recordStep('select', el, el.value);
        } else if (el.type === 'checkbox' || el.type === 'radio') {
            recordStep('click', el);
        } else {
            recordStep('fill', el, el.value);
        }
    }, true);

    document.addEventListener('submit', (e) => {
        const el = e.target;
        recordStep('submit', el);
    }, true);
})();
"#;

    driver.execute_js(inject_js).await?;

    if !cli.json {
        println!("{} {}", "Recording:".cyan().bold(), url);
        println!("{}", "Interact with the page in the browser.".dimmed());
        println!(
            "{}",
            "Press Ctrl+C to stop recording and save the scenario.".dimmed()
        );
        println!();
    }

    // Wait for Ctrl+C
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| ActionbookError::Other(format!("Failed to listen for Ctrl+C: {}", e)))?;

    // Collect recorded steps
    let collect_js = r#"JSON.stringify(window.__actionbook_steps || [])"#;
    let steps_json = driver.execute_js(collect_js).await?;

    // Parse steps - the result might be double-quoted or escaped
    let steps: Vec<RecordedStep> = serde_json::from_str(&steps_json)
        .or_else(|_| {
            // Try unescaping if it's a JSON string within a string
            let unescaped: String =
                serde_json::from_str(&steps_json).unwrap_or(steps_json.clone());
            serde_json::from_str(&unescaped)
        })
        .unwrap_or_default();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            format!("{}", secs)
        })
        .unwrap_or_else(|_| "0".to_string());

    let scenario = RecordedScenario {
        url: url.to_string(),
        recorded_at: timestamp,
        steps,
    };

    let json_output = serde_json::to_string_pretty(&scenario)
        .map_err(|e| ActionbookError::Other(format!("Failed to serialize scenario: {}", e)))?;

    // Write to file or stdout
    if let Some(path) = output {
        std::fs::write(path, &json_output)
            .map_err(|e| ActionbookError::Other(format!("Failed to write file: {}", e)))?;
        if !cli.json {
            println!(
                "\n{} Saved {} steps to {}",
                "Done.".green(),
                scenario.steps.len(),
                path
            );
        } else {
            println!(
                "{}",
                serde_json::json!({
                    "success": true,
                    "file": path,
                    "steps_count": scenario.steps.len(),
                })
            );
        }
    } else {
        println!("{}", json_output);
    }

    Ok(())
}
