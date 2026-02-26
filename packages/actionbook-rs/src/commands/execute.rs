use colored::Colorize;

use crate::api::ApiClient;
use crate::browser::BrowserDriver;
use crate::cli::Cli;
use crate::config::Config;
use crate::error::{ActionbookError, Result};

/// Try a CSS selector, return Ok(true) if element exists, Ok(false) if not, Err on eval failure
async fn try_css_selector(driver: &mut BrowserDriver, css: &str) -> Result<bool> {
    let js = format!(
        "document.querySelector({}) !== null",
        serde_json::to_string(css).unwrap_or_else(|_| format!("\"{}\"", css))
    );
    let result = driver.eval(&js).await?;
    Ok(result.contains("true"))
}

/// Try an XPath selector, return Ok(true) if element exists, Ok(false) if not, Err on eval failure
async fn try_xpath_selector(driver: &mut BrowserDriver, xpath: &str) -> Result<bool> {
    let js = format!(
        "document.evaluate({}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue !== null",
        serde_json::to_string(xpath).unwrap_or_else(|_| format!("\"{}\"", xpath))
    );
    let result = driver.eval(&js).await?;
    Ok(result.contains("true"))
}

/// Try to find element via accessibility snapshot by name and role
async fn try_snapshot_match(
    driver: &mut BrowserDriver,
    description: Option<&str>,
    element_type: Option<&str>,
) -> Option<String> {
    // Get accessibility snapshot
    let js = r#"
    (() => {
        const refs = [];
        const walk = (node) => {
            if (node.nodeType === 1) {
                const role = node.getAttribute('role') || node.tagName.toLowerCase();
                const name = node.getAttribute('aria-label')
                    || node.getAttribute('title')
                    || node.getAttribute('placeholder')
                    || node.textContent?.trim().substring(0, 50)
                    || '';
                if (name) {
                    refs.push({ role, name, tag: node.tagName.toLowerCase() });
                }
            }
            for (const child of node.childNodes) walk(child);
        };
        walk(document.body);
        return JSON.stringify(refs.slice(0, 200));
    })()
    "#;

    let result = driver.eval(js).await.ok()?;
    let items: Vec<serde_json::Value> = serde_json::from_str(&result)
        .or_else(|_| {
            let unescaped: String = serde_json::from_str(&result).unwrap_or(result.clone());
            serde_json::from_str(&unescaped)
        })
        .ok()?;

    let desc_lower = description.map(|d| d.to_lowercase());
    let type_lower = element_type.map(|t| t.to_lowercase());

    for item in &items {
        let name = item.get("name")?.as_str()?.to_lowercase();
        let role = item.get("role")?.as_str()?.to_lowercase();

        let name_match = desc_lower
            .as_ref()
            .map(|d| name.contains(d.as_str()) || d.contains(name.as_str()))
            .unwrap_or(false);
        let type_match = type_lower
            .as_ref()
            .map(|t| role.contains(t.as_str()) || t.contains(role.as_str()))
            .unwrap_or(true);

        if name_match && type_match {
            // Build a selector by aria-label if possible
            let orig_name = item.get("name")?.as_str()?;
            let tag = item.get("tag")?.as_str()?;
            return Some(format!(
                "{}[aria-label=\"{}\"]",
                tag,
                orig_name.replace('"', "\\\"")
            ));
        }
    }

    None
}

/// Execute an action with selector fallback strategy:
/// 1. CSS selector
/// 2. XPath selector
/// 3. Accessibility snapshot match (CDP only)
///
/// On non-CDP backends (e.g. Camoufox) where JS eval is not available,
/// skips element testing and executes directly with the first available selector.
async fn execute_with_fallback(
    driver: &mut BrowserDriver,
    method: &str,
    css: Option<&str>,
    xpath: Option<&str>,
    description: Option<&str>,
    element_type: Option<&str>,
    text: Option<&str>,
    value: Option<&str>,
    element_id: &str,
    area_id: &str,
) -> Result<(&'static str, String)> {
    let js_supported = driver.is_cdp();

    if js_supported {
        // CDP path: test selectors before executing
        // Strategy 1: CSS selector
        if let Some(css) = css {
            if try_css_selector(driver, css).await? {
                execute_action(driver, css, method, text, value, element_id).await?;
                return Ok(("css", css.to_string()));
            }
        }

        // Strategy 2: XPath selector
        if let Some(xpath) = xpath {
            if try_xpath_selector(driver, xpath).await? {
                execute_action(driver, xpath, method, text, value, element_id).await?;
                return Ok(("xpath", xpath.to_string()));
            }
        }

        // Strategy 3: Accessibility snapshot match
        if let Some(snapshot_selector) =
            try_snapshot_match(driver, description, element_type).await
        {
            if try_css_selector(driver, &snapshot_selector).await? {
                execute_action(driver, &snapshot_selector, method, text, value, element_id)
                    .await?;
                return Ok(("snapshot", snapshot_selector));
            }
        }
    } else {
        // Non-CDP path: execute directly with first available selector
        if let Some(css) = css {
            execute_action(driver, css, method, text, value, element_id).await?;
            return Ok(("css", css.to_string()));
        }
        if let Some(xpath) = xpath {
            execute_action(driver, xpath, method, text, value, element_id).await?;
            return Ok(("xpath", xpath.to_string()));
        }
    }

    // All strategies failed
    let tried = [
        css.map(|s| format!("CSS: {}", s)),
        xpath.map(|s| format!("XPath: {}", s)),
        if js_supported {
            Some("Snapshot: no match".to_string())
        } else {
            Some("JS eval not available on this backend".to_string())
        },
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");

    Err(ActionbookError::ElementNotFound(format!(
        "Element '{}' not found after trying all selectors ({}). Try: actionbook validate \"{}\"",
        element_id, tried, area_id
    )))
}

/// Execute the actual browser action on a resolved selector
async fn execute_action(
    driver: &mut BrowserDriver,
    selector: &str,
    method: &str,
    text: Option<&str>,
    value: Option<&str>,
    element_id: &str,
) -> Result<()> {
    match method {
        "click" => driver.click(selector).await,
        "fill" => {
            let txt = text.ok_or_else(|| {
                ActionbookError::ElementActionFailed(
                    element_id.to_string(),
                    "fill".to_string(),
                    "--text is required for fill method".to_string(),
                )
            })?;
            driver.fill(selector, txt).await
        }
        "type" => {
            let txt = text.ok_or_else(|| {
                ActionbookError::ElementActionFailed(
                    element_id.to_string(),
                    "type".to_string(),
                    "--text is required for type method".to_string(),
                )
            })?;
            driver.type_text(selector, txt).await
        }
        "select" => {
            let val = value.ok_or_else(|| {
                ActionbookError::ElementActionFailed(
                    element_id.to_string(),
                    "select".to_string(),
                    "--value is required for select method".to_string(),
                )
            })?;
            driver.select(selector, val).await
        }
        "hover" => driver.hover(selector).await,
        "focus" => driver.focus(selector).await,
        other => Err(ActionbookError::ElementActionFailed(
            element_id.to_string(),
            other.to_string(),
            format!(
                "Unknown method '{}'. Supported: click, fill, type, select, hover, focus",
                other
            ),
        )),
    }
}

pub async fn run(
    cli: &Cli,
    area_id: &str,
    element_id: &str,
    method: &str,
    text: Option<&str>,
    value: Option<&str>,
    navigate: bool,
) -> Result<()> {
    // 1. Fetch structured area data from API
    let mut config = Config::load()?;
    if let Some(ref key) = cli.api_key {
        config.api.api_key = Some(key.clone());
    }
    let client = ApiClient::from_config(&config)?;

    let detail = client.get_action_by_area_id_json(area_id).await?;

    // 2. Find the element
    let element = detail.elements.get(element_id).ok_or_else(|| {
        ActionbookError::ElementNotFound(format!(
            "Element '{}' not found in area '{}'. Available: {}",
            element_id,
            area_id,
            detail
                .elements
                .keys()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    // 3. Validate method is allowed
    let method_lower = method.to_lowercase();
    if !element.allow_methods.is_empty()
        && !element
            .allow_methods
            .iter()
            .any(|m| m.to_lowercase() == method_lower)
    {
        return Err(ActionbookError::ElementActionFailed(
            element_id.to_string(),
            method.to_string(),
            format!(
                "Method '{}' not allowed. Allowed methods: {}",
                method,
                element.allow_methods.join(", ")
            ),
        ));
    }

    // 4. Create browser driver
    let mut driver = BrowserDriver::from_cli(cli).await?;

    // 5. Optionally navigate first
    if navigate {
        if let Some(ref url) = detail.url {
            driver.goto(url).await?;
        }
    }

    // 6. Execute with selector fallback strategy
    let (selector_used, actual_selector) = execute_with_fallback(
        &mut driver,
        &method_lower,
        element.css_selector.as_deref(),
        element.xpath_selector.as_deref(),
        element.description.as_deref(),
        element.element_type.as_deref(),
        text,
        value,
        element_id,
        area_id,
    )
    .await?;

    // 7. Output result
    if cli.json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "area_id": area_id,
                "element_id": element_id,
                "method": method_lower,
                "selector": actual_selector,
                "selector_used": selector_used,
            })
        );
    } else {
        let fallback_note = match selector_used {
            "css" => String::new(),
            "xpath" => " (used xpath fallback)".yellow().to_string(),
            "snapshot" => " (used snapshot fallback)".yellow().to_string(),
            _ => String::new(),
        };
        println!(
            "{} {} {} on {}{}",
            "✓".green(),
            method_lower.green().bold(),
            element_id.yellow(),
            actual_selector.dimmed(),
            fallback_note
        );
    }

    Ok(())
}
