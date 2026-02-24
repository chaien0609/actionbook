//! Browser driver router for multi-backend support
//!
//! Routes commands to either CDP (Chrome/Edge/Brave) or Camoufox backend based on configuration.

use super::{
    session::SessionManager,
    BrowserBackend,
};

#[cfg(feature = "camoufox")]
use super::{
    camofox::CamofoxSession,
    camofox_webdriver::CamofoxDriver,
};
#[allow(unused_imports)]
use crate::{
    cli::Cli,
    config::{Config, ProfileConfig},
    error::{ActionbookError, Result},
};

/// Unified browser driver that routes commands to the appropriate backend
pub enum BrowserDriver {
    /// Chrome DevTools Protocol backend
    Cdp(SessionManager),
    #[cfg(feature = "camoufox")]
    /// Camoufox browser backend (REST API via Python server)
    Camofox(CamofoxSession),
    #[cfg(feature = "camoufox")]
    /// Camoufox WebDriver backend (direct Rust control)
    CamofoxWebDriver(CamofoxDriver),
}

impl BrowserDriver {
    /// Create a browser driver from CLI flags (convenience wrapper)
    ///
    /// Respects `--profile` and `--cdp` CLI flags to override config defaults.
    pub async fn from_cli(cli: &Cli) -> Result<Self> {
        let config = Config::load()?;
        let profile_name = match cli.profile.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(name) => name.to_string(),
            None => config.effective_default_profile_name(),
        };
        let mut profile = config.get_profile(&profile_name)?;
        // Apply CLI overrides
        if let Some(ref cdp) = cli.cdp {
            profile.cdp_url = Some(cdp.clone());
        }
        if cli.headless {
            profile.headless = true;
        }
        Self::from_config(&config, &profile, cli).await
    }

    /// Create a browser driver from configuration
    ///
    /// Backend selection hierarchy:
    /// 1. CLI flag: `--camofox`
    /// 2. Profile config: `profiles.{name}.backend`
    /// 3. Global config: `browser.backend`
    /// 4. Default: CDP
    pub async fn from_config(
        config: &Config,
        profile: &ProfileConfig,
        cli: &Cli,
    ) -> Result<Self> {
        // Determine backend
        let backend = if cli.camofox {
            BrowserBackend::Camofox
        } else {
            profile
                .backend
                .or(Some(config.browser.backend))
                .unwrap_or_default()
        };

        match backend {
            BrowserBackend::Cdp => {
                let session_mgr = SessionManager::new(config.clone());
                Ok(Self::Cdp(session_mgr))
            }
            #[cfg(feature = "camoufox")]
            BrowserBackend::Camofox => {
                // Check if using WebDriver mode
                if config.browser.camofox.use_webdriver {
                    // Use Rust WebDriver implementation
                    let headless = config.browser.camofox.headless;
                    let driver = CamofoxDriver::new(headless).await?;
                    Ok(Self::CamofoxWebDriver(driver))
                } else {
                    // Use REST API (Python server)
                    let port = cli
                        .camofox_port
                        .or(profile.camofox_port)
                        .unwrap_or(config.browser.camofox.port);

                    let user_id = config
                        .browser
                        .camofox
                        .user_id
                        .clone()
                        .unwrap_or_else(|| "actionbook-user".to_string());

                    let session_key = config
                        .browser
                        .camofox
                        .session_key
                        .clone()
                        .unwrap_or_else(|| format!("actionbook-default"));

                    let session = CamofoxSession::connect(port, user_id, session_key).await?;
                    Ok(Self::Camofox(session))
                }
            }
            #[cfg(not(feature = "camoufox"))]
            BrowserBackend::Camofox => {
                Err(crate::error::ActionbookError::FeatureNotEnabled(
                    "camoufox".to_string(),
                    "Compile with --features camoufox to use Camoufox backend".to_string(),
                ))
            }
        }
    }

    /// Navigate to a URL
    pub async fn goto(&mut self, url: &str) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.goto(None, url).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(session) => {
                // If no active tab, create one
                if session.active_tab().is_err() {
                    session.create_tab(url).await?;
                    Ok(())
                } else {
                    session.navigate(url).await
                }
            }
                                    #[cfg(feature = "camoufox")]
            Self::CamofoxWebDriver(driver) => driver.goto(url).await,
        }
    }

    /// Ensure Camoufox session has an active tab (creates blank tab if needed)
    #[cfg(feature = "camoufox")]
    async fn ensure_camofox_tab(session: &mut CamofoxSession) -> Result<()> {
        if session.active_tab().is_err() {
            // Create a blank tab at about:blank
            session.create_tab("about:blank").await?;
        }
        Ok(())
    }

    /// Click an element by selector
    pub async fn click(&mut self, selector: &str) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.click_on_page(None, selector).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(session) => {
                Self::ensure_camofox_tab(session).await?;
                session.click(selector).await
            }
                        #[cfg(feature = "camoufox")]
            Self::CamofoxWebDriver(driver) => driver.click(selector).await,
        }
    }

    /// Fill (clear + type) text into an element
    pub async fn fill(&mut self, selector: &str, text: &str) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.fill_on_page(None, selector, text).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(crate::error::ActionbookError::FeatureNotSupported(
                    "fill is not yet supported for Camoufox backend".to_string(),
                ))
            }
        }
    }

    /// Select an option from a dropdown
    pub async fn select(&mut self, selector: &str, value: &str) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.select_on_page(None, selector, value).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(crate::error::ActionbookError::FeatureNotSupported(
                    "select is not yet supported for Camoufox backend".to_string(),
                ))
            }
        }
    }

    /// Hover over an element
    pub async fn hover(&mut self, selector: &str) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.hover_on_page(None, selector).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(crate::error::ActionbookError::FeatureNotSupported(
                    "hover is not yet supported for Camoufox backend".to_string(),
                ))
            }
        }
    }

    /// Focus on an element
    pub async fn focus(&mut self, selector: &str) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.focus_on_page(None, selector).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(crate::error::ActionbookError::FeatureNotSupported(
                    "focus is not yet supported for Camoufox backend".to_string(),
                ))
            }
        }
    }

    /// Type text into an element
    pub async fn type_text(&mut self, selector: &str, text: &str) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.type_on_page(None, selector, text).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(session) => {
                Self::ensure_camofox_tab(session).await?;
                session.type_text(selector, text).await
            }
                        #[cfg(feature = "camoufox")]
            Self::CamofoxWebDriver(driver) => driver.type_text(selector, text).await,
        }
    }

    /// Take a screenshot
    pub async fn screenshot(&mut self) -> Result<Vec<u8>> {
        match self {
            Self::Cdp(mgr) => mgr.screenshot_page(None).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(session) => {
                Self::ensure_camofox_tab(session).await?;
                session.screenshot().await
            }
                        #[cfg(feature = "camoufox")]
            Self::CamofoxWebDriver(driver) => driver.screenshot().await,
        }
    }

    /// Get page content as string
    ///
    /// For CDP: Returns HTML
    /// For Camoufox REST: Returns accessibility tree JSON
    /// For Camoufox WebDriver: Returns HTML
    pub async fn get_content(&mut self) -> Result<String> {
        match self {
            Self::Cdp(mgr) => mgr.get_html(None, None).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(session) => {
                Self::ensure_camofox_tab(session).await?;
                session.get_content().await
            }
            #[cfg(feature = "camoufox")]
            Self::CamofoxWebDriver(driver) => driver.get_html().await,
        }
    }

    // ========== G5: Fingerprint Rotation ==========

    /// Rotate the browser fingerprint dynamically
    pub async fn rotate_fingerprint(
        &mut self,
        fingerprint: &super::stealth_enhanced::EnhancedStealthProfile,
    ) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.rotate_fingerprint(None, fingerprint).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Fingerprint rotation is only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    // ========== G2: Global Animation Disabling ==========

    /// Disable CSS animations and transitions on all pages
    pub async fn disable_animations(&mut self) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.disable_animations(None).await,
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Animation disabling is only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    // ========== F3: Resource Blocking ==========

    /// Block resource loading at network level
    pub async fn set_resource_blocking(
        &mut self,
        level: super::session::ResourceBlockLevel,
    ) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.set_resource_blocking(None, level).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Resource blocking is only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    // ========== F4: Readability Text Extraction ==========

    /// Get readable text content from the page
    pub async fn get_readable_text(
        &mut self,
        mode: super::session::TextExtractionMode,
    ) -> Result<String> {
        match self {
            Self::Cdp(mgr) => mgr.get_readable_text(None, mode).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                // Fallback: use eval for raw mode
                Err(ActionbookError::FeatureNotSupported(
                    "Readability extraction is only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    // ========== F1: CDP Accessibility Tree ==========

    /// Get the accessibility tree via CDP (returns raw JSON)
    pub async fn get_accessibility_tree_raw(&mut self) -> Result<serde_json::Value> {
        match self {
            Self::Cdp(mgr) => mgr.get_accessibility_tree(None).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "CDP Accessibility Tree is only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    /// Get backend node ID for a CSS selector (for snapshot scoping)
    pub async fn get_backend_node_id(&mut self, selector: &str) -> Result<Option<i64>> {
        match self {
            Self::Cdp(mgr) => mgr.get_backend_node_id(None, selector).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => Ok(None),
        }
    }

    // ========== F2: Ref-based actions ==========

    /// Click an element by backendNodeId
    pub async fn click_by_node_id(&mut self, backend_node_id: i64) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.click_by_node_id(None, backend_node_id).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Node ID actions are only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    /// Type text into an element by backendNodeId
    pub async fn type_by_node_id(&mut self, backend_node_id: i64, text: &str) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.type_by_node_id(None, backend_node_id, text).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Node ID actions are only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    /// Fill an element by backendNodeId
    pub async fn fill_by_node_id(&mut self, backend_node_id: i64, text: &str) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.fill_by_node_id(None, backend_node_id, text).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Node ID actions are only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    /// Focus an element by backendNodeId
    pub async fn focus_by_node_id(&mut self, backend_node_id: i64) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.focus_by_node_id(None, backend_node_id).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Node ID actions are only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    /// Get the center coordinates of an element by backendNodeId
    pub async fn get_element_center_by_node_id(&mut self, backend_node_id: i64) -> Result<(f64, f64)> {
        match self {
            Self::Cdp(mgr) => mgr.get_element_center_by_node_id(None, backend_node_id).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Node ID actions are only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    /// Get the center coordinates of an element by CSS selector
    pub async fn get_element_center(&mut self, selector: &str) -> Result<(f64, f64)> {
        match self {
            Self::Cdp(mgr) => mgr.get_element_center(None, selector).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Element center lookup is only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    // ========== F5: Human-like input ==========

    /// Dispatch mouse move events along a path
    pub async fn dispatch_mouse_moves(&mut self, points: &[(f64, f64)]) -> Result<()> {
        match self {
            Self::Cdp(mgr) => mgr.dispatch_mouse_moves(None, points).await,
                        #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(ActionbookError::FeatureNotSupported(
                    "Human mouse simulation is only supported on CDP backend".to_string(),
                ))
            }
        }
    }

    /// Execute JavaScript and return result as string (convenience alias for execute_js)
    pub async fn eval(&mut self, script: &str) -> Result<String> {
        self.execute_js(script).await
    }

    /// Execute JavaScript (CDP only)
    ///
    /// For Camoufox, returns an error as it doesn't support arbitrary JS execution
    #[allow(dead_code)]
    pub async fn execute_js(&mut self, script: &str) -> Result<String> {
        match self {
            Self::Cdp(mgr) => {
                let result = mgr.eval_on_page(None, script).await?;
                Ok(serde_json::to_string(&result).unwrap_or_default())
            }
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => {
                Err(crate::error::ActionbookError::BrowserOperation(
                    "JavaScript execution not supported in Camoufox backend".to_string(),
                ))
            }
        }
    }

    /// Get the backend type
    pub fn backend(&self) -> BrowserBackend {
        match self {
            Self::Cdp(_) => BrowserBackend::Cdp,
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => BrowserBackend::Camofox,
        }
    }

    /// Check if the driver is using Camoufox (either REST or WebDriver)
    pub fn is_camofox(&self) -> bool {
        #[cfg(feature = "camoufox")]
        {
            matches!(self, Self::Camofox(_) | Self::CamofoxWebDriver(_))
        }
        #[cfg(not(feature = "camoufox"))]
        {
            false
        }
    }

    /// Check if the driver is using CDP
    pub fn is_cdp(&self) -> bool {
        matches!(self, Self::Cdp(_))
    }

    /// Check if using WebDriver mode (direct Rust control)
    #[allow(dead_code)]
    pub fn is_webdriver(&self) -> bool {
        #[cfg(feature = "camoufox")]
        {
            matches!(self, Self::CamofoxWebDriver(_))
        }
        #[cfg(not(feature = "camoufox"))]
        {
            false
        }
    }

    /// Get CDP session manager (if using CDP backend)
    #[allow(dead_code)]
    pub fn as_cdp(&self) -> Option<&SessionManager> {
        match self {
            Self::Cdp(mgr) => Some(mgr),
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => None,
        }
    }

    /// Get CDP session manager mutably (if using CDP backend)
    pub fn as_cdp_mut(&mut self) -> Option<&mut SessionManager> {
        match self {
            Self::Cdp(mgr) => Some(mgr),
            #[cfg(feature = "camoufox")]
            Self::Camofox(_) | Self::CamofoxWebDriver(_) => None,
        }
    }

    /// Get Camoufox session (if using Camoufox REST backend)
    #[cfg(feature = "camoufox")]
    #[allow(dead_code)]
    pub fn as_camofox(&self) -> Option<&CamofoxSession> {
        match self {
            Self::Cdp(_) | Self::CamofoxWebDriver(_) => None,
            Self::Camofox(session) => Some(session),
        }
    }

    /// Get Camoufox session mutably (if using Camoufox REST backend)
    #[cfg(feature = "camoufox")]
    #[allow(dead_code)]
    pub fn as_camofox_mut(&mut self) -> Option<&mut CamofoxSession> {
        match self {
            Self::Cdp(_) | Self::CamofoxWebDriver(_) => None,
            Self::Camofox(session) => Some(session),
        }
    }

    /// Get Camoufox WebDriver (if using WebDriver backend)
    #[cfg(feature = "camoufox")]
    #[allow(dead_code)]
    pub fn as_webdriver(&self) -> Option<&CamofoxDriver> {
        match self {
            Self::CamofoxWebDriver(driver) => Some(driver),
            Self::Cdp(_) | Self::Camofox(_) => None,
        }
    }

    /// Get Camoufox WebDriver mutably (if using WebDriver backend)
    #[cfg(feature = "camoufox")]
    #[allow(dead_code)]
    pub fn as_webdriver_mut(&mut self) -> Option<&mut CamofoxDriver> {
        match self {
            Self::CamofoxWebDriver(driver) => Some(driver),
            Self::Cdp(_) | Self::Camofox(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_type_checking() {
        let config = Config::default();
        let session_mgr = SessionManager::new(config);
        let driver = BrowserDriver::Cdp(session_mgr);

        assert!(driver.is_cdp());
        assert!(!driver.is_camofox());
        assert_eq!(driver.backend(), BrowserBackend::Cdp);
        assert!(driver.as_cdp().is_some());
        #[cfg(feature = "camoufox")]
        assert!(driver.as_camofox().is_none());
    }
}
