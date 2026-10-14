//! What `browser_open` asks the site list. A trait so `browser/` and `mcp/tools/browser.rs` do not
//! depend on the database module, and tests can plug a fake.
pub trait SiteGate: Send + Sync {
    /// `Ok(site key)` when the agent may open `url`; `Err(message)` is returned to the agent.
    fn check_open(&self, url: &str) -> Result<String, String>;
    /// A read of the page at `url` ended in `estado: login_wall`.
    fn login_wall(&self, url: &str);
}
